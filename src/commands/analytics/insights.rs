use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

const INSIGHTS_SQL: &str = r#"
WITH session_summary AS (
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
    ) AS error_count,
    COUNTIF(event_type = 'LLM_REQUEST') AS llm_requests,
    COUNTIF(event_type = 'TOOL_CALL' OR event_type = 'TOOL_STARTING') AS tool_calls,
    MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS max_latency_ms,
    AVG(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS avg_latency_ms
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
  GROUP BY session_id, agent
)
SELECT
  COUNT(*) AS total_sessions,
  SUM(event_count) AS total_events,
  SUM(error_count) AS total_errors,
  SAFE_DIVIDE(SUM(error_count), SUM(event_count)) AS error_rate,
  COUNTIF(error_count > 0) AS sessions_with_errors,
  SAFE_DIVIDE(COUNTIF(error_count > 0), COUNT(*)) AS session_error_rate,
  AVG(event_count) AS avg_events_per_session,
  SUM(llm_requests) AS total_llm_requests,
  SUM(tool_calls) AS total_tool_calls,
  MAX(max_latency_ms) AS peak_latency_ms,
  AVG(avg_latency_ms) AS avg_latency_ms,
  MIN(started_at) AS earliest_session,
  MAX(ended_at) AS latest_session
FROM session_summary
"#;

const TOP_ERRORS_SQL: &str = r#"
SELECT
  event_type,
  IFNULL(error_message, '(no message)') AS error_message,
  COUNT(*) AS occurrences
FROM `{project}.{dataset}.{table}`
WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
  {agent_filter}
  AND (
    ENDS_WITH(event_type, '_ERROR')
    OR error_message IS NOT NULL
    OR status = 'ERROR'
  )
GROUP BY event_type, error_message
ORDER BY occurrences DESC
LIMIT 5
"#;

const TOP_TOOLS_SQL: &str = r#"
SELECT
  IFNULL(JSON_VALUE(content, '$.tool_name'), event_type) AS tool_name,
  COUNT(*) AS call_count,
  AVG(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS avg_latency_ms,
  MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS max_latency_ms
FROM `{project}.{dataset}.{table}`
WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
  {agent_filter}
  AND event_type IN ('TOOL_CALL', 'TOOL_STARTING', 'TOOL_COMPLETED')
GROUP BY tool_name
ORDER BY call_count DESC
LIMIT 10
"#;

#[derive(Serialize)]
pub struct InsightsResult {
    pub time_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub summary: InsightsSummary,
    pub top_errors: Vec<TopError>,
    pub top_tools: Vec<TopTool>,
}

#[derive(Serialize)]
pub struct InsightsSummary {
    pub total_sessions: u64,
    pub total_events: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub sessions_with_errors: u64,
    pub session_error_rate: f64,
    pub avg_events_per_session: f64,
    pub total_llm_requests: u64,
    pub total_tool_calls: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest_session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_session: Option<String>,
}

#[derive(Serialize)]
pub struct TopError {
    pub event_type: String,
    pub error_message: String,
    pub occurrences: u64,
}

#[derive(Serialize)]
pub struct TopTool {
    pub tool_name: String,
    pub call_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_latency_ms: Option<f64>,
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

pub fn build_insights_query(
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
) -> String {
    replace_common(INSIGHTS_SQL, project, dataset, table, interval_sql, agent_id)
}

pub fn build_top_errors_query(
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
) -> String {
    replace_common(
        TOP_ERRORS_SQL,
        project,
        dataset,
        table,
        interval_sql,
        agent_id,
    )
}

pub fn build_top_tools_query(
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
) -> String {
    replace_common(
        TOP_TOOLS_SQL,
        project,
        dataset,
        table,
        interval_sql,
        agent_id,
    )
}

// ── Result mappers ──

fn get_f64(row: &serde_json::Map<String, serde_json::Value>, key: &str) -> f64 {
    row.get(key)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0.0)
}

fn get_u64(row: &serde_json::Map<String, serde_json::Value>, key: &str) -> u64 {
    row.get(key)
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0)
}

