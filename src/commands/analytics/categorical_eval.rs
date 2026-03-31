use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

// ── SQL templates ──

const LIST_SESSIONS_SQL: &str = r#"
SELECT DISTINCT
  session_id,
  agent
FROM `{project}.{dataset}.{table}`
WHERE TRUE
  {time_filter}
  {agent_filter}
ORDER BY session_id
LIMIT {limit}
"#;

const SESSION_EVENTS_SQL: &str = r#"
SELECT
  event_type,
  timestamp,
  IFNULL(JSON_VALUE(event_data, '$.user_query'), '') AS user_query,
  IFNULL(JSON_VALUE(event_data, '$.agent_response'), '') AS agent_response,
  IFNULL(JSON_VALUE(event_data, '$.tool_name'), '') AS tool_name,
  IFNULL(error_message, '') AS error_message
FROM `{project}.{dataset}.{table}`
WHERE session_id = '{session_id}'
ORDER BY timestamp
"#;

const PERSIST_RESULTS_SQL: &str = r#"
INSERT INTO `{project}.{dataset}.{results_table}` (
  session_id, agent, metric_name, category, justification, prompt_version, evaluated_at
)
VALUES {values}
"#;

// ── Data types ──

#[derive(Deserialize)]
pub struct MetricsFile {
    #[serde(default)]
    pub metrics: Vec<MetricDefinition>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricDefinition {
    pub name: String,
    pub definition: String,
    pub categories: Vec<CategoryDef>,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct CategoryDef {
    pub name: String,
    pub definition: String,
}

#[derive(Serialize)]
pub struct CategoricalEvalResult {
    pub total_sessions: usize,
    pub total_evaluations: usize,
    pub metrics: Vec<MetricSummary>,
    pub sessions: Vec<SessionCategoricalResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub persisted: bool,
}

#[derive(Serialize)]
pub struct MetricSummary {
    pub metric: String,
    pub categories: Vec<CategoryCount>,
}

#[derive(Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: usize,
}

#[derive(Serialize)]
pub struct SessionCategoricalResult {
    pub session_id: String,
    pub agent: String,
    pub classifications: Vec<Classification>,
}

#[derive(Serialize)]
pub struct Classification {
    pub metric: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
}

// ── SQL helpers ──

/// Escape a string for use in a BigQuery SQL literal.
/// Replaces backslashes then single quotes to prevent SQL injection.
fn sql_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Format a value as a SQL string literal, or NULL if None.
fn sql_string_or_null(s: Option<&str>) -> String {
    match s {
        Some(v) => format!("'{}'", sql_escape(v)),
        None => "NULL".to_string(),
    }
}

// ── SQL builder ──

pub fn build_list_sessions_query(
    project: &str,
    dataset: &str,
    table: &str,
    last: Option<&str>,
    agent_id: Option<&str>,
    limit: u32,
) -> Result<String> {
    let time_filter = match last {
        Some(l) => {
            let d = config::parse_duration(l)?;
            format!(
                "AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {})",
                d.interval_sql
            )
        }
        None => String::new(),
    };
    let agent_filter = match agent_id {
        Some(id) => format!("AND agent = '{}'", sql_escape(id)),
        None => String::new(),
    };

    Ok(LIST_SESSIONS_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{time_filter}", &time_filter)
        .replace("{agent_filter}", &agent_filter)
        .replace("{limit}", &limit.to_string()))
}

pub fn build_session_events_query(
    project: &str,
    dataset: &str,
    table: &str,
    session_id: &str,
) -> String {
    SESSION_EVENTS_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{session_id}", &sql_escape(session_id))
}

pub fn build_persist_sql(
    project: &str,
    dataset: &str,
    results_table: &str,
    results: &[SessionCategoricalResult],
    prompt_version: Option<&str>,
) -> String {
    let values: Vec<String> = results
        .iter()
        .flat_map(|s| {
            s.classifications.iter().map(move |c| {
                format!(
                    "({}, {}, {}, {}, {}, {}, CURRENT_TIMESTAMP())",
                    sql_string_or_null(Some(&s.session_id)),
                    sql_string_or_null(Some(&s.agent)),
                    sql_string_or_null(Some(&c.metric)),
                    sql_string_or_null(Some(&c.category)),
                    sql_string_or_null(c.justification.as_deref()),
                    sql_string_or_null(prompt_version),
                )
            })
        })
        .collect();

    PERSIST_RESULTS_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{results_table}", results_table)
        .replace("{values}", &values.join(",\n"))
}

// ── Result mapper ──

pub fn sessions_from_rows(result: &QueryResult) -> Vec<(String, String)> {
    result
        .rows
        .iter()
        .map(|row| {
            let session_id = row
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let agent = row
                .get("agent")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (session_id, agent)
        })
        .collect()
}

/// Classify a session against metric definitions using heuristic rules.
///
/// For a full LLM-judge implementation, this would call AI.GENERATE via BQ.
/// This implementation uses rule-based classification as a first pass,
/// matching the SDK's code-evaluator pattern.
pub fn classify_session(
    events: &QueryResult,
    metrics: &[MetricDefinition],
    include_justification: bool,
) -> Vec<Classification> {
    let mut classifications = Vec::new();

    let event_summary = summarize_events(events);

    for metric in metrics {
        let (category, justification) = match_metric(&event_summary, metric);
        classifications.push(Classification {
            metric: metric.name.clone(),
            category,
            justification: if include_justification {
                Some(justification)
            } else {
                None
            },
        });
    }

    classifications
}

struct EventSummary {
    #[allow(dead_code)]
    total_events: usize,
    error_count: usize,
    has_tool_errors: bool,
    has_user_queries: bool,
    has_agent_responses: bool,
    tool_names: Vec<String>,
}

fn summarize_events(result: &QueryResult) -> EventSummary {
    let mut total_events = 0;
    let mut error_count = 0;
    let mut has_tool_errors = false;
    let mut has_user_queries = false;
    let mut has_agent_responses = false;
    let mut tool_names = Vec::new();

    for row in &result.rows {
        total_events += 1;
        let event_type = row
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let error_message = row
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let user_query = row
            .get("user_query")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let agent_response = row
            .get("agent_response")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tool_name = row
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if event_type.ends_with("_ERROR") || !error_message.is_empty() {
            error_count += 1;
        }
        if event_type == "TOOL_ERROR" {
            has_tool_errors = true;
        }
        if !user_query.is_empty() {
            has_user_queries = true;
        }
        if !agent_response.is_empty() {
            has_agent_responses = true;
        }
        if !tool_name.is_empty() && !tool_names.contains(&tool_name.to_string()) {
            tool_names.push(tool_name.to_string());
        }
    }

    EventSummary {
        total_events,
        error_count,
        has_tool_errors,
        has_user_queries,
        has_agent_responses,
        tool_names,
    }
}

fn match_metric(summary: &EventSummary, metric: &MetricDefinition) -> (String, String) {
    // Use heuristic matching based on metric name patterns.
    // This matches common categorical metrics the SDK defines.
    let definition_lower = metric.definition.to_lowercase();

    if definition_lower.contains("error") || definition_lower.contains("failure") {
        if summary.error_count == 0 {
            return first_or_default(&metric.categories, "no_errors", "No errors in session");
        } else {
            return last_or_default(
                &metric.categories,
                "has_errors",
                &format!("{} error(s) found", summary.error_count),
            );
        }
    }

    if definition_lower.contains("tool") || definition_lower.contains("function") {
        if summary.has_tool_errors {
            return last_or_default(
                &metric.categories,
                "tool_error",
                "Session has tool errors",
            );
        } else if !summary.tool_names.is_empty() {
            return first_or_default(
                &metric.categories,
                "tools_used",
                &format!("Tools used: {}", summary.tool_names.join(", ")),
            );
        } else {
            return first_or_default(
                &metric.categories,
                "no_tools",
                "No tool usage in session",
            );
        }
    }

    if definition_lower.contains("completeness") || definition_lower.contains("response") {
        if summary.has_user_queries && summary.has_agent_responses {
            return first_or_default(
                &metric.categories,
                "complete",
                "Session has both queries and responses",
            );
        } else {
            return last_or_default(
                &metric.categories,
                "incomplete",
                "Session missing queries or responses",
            );
        }
    }

    // Default: pick the first category
    first_or_default(
        &metric.categories,
        "unclassified",
        "No matching heuristic; defaulted to first category",
    )
}

fn first_or_default(
    categories: &[CategoryDef],
    fallback_name: &str,
    justification: &str,
) -> (String, String) {
    let name = categories
        .first()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| fallback_name.to_string());
    (name, justification.to_string())
}

