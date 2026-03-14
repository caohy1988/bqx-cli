use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

const HITL_METRICS_SQL: &str = r#"
WITH hitl_events AS (
  SELECT
    session_id,
    agent,
    event_type,
    timestamp
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
    AND event_type IN ('HUMAN_INPUT_REQUIRED', 'HUMAN_INPUT_RECEIVED')
),
session_counts AS (
  SELECT DISTINCT session_id
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
)
SELECT
  (SELECT COUNT(*) FROM session_counts) AS total_sessions,
  COUNTIF(event_type = 'HUMAN_INPUT_REQUIRED') AS hitl_required_count,
  COUNTIF(event_type = 'HUMAN_INPUT_RECEIVED') AS hitl_received_count,
  COUNT(DISTINCT session_id) AS sessions_with_hitl,
  SAFE_DIVIDE(
    COUNT(DISTINCT session_id),
    (SELECT COUNT(*) FROM session_counts)
  ) AS hitl_session_rate
FROM hitl_events
"#;

const HITL_PER_SESSION_SQL: &str = r#"
SELECT
  session_id,
  agent,
  COUNTIF(event_type = 'HUMAN_INPUT_REQUIRED') AS required_count,
  COUNTIF(event_type = 'HUMAN_INPUT_RECEIVED') AS received_count,
  MIN(timestamp) AS first_hitl_at,
  MAX(timestamp) AS last_hitl_at
FROM `{project}.{dataset}.{table}`
WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
  {agent_filter}
  AND event_type IN ('HUMAN_INPUT_REQUIRED', 'HUMAN_INPUT_RECEIVED')
GROUP BY session_id, agent
ORDER BY required_count DESC
LIMIT {limit}
"#;

#[derive(Serialize)]
pub struct HitlMetricsResult {
    pub time_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub summary: HitlSummary,
    pub sessions: Vec<HitlSession>,
}

#[derive(Serialize)]
pub struct HitlSummary {
    pub total_sessions: u64,
    pub hitl_required_count: u64,
    pub hitl_received_count: u64,
    pub sessions_with_hitl: u64,
    pub hitl_session_rate: f64,
}

#[derive(Serialize)]
pub struct HitlSession {
    pub session_id: String,
    pub agent: String,
    pub required_count: u64,
    pub received_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_hitl_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_hitl_at: Option<String>,
}

// ── SQL builders ──

fn replace_common(
    sql: &str,
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
) -> String {
    let agent_filter = match agent_id {
        Some(id) => format!("AND agent = '{id}'"),
        None => String::new(),
    };
    sql.replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{interval}", interval_sql)
        .replace("{agent_filter}", &agent_filter)
}

pub fn build_hitl_summary_query(
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
) -> String {
    replace_common(
        HITL_METRICS_SQL,
        project,
        dataset,
        table,
        interval_sql,
        agent_id,
    )
}

pub fn build_hitl_sessions_query(
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
    limit: u32,
) -> String {
    replace_common(
        HITL_PER_SESSION_SQL,
        project,
        dataset,
        table,
        interval_sql,
        agent_id,
    )
    .replace("{limit}", &limit.to_string())
}

// ── Result mappers ──

pub fn hitl_summary_from_rows(result: &QueryResult) -> HitlSummary {
    let row = match result.rows.first() {
        Some(r) => r,
        None => {
            return HitlSummary {
                total_sessions: 0,
                hitl_required_count: 0,
                hitl_received_count: 0,
                sessions_with_hitl: 0,
                hitl_session_rate: 0.0,
            }
        }
    };

    let get_u64 = |key: &str| -> u64 {
        row.get(key)
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0)
    };
    let get_f64 = |key: &str| -> f64 {
        row.get(key)
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0.0)
    };

    HitlSummary {
        total_sessions: get_u64("total_sessions"),
        hitl_required_count: get_u64("hitl_required_count"),
        hitl_received_count: get_u64("hitl_received_count"),
        sessions_with_hitl: get_u64("sessions_with_hitl"),
        hitl_session_rate: get_f64("hitl_session_rate"),
    }
}

