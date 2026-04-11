use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

const GET_TRACE_SQL: &str = r#"
SELECT
  session_id,
  agent,
  event_type,
  timestamp,
  status,
  error_message,
  latency_ms,
  content
FROM `{project}.{dataset}.{table}`
WHERE session_id = '{session_id}'
ORDER BY timestamp ASC
"#;

#[derive(Serialize)]
pub struct TraceResult {
    pub session_id: String,
    pub agent: String,
    pub event_count: u64,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub has_errors: bool,
    pub events: Vec<TraceEvent>,
}

#[derive(Serialize)]
pub struct TraceEvent {
    pub event_type: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

// ── SQL builder ──

pub fn build_trace_query(project: &str, dataset: &str, table: &str, session_id: &str) -> String {
    GET_TRACE_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{session_id}", session_id)
}

// ── Result mapper ──

pub fn trace_result_from_rows(session_id: String, result: &QueryResult) -> Result<TraceResult> {
    if result.rows.is_empty() {
        anyhow::bail!("No events found for session_id: {session_id}");
    }

    let agent = result
        .rows
        .first()
        .and_then(|r| r.get("agent"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let events: Vec<TraceEvent> = result
        .rows
        .iter()
        .map(|row| {
            let get_str = |key: &str| -> Option<String> {
                row.get(key)
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            };

            let get_json = |key: &str| -> Option<serde_json::Value> {
                row.get(key).and_then(|v| {
                    if v.is_null() {
                        None
                    } else if let Some(s) = v.as_str() {
                        serde_json::from_str(s).ok().or_else(|| Some(v.clone()))
                    } else {
                        Some(v.clone())
                    }
                })
            };

            TraceEvent {
                event_type: get_str("event_type").unwrap_or_default(),
                timestamp: get_str("timestamp").unwrap_or_default(),
                status: get_str("status"),
                error_message: get_str("error_message"),
                latency_ms: get_json("latency_ms"),
                content: get_json("content"),
            }
        })
        .collect();

    let has_errors = events.iter().any(|e| {
        e.event_type.ends_with("_ERROR")
            || e.error_message.is_some()
            || e.status.as_deref() == Some("ERROR")
    });

    let started_at = events.first().map(|e| e.timestamp.clone());
    let ended_at = events.last().map(|e| e.timestamp.clone());

    Ok(TraceResult {
        session_id,
        agent,
        event_count: events.len() as u64,
        started_at,
        ended_at,
        has_errors,
        events,
    })
}

// ── Data builder (shared by run + run_with_executor) ──

async fn build_trace(
    executor: &dyn QueryExecutor,
    session_id: &str,
    config: &Config,
) -> Result<TraceResult> {
    config::validate_session_id(session_id)?;
    let dataset_id = config.require_dataset_id()?;

    let sql = build_trace_query(&config.project_id, dataset_id, &config.table, session_id);

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

    trace_result_from_rows(session_id.to_string(), &result)
}

fn render_trace_output(trace: &TraceResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_trace(trace);
        }
        OutputFormat::Table => {
            println!(
                "Session: {}  Agent: {}  Events: {}  Errors: {}",
                trace.session_id, trace.agent, trace.event_count, trace.has_errors
            );
            if let (Some(ref start), Some(ref end)) = (&trace.started_at, &trace.ended_at) {
                println!("Time:    {} → {}", start, end);
            }
            println!();

            let columns = vec![
                "timestamp".into(),
                "event_type".into(),
                "status".into(),
                "latency_ms".into(),
                "error_message".into(),
            ];
            let rows: Vec<Vec<String>> = trace
                .events
                .iter()
                .map(|e| {
                    let latency = e.latency_ms.as_ref().map_or("-".into(), |v| {
                        if let Some(obj) = v.as_object() {
                            obj.get("total_ms").map_or("-".into(), |ms| ms.to_string())
                        } else {
                            v.to_string()
                        }
                    });
                    vec![
                        e.timestamp.clone(),
                        e.event_type.clone(),
                        e.status.clone().unwrap_or("-".into()),
                        latency,
                        e.error_message.clone().unwrap_or("-".into()),
                    ]
                })
                .collect();
            output::render_rows_as_table(&columns, &rows)?;
        }
        OutputFormat::Json | OutputFormat::JsonMinified => {
            output::render(trace, &config.format)?;
        }
    }
    Ok(())
}

// ── Entry points ──

/// Resolve session_id from either --session-id or --trace-id.
///
/// Note: --trace-id is currently treated as an alias for --session-id.
/// The upstream SDK distinguishes get_session_trace(session_id) from
/// get_trace(trace_id); dcx does not yet implement a separate trace-id
/// lookup path.
fn resolve_id(session_id: Option<String>, trace_id: Option<String>) -> Result<String> {
    match (session_id, trace_id) {
        (Some(sid), _) => Ok(sid),
        (None, Some(tid)) => {
            eprintln!(
                "Warning: --trace-id is currently treated as an alias for --session-id. \
                 A dedicated trace-id lookup is planned for a future release."
            );
            Ok(tid)
        }
        (None, None) => anyhow::bail!("Provide --session-id or --trace-id."),
    }
}

pub async fn run(
    session_id: Option<String>,
    trace_id: Option<String>,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    let session_id = resolve_id(session_id, trace_id)?;
    config::validate_session_id(&session_id)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let trace = build_trace(&client, &session_id, config).await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&trace)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    render_trace_output(&trace, config)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    session_id: String,
    config: &Config,
) -> Result<()> {
    let trace = build_trace(executor, &session_id, config).await?;
    render_trace_output(&trace, config)
}