fn last_or_default(
    categories: &[CategoryDef],
    fallback_name: &str,
    justification: &str,
) -> (String, String) {
    let name = categories
        .last()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| fallback_name.to_string());
    (name, justification.to_string())
}

fn build_metric_summaries(
    sessions: &[SessionCategoricalResult],
    metrics: &[MetricDefinition],
) -> Vec<MetricSummary> {
    metrics
        .iter()
        .map(|m| {
            let mut counts: Vec<CategoryCount> = m
                .categories
                .iter()
                .map(|c| CategoryCount {
                    category: c.name.clone(),
                    count: 0,
                })
                .collect();

            for session in sessions {
                for c in &session.classifications {
                    if c.metric == m.name {
                        if let Some(cc) = counts.iter_mut().find(|cc| cc.category == c.category) {
                            cc.count += 1;
                        }
                    }
                }
            }

            MetricSummary {
                metric: m.name.clone(),
                categories: counts,
            }
        })
        .collect()
}

// ── Data builder ──

async fn build_categorical_eval(
    executor: &dyn QueryExecutor,
    metrics: &[MetricDefinition],
    last: Option<&str>,
    agent_id: Option<&str>,
    limit: u32,
    include_justification: bool,
    persist: bool,
    results_table: Option<&str>,
    prompt_version: Option<&str>,
    config: &Config,
) -> Result<CategoricalEvalResult> {
    let dataset_id = config.require_dataset_id()?;

    // Step 1: List matching sessions
    let session_sql = build_list_sessions_query(
        &config.project_id,
        dataset_id,
        &config.table,
        last,
        agent_id,
        limit,
    )?;

    let session_result = executor
        .query(
            &config.project_id,
            QueryRequest {
                query: session_sql,
                use_legacy_sql: false,
                location: config.location.clone(),
                max_results: None,
                timeout_ms: Some(30000),
            },
        )
        .await?;

    let session_ids = sessions_from_rows(&session_result);

    // Step 2: For each session, fetch events and classify
    let mut sessions = Vec::new();
    for (session_id, agent) in &session_ids {
        let events_sql = build_session_events_query(
            &config.project_id,
            dataset_id,
            &config.table,
            session_id,
        );

        let events = executor
            .query(
                &config.project_id,
                QueryRequest {
                    query: events_sql,
                    use_legacy_sql: false,
                    location: config.location.clone(),
                    max_results: None,
                    timeout_ms: Some(30000),
                },
            )
            .await?;

        let classifications = classify_session(&events, metrics, include_justification);
        sessions.push(SessionCategoricalResult {
            session_id: session_id.clone(),
            agent: agent.clone(),
            classifications,
        });
    }

    // Step 3: Build summaries
    let metric_summaries = build_metric_summaries(&sessions, metrics);
    let total_evaluations: usize = sessions.iter().map(|s| s.classifications.len()).sum();

    // Step 4: Optionally persist results
    let persisted = if persist && !sessions.is_empty() {
        let table = results_table.unwrap_or("categorical_results");
        let persist_sql = build_persist_sql(
            &config.project_id,
            dataset_id,
            table,
            &sessions,
            prompt_version,
        );
        executor
            .query(
                &config.project_id,
                QueryRequest {
                    query: persist_sql,
                    use_legacy_sql: false,
                    location: config.location.clone(),
                    max_results: None,
                    timeout_ms: Some(30000),
                },
            )
            .await?;
        true
    } else {
        false
    };

    Ok(CategoricalEvalResult {
        total_sessions: sessions.len(),
        total_evaluations,
        metrics: metric_summaries,
        sessions,
        prompt_version: prompt_version.map(|s| s.to_string()),
        persisted,
    })
}