pub fn hitl_sessions_from_rows(result: &QueryResult) -> Vec<HitlSession> {
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

            HitlSession {
                session_id: get_str("session_id").unwrap_or_default(),
                agent: get_str("agent").unwrap_or_else(|| "unknown".to_string()),
                required_count: get_u64("required_count"),
                received_count: get_u64("received_count"),
                first_hitl_at: get_str("first_hitl_at"),
                last_hitl_at: get_str("last_hitl_at"),
            }
        })
        .collect()
}

// ── Data builder ──

async fn build_hitl_metrics(
    executor: &dyn QueryExecutor,
    last: &str,
    agent_id: Option<&str>,
    limit: u32,
    config: &Config,
) -> Result<HitlMetricsResult> {
    if let Some(id) = agent_id {
        config::validate_agent_id(id)?;
    }
    let parsed = config::parse_duration(last)?;
    let dataset_id = config.require_dataset_id()?;

    let summary_sql = build_hitl_summary_query(
        &config.project_id,
        dataset_id,
        &config.table,
        &parsed.interval_sql,
        agent_id,
    );
    let sessions_sql = build_hitl_sessions_query(
        &config.project_id,
        dataset_id,
        &config.table,
        &parsed.interval_sql,
        agent_id,
        limit,
    );

    let make_req = |sql: String| QueryRequest {
        query: sql,
        use_legacy_sql: false,
        location: config.location.clone(),
        max_results: None,
        timeout_ms: Some(30000),
    };

    let summary_result = executor
        .query(&config.project_id, make_req(summary_sql))
        .await?;
    let sessions_result = executor
        .query(&config.project_id, make_req(sessions_sql))
        .await?;

    Ok(HitlMetricsResult {
        time_window: last.to_string(),
        agent_id: agent_id.map(|s| s.to_string()),
        summary: hitl_summary_from_rows(&summary_result),
        sessions: hitl_sessions_from_rows(&sessions_result),
    })
}

fn render_hitl_metrics(result: &HitlMetricsResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_hitl_metrics(result);
        }
        OutputFormat::Table => {
            let s = &result.summary;
            println!(
                "HITL Metrics: Window={}  Sessions with HITL={}/{}",
                result.time_window, s.sessions_with_hitl, s.total_sessions
            );
            if let Some(ref agent) = result.agent_id {
                println!("Agent: {agent}");
            }
            println!();
            let columns = vec![
                "session_id".into(),
                "agent".into(),
                "required".into(),
                "received".into(),
                "first_at".into(),
            ];
            let rows: Vec<Vec<String>> = result
                .sessions
                .iter()
                .map(|s| {
                    vec![
                        s.session_id.clone(),
                        s.agent.clone(),
                        s.required_count.to_string(),
                        s.received_count.to_string(),
                        s.first_hitl_at.clone().unwrap_or("-".into()),
                    ]
                })
                .collect();
            output::render_rows_as_table(&columns, &rows)?;
        }
        OutputFormat::Json => {
            output::render(result, &config.format)?;
        }
    }
    Ok(())
}

// ── Entry points ──

pub async fn run(
    last: String,
    agent_id: Option<String>,
    limit: u32,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if let Some(ref id) = agent_id {
        config::validate_agent_id(id)?;
    }
    config::parse_duration(&last)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_hitl_metrics(&client, &last, agent_id.as_deref(), limit, config).await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    render_hitl_metrics(&result, config)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    last: String,
    agent_id: Option<String>,
    limit: u32,
    config: &Config,
) -> Result<()> {
    let result = build_hitl_metrics(executor, &last, agent_id.as_deref(), limit, config).await?;
    render_hitl_metrics(&result, config)
}
