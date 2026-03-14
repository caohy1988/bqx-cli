use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

const DISTRIBUTION_SQL: &str = r#"
SELECT
  event_type,
  COUNT(*) AS event_count,
  COUNT(DISTINCT session_id) AS session_count,
  SAFE_DIVIDE(COUNT(*), SUM(COUNT(*)) OVER()) AS proportion
FROM `{project}.{dataset}.{table}`
WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
  {agent_filter}
GROUP BY event_type
ORDER BY event_count DESC
"#;

#[derive(Serialize)]
pub struct DistributionResult {
    pub time_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub total_events: u64,
    pub event_types: Vec<EventDistribution>,
}

#[derive(Serialize)]
pub struct EventDistribution {
    pub event_type: String,
    pub event_count: u64,
    pub session_count: u64,
    pub proportion: f64,
}

// ── SQL builder ──

pub fn build_distribution_query(
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
    DISTRIBUTION_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{interval}", interval_sql)
        .replace("{agent_filter}", &agent_filter)
}

// ── Result mapper ──

pub fn distribution_from_rows(result: &QueryResult) -> Vec<EventDistribution> {
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
            let get_f64 = |key: &str| -> f64 {
                row.get(key)
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0.0)
            };

            EventDistribution {
                event_type: get_str("event_type").unwrap_or_default(),
                event_count: get_u64("event_count"),
                session_count: get_u64("session_count"),
                proportion: get_f64("proportion"),
            }
        })
        .collect()
}

// ── Data builder ──

async fn build_distribution(
    executor: &dyn QueryExecutor,
    last: &str,
    agent_id: Option<&str>,
    config: &Config,
) -> Result<DistributionResult> {
    if let Some(id) = agent_id {
        config::validate_agent_id(id)?;
    }
    let parsed = config::parse_duration(last)?;
    let dataset_id = config.require_dataset_id()?;

    let sql = build_distribution_query(
        &config.project_id,
        dataset_id,
        &config.table,
        &parsed.interval_sql,
        agent_id,
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

    let event_types = distribution_from_rows(&result);
    let total_events = event_types.iter().map(|e| e.event_count).sum();

    Ok(DistributionResult {
        time_window: last.to_string(),
        agent_id: agent_id.map(|s| s.to_string()),
        total_events,
        event_types,
    })
}

fn render_distribution(result: &DistributionResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_distribution(result);
        }
        OutputFormat::Table => {
            println!(
                "Distribution: Window={}  Total events={}",
                result.time_window, result.total_events
            );
            if let Some(ref agent) = result.agent_id {
                println!("Agent: {agent}");
            }
            println!();
            let columns = vec![
                "event_type".into(),
                "count".into(),
                "sessions".into(),
                "proportion".into(),
            ];
            let rows: Vec<Vec<String>> = result
                .event_types
                .iter()
                .map(|e| {
                    vec![
                        e.event_type.clone(),
                        e.event_count.to_string(),
                        e.session_count.to_string(),
                        format!("{:.2}%", e.proportion * 100.0),
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
    let result = build_distribution(&client, &last, agent_id.as_deref(), config).await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    render_distribution(&result, config)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    last: String,
    agent_id: Option<String>,
    config: &Config,
) -> Result<()> {
    let result = build_distribution(executor, &last, agent_id.as_deref(), config).await?;
    render_distribution(&result, config)
}
