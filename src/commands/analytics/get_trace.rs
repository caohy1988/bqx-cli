use anyhow::Result;
use serde::Serialize;

use crate::bigquery::client::{BigQueryClient, QueryRequest};
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

pub async fn run(session_id: String, config: &Config) -> Result<()> {
    config::validate_session_id(&session_id)?;

    let sql = GET_TRACE_SQL
        .replace("{project}", &config.project_id)
        .replace("{dataset}", &config.dataset_id)
        .replace("{table}", &config.table)
        .replace("{session_id}", &session_id);

    let client = BigQueryClient::new().await?;
    let result = client
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

    let trace = TraceResult {
        session_id,
        agent,
        event_count: events.len() as u64,
        started_at,
        ended_at,
        has_errors,
        events,
    };

    output::render(&trace, &config.format)?;
    Ok(())
}