fn render_categorical_eval(result: &CategoricalEvalResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            println!(
                "Categorical Evaluation: {} sessions, {} evaluations",
                result.total_sessions, result.total_evaluations
            );
            if let Some(ref pv) = result.prompt_version {
                println!("Prompt version: {pv}");
            }
            if result.persisted {
                println!("Results persisted to BigQuery.");
            }
            println!();
            for metric in &result.metrics {
                println!("  {}:", metric.metric);
                for cat in &metric.categories {
                    println!("    {}: {}", cat.category, cat.count);
                }
            }
        }
        OutputFormat::Table => {
            let columns = vec![
                "metric".into(),
                "category".into(),
                "count".into(),
            ];
            let rows: Vec<Vec<String>> = result
                .metrics
                .iter()
                .flat_map(|m| {
                    m.categories
                        .iter()
                        .map(move |c| vec![m.metric.clone(), c.category.clone(), c.count.to_string()])
                })
                .collect();
            println!(
                "Sessions: {}  Evaluations: {}",
                result.total_sessions, result.total_evaluations
            );
            println!();
            output::render_rows_as_table(&columns, &rows)?;
        }
        OutputFormat::Json => {
            output::render(result, &config.format)?;
        }
    }
    Ok(())
}

// ── Entry points ──