fn get_opt_f64(row: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<f64> {
    row.get(key).and_then(|v| {
        if v.is_null() {
            None
        } else {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        }
    })
}

fn get_opt_str(row: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    row.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn summary_from_rows(result: &QueryResult) -> InsightsSummary {
    let row = match result.rows.first() {
        Some(r) => r,
        None => {
            return InsightsSummary {
                total_sessions: 0,
                total_events: 0,
                total_errors: 0,
                error_rate: 0.0,
                sessions_with_errors: 0,
                session_error_rate: 0.0,
                avg_events_per_session: 0.0,
                total_llm_requests: 0,
                total_tool_calls: 0,
                peak_latency_ms: None,
                avg_latency_ms: None,
                earliest_session: None,
                latest_session: None,
            }
        }
    };

    InsightsSummary {
        total_sessions: get_u64(row, "total_sessions"),
        total_events: get_u64(row, "total_events"),
        total_errors: get_u64(row, "total_errors"),
        error_rate: get_f64(row, "error_rate"),
        sessions_with_errors: get_u64(row, "sessions_with_errors"),
        session_error_rate: get_f64(row, "session_error_rate"),
        avg_events_per_session: get_f64(row, "avg_events_per_session"),
        total_llm_requests: get_u64(row, "total_llm_requests"),
        total_tool_calls: get_u64(row, "total_tool_calls"),
        peak_latency_ms: get_opt_f64(row, "peak_latency_ms"),
        avg_latency_ms: get_opt_f64(row, "avg_latency_ms"),
        earliest_session: get_opt_str(row, "earliest_session"),
        latest_session: get_opt_str(row, "latest_session"),
    }
}

pub fn top_errors_from_rows(result: &QueryResult) -> Vec<TopError> {
    result
        .rows
        .iter()
        .map(|row| TopError {
            event_type: get_opt_str(row, "event_type").unwrap_or_default(),
            error_message: get_opt_str(row, "error_message").unwrap_or_default(),
            occurrences: get_u64(row, "occurrences"),
        })
        .collect()
}

pub fn top_tools_from_rows(result: &QueryResult) -> Vec<TopTool> {
    result
        .rows
        .iter()
        .map(|row| TopTool {
            tool_name: get_opt_str(row, "tool_name").unwrap_or_default(),
            call_count: get_u64(row, "call_count"),
            avg_latency_ms: get_opt_f64(row, "avg_latency_ms"),
            max_latency_ms: get_opt_f64(row, "max_latency_ms"),
        })
        .collect()
}

// ── Data builder ──

async fn build_insights(
    executor: &dyn QueryExecutor,
    last: &str,
    agent_id: Option<&str>,
    config: &Config,
) -> Result<InsightsResult> {
    if let Some(id) = agent_id {
        config::validate_agent_id(id)?;
    }
    let parsed = config::parse_duration(last)?;
    let dataset_id = config.require_dataset_id()?;

    let query_args = (
        config.project_id.as_str(),
        dataset_id,
        config.table.as_str(),
        parsed.interval_sql.as_str(),
        agent_id,
    );

    let summary_sql = build_insights_query(query_args.0, query_args.1, query_args.2, query_args.3, query_args.4);
    let errors_sql = build_top_errors_query(query_args.0, query_args.1, query_args.2, query_args.3, query_args.4);
    let tools_sql = build_top_tools_query(query_args.0, query_args.1, query_args.2, query_args.3, query_args.4);

    let make_req = |sql: String| QueryRequest {
        query: sql,
        use_legacy_sql: false,
        location: config.location.clone(),
        max_results: None,
        timeout_ms: Some(30000),
    };

    let summary_result = executor.query(&config.project_id, make_req(summary_sql)).await?;
    let errors_result = executor.query(&config.project_id, make_req(errors_sql)).await?;
    let tools_result = executor.query(&config.project_id, make_req(tools_sql)).await?;

    Ok(InsightsResult {
        time_window: last.to_string(),
        agent_id: agent_id.map(|s| s.to_string()),
        summary: summary_from_rows(&summary_result),
        top_errors: top_errors_from_rows(&errors_result),
        top_tools: top_tools_from_rows(&tools_result),
    })
}

fn render_insights(result: &InsightsResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_insights(result);
        }
        OutputFormat::Table => {
            let s = &result.summary;
            println!("Insights: Window={}  Sessions={}", result.time_window, s.total_sessions);
            if let Some(ref agent) = result.agent_id {
                println!("Agent: {agent}");
            }
            println!();

            let columns = vec![
                "metric".into(),
                "value".into(),
            ];
            let rows: Vec<Vec<String>> = vec![
                vec!["total_events".into(), s.total_events.to_string()],
                vec!["total_errors".into(), s.total_errors.to_string()],
                vec!["error_rate".into(), format!("{:.4}", s.error_rate)],
                vec!["sessions_with_errors".into(), s.sessions_with_errors.to_string()],
                vec!["total_llm_requests".into(), s.total_llm_requests.to_string()],
                vec!["total_tool_calls".into(), s.total_tool_calls.to_string()],
                vec!["peak_latency_ms".into(), s.peak_latency_ms.map(|v| format!("{v:.1}")).unwrap_or("-".into())],
                vec!["avg_latency_ms".into(), s.avg_latency_ms.map(|v| format!("{v:.1}")).unwrap_or("-".into())],
            ];
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
    let result = build_insights(&client, &last, agent_id.as_deref(), config).await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    render_insights(&result, config)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    last: String,
    agent_id: Option<String>,
    config: &Config,
) -> Result<()> {
    let result = build_insights(executor, &last, agent_id.as_deref(), config).await?;
    render_insights(&result, config)
}
