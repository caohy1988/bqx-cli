use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

const LIST_TRACES_SQL: &str = r#"
SELECT
  session_id,
  agent,
  COUNT(*) AS event_count,
  MIN(timestamp) AS started_at,
  MAX(timestamp) AS ended_at,
  COUNTIF(
    ENDS_WITH(event_type, '_ERROR')
    OR error_message IS NOT NULL
    OR status = 'ERROR'
  ) > 0 AS has_errors
FROM `{project}.{dataset}.{table}`
WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
{agent_filter}
{session_filter}
GROUP BY session_id, agent
ORDER BY started_at DESC
LIMIT {limit}
"#;

#[derive(Serialize)]
pub struct ListTracesResult {
    pub traces: Vec<TraceSummary>,
    pub total: usize,
    pub time_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

#[derive(Serialize)]
pub struct TraceSummary {
    pub session_id: String,
    pub agent: String,
    pub event_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    pub has_errors: bool,
}

// ── SQL builder ──

pub fn build_list_traces_query(
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
    session_id: Option<&str>,
    limit: u32,
) -> String {
    let agent_filter = match agent_id {
        Some(id) => format!("AND agent = '{id}'"),
        None => String::new(),
    };
    let session_filter = match session_id {
        Some(id) => format!("AND session_id = '{id}'"),
        None => String::new(),
    };
    LIST_TRACES_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{interval}", interval_sql)
        .replace("{agent_filter}", &agent_filter)
        .replace("{session_filter}", &session_filter)
        .replace("{limit}", &limit.to_string())
}

// ── Result mapper ──

pub fn traces_from_rows(result: &QueryResult) -> Vec<TraceSummary> {
    result
        .rows
        .iter()
        .map(|row| {
            let get_str = |key: &str| -> Option<String> {
                row.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
            };
            let get_u64 = |key: &str| -> u64 {
                row.get(key)
                    .and_then(|v| {
                        v.as_u64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0)
            };
            let has_errors = row
                .get("has_errors")
                .map(|v| v.as_bool().unwrap_or(false) || v.as_str() == Some("true"))
                .unwrap_or(false);

            TraceSummary {
                session_id: get_str("session_id").unwrap_or_default(),
                agent: get_str("agent").unwrap_or_else(|| "unknown".to_string()),
                event_count: get_u64("event_count"),
                started_at: get_str("started_at"),
                ended_at: get_str("ended_at"),
                has_errors,
            }
        })
        .collect()
}

// ── Data builder ──

async fn build_list_traces(
    executor: &dyn QueryExecutor,
    last: &str,
    agent_id: Option<&str>,
    session_id: Option<&str>,
    limit: u32,
    config: &Config,
) -> Result<ListTracesResult> {
    if let Some(id) = agent_id {
        config::validate_agent_id(id)?;
    }
    if let Some(id) = session_id {
        config::validate_session_id(id)?;
    }
    let parsed = config::parse_duration(last)?;
    let dataset_id = config.require_dataset_id()?;

    let sql = build_list_traces_query(
        &config.project_id,
        dataset_id,
        &config.table,
        &parsed.interval_sql,
        agent_id,
        session_id,
        limit,
    );

    let result = executor
        .query(
            &config.project_id,
            QueryRequest {
                query: sql,
                use_legacy_sql: false,
                location: config.location.clone(),
                max_results: None,
                timeout_ms: Some(30000),
            },
        )
        .await?;

    let traces = traces_from_rows(&result);
    let total = traces.len();
    Ok(ListTracesResult {
        traces,
        total,
        time_window: last.to_string(),
        agent_id: agent_id.map(|s| s.to_string()),
    })
}

fn render_list_traces(result: &ListTracesResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_list_traces(result);
        }
        OutputFormat::Table => {
            let columns = vec![
                "session_id".into(),
                "agent".into(),
                "events".into(),
                "started_at".into(),
                "ended_at".into(),
                "errors".into(),
            ];
            let rows: Vec<Vec<String>> = result
                .traces
                .iter()
                .map(|t| {
                    vec![
                        t.session_id.clone(),
                        t.agent.clone(),
                        t.event_count.to_string(),
                        t.started_at.clone().unwrap_or("-".into()),
                        t.ended_at.clone().unwrap_or("-".into()),
                        t.has_errors.to_string(),
                    ]
                })
                .collect();
            println!("Traces: {}  Window: {}", result.total, result.time_window);
            if let Some(ref agent) = result.agent_id {
                println!("Agent: {agent}");
            }
            println!();
            output::render_rows_as_table(&columns, &rows)?;
        }
        OutputFormat::Json | OutputFormat::JsonMinified => {
            output::render(result, &config.format)?;
        }
    }
    Ok(())
}

// ── Entry points ──

pub async fn run(
    last: String,
    session_id: Option<String>,
    agent_id: Option<String>,
    limit: u32,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if let Some(ref id) = agent_id {
        config::validate_agent_id(id)?;
    }
    if let Some(ref id) = session_id {
        config::validate_session_id(id)?;
    }
    config::parse_duration(&last)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_list_traces(
        &client,
        &last,
        agent_id.as_deref(),
        session_id.as_deref(),
        limit,
        config,
    )
    .await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    render_list_traces(&result, config)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    last: String,
    session_id: Option<String>,
    agent_id: Option<String>,
    limit: u32,
    config: &Config,
) -> Result<()> {
    let result = build_list_traces(
        executor,
        &last,
        agent_id.as_deref(),
        session_id.as_deref(),
        limit,
        config,
    )
    .await?;
    render_list_traces(&result, config)
}