pub fn load_metrics_file(path: &str) -> Result<Vec<MetricDefinition>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read metrics file '{}': {}", path, e))?;
    let raw: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid JSON in metrics file '{}': {}", path, e))?;

    let metrics: Vec<MetricDefinition> = if let Some(arr) = raw.as_array() {
        serde_json::from_value(serde_json::Value::Array(arr.clone()))?
    } else if let Some(obj) = raw.as_object() {
        if let Some(arr) = obj.get("metrics").and_then(|v| v.as_array()) {
            serde_json::from_value(serde_json::Value::Array(arr.clone()))?
        } else {
            anyhow::bail!("Metrics file must contain a JSON array or an object with a 'metrics' key");
        }
    } else {
        anyhow::bail!("Metrics file must contain a JSON array or object");
    };

    if metrics.is_empty() {
        anyhow::bail!("No metrics found in metrics file '{}'", path);
    }

    // Reject duplicate metric names — they would collapse in persistence
    // and double-count in summary output.
    let mut seen = std::collections::HashSet::new();
    for m in &metrics {
        if !seen.insert(&m.name) {
            anyhow::bail!(
                "Duplicate metric name '{}' in metrics file '{}'. \
                 Each metric must have a unique name.",
                m.name,
                path
            );
        }
    }

    Ok(metrics)
}

pub async fn run(
    metrics_file: String,
    agent_id: Option<String>,
    last: Option<String>,
    limit: u32,
    endpoint: Option<String>,
    include_justification: bool,
    persist: bool,
    results_table: Option<String>,
    prompt_version: Option<String>,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if endpoint.is_some() {
        anyhow::bail!(
            "--endpoint is not yet supported. LLM-based categorical evaluation \
             (AI.GENERATE) is planned for Milestone C. Remove --endpoint to use \
             heuristic-based classification."
        );
    }
    let metrics = load_metrics_file(&metrics_file)?;
    if let Some(ref id) = agent_id {
        config::validate_agent_id(id)?;
    }
    if let Some(ref l) = last {
        config::parse_duration(l)?;
    }
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_categorical_eval(
        &client,
        &metrics,
        last.as_deref(),
        agent_id.as_deref(),
        limit,
        include_justification,
        persist,
        results_table.as_deref(),
        prompt_version.as_deref(),
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

    render_categorical_eval(&result, config)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    metrics: &[MetricDefinition],
    last: Option<String>,
    agent_id: Option<String>,
    limit: u32,
    include_justification: bool,
    persist: bool,
    results_table: Option<String>,
    prompt_version: Option<String>,
    config: &Config,
) -> Result<()> {
    let result = build_categorical_eval(
        executor,
        metrics,
        last.as_deref(),
        agent_id.as_deref(),
        limit,
        include_justification,
        persist,
        results_table.as_deref(),
        prompt_version.as_deref(),
        config,
    )
    .await?;
    render_categorical_eval(&result, config)
}
