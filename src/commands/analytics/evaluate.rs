use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::{EvaluatorType, OutputFormat};
use crate::config::{self, Config};
use crate::models::BqxError;
use crate::output;

const LATENCY_EVAL_SQL: &str = r#"
WITH session_latency AS (
  SELECT
    session_id,
    agent,
    MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS max_latency_ms,
    AVG(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS FLOAT64)) AS avg_latency_ms,
    COUNTIF(latency_ms IS NOT NULL) AS has_latency_count
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
  GROUP BY session_id, agent
)
SELECT
  session_id,
  agent,
  max_latency_ms,
  avg_latency_ms,
  has_latency_count = 0 AS no_latency_data,
  CASE
    WHEN has_latency_count = 0 THEN false
    WHEN max_latency_ms <= {threshold} THEN true
    ELSE false
  END AS passed
FROM session_latency
ORDER BY max_latency_ms DESC NULLS LAST
LIMIT {limit}
"#;

const ERROR_RATE_EVAL_SQL: &str = r#"
WITH session_errors AS (
  SELECT
    session_id,
    agent,
    COUNT(*) AS total_events,
    COUNTIF(
      ENDS_WITH(event_type, '_ERROR')
      OR error_message IS NOT NULL
      OR status = 'ERROR'
    ) AS error_events
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
  GROUP BY session_id, agent
)
SELECT
  session_id,
  agent,
  total_events,
  error_events,
  SAFE_DIVIDE(error_events, total_events) AS error_rate,
  CASE WHEN SAFE_DIVIDE(error_events, total_events) <= {threshold} THEN true ELSE false END AS passed
FROM session_errors
ORDER BY error_rate DESC
LIMIT {limit}
"#;

const TURN_COUNT_EVAL_SQL: &str = r#"
WITH session_turns AS (
  SELECT
    session_id,
    agent,
    COUNTIF(event_type = 'HUMAN_INPUT_RECEIVED') AS turn_count
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
  GROUP BY session_id, agent
)
SELECT
  session_id,
  agent,
  turn_count,
  CASE WHEN turn_count <= {threshold} THEN true ELSE false END AS passed
FROM session_turns
ORDER BY turn_count DESC
LIMIT {limit}
"#;

const TOKEN_EFFICIENCY_EVAL_SQL: &str = r#"
WITH session_tokens AS (
  SELECT
    session_id,
    agent,
    SUM(CAST(IFNULL(JSON_VALUE(content, '$.total_tokens'), '0') AS INT64)) AS total_tokens
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
    AND event_type IN ('LLM_REQUEST', 'LLM_RESPONSE')
  GROUP BY session_id, agent
)
SELECT
  session_id,
  agent,
  total_tokens,
  CASE WHEN total_tokens <= {threshold} THEN true ELSE false END AS passed
FROM session_tokens
ORDER BY total_tokens DESC
LIMIT {limit}
"#;

const TTFT_EVAL_SQL: &str = r#"
WITH session_ttft AS (
  SELECT
    session_id,
    agent,
    TIMESTAMP_DIFF(
      MIN(IF(event_type = 'LLM_RESPONSE', timestamp, NULL)),
      MIN(timestamp),
      MILLISECOND
    ) AS ttft_ms
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
  GROUP BY session_id, agent
  HAVING ttft_ms IS NOT NULL
)
SELECT
  session_id,
  agent,
  ttft_ms,
  CASE WHEN ttft_ms <= {threshold} THEN true ELSE false END AS passed
FROM session_ttft
ORDER BY ttft_ms DESC
LIMIT {limit}
"#;

const COST_EVAL_SQL: &str = r#"
WITH session_cost AS (
  SELECT
    session_id,
    agent,
    SUM(CAST(IFNULL(JSON_VALUE(content, '$.cost_usd'), '0') AS FLOAT64)) AS cost_usd
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
  GROUP BY session_id, agent
)
SELECT
  session_id,
  agent,
  cost_usd,
  CASE WHEN cost_usd <= {threshold} THEN true ELSE false END AS passed
FROM session_cost
ORDER BY cost_usd DESC
LIMIT {limit}
"#;

#[derive(Serialize)]
pub struct EvalResult {
    pub evaluator: String,
    pub threshold: f64,
    pub time_window: String,
    pub agent_id: Option<String>,
    pub total_sessions: u64,
    pub passed: u64,
    pub failed: u64,
    pub pass_rate: f64,
    pub sessions: Vec<SessionEval>,
}

#[derive(Serialize)]
pub struct SessionEval {
    pub session_id: String,
    pub agent: String,
    pub passed: bool,
    pub score: f64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_latency_data: bool,
}

// ── SQL builder ──

#[allow(clippy::too_many_arguments)]
pub fn build_evaluate_query(
    evaluator: &EvaluatorType,
    project: &str,
    dataset: &str,
    table: &str,
    interval_sql: &str,
    threshold: f64,
    agent_id: Option<&str>,
    limit: u32,
) -> String {
    let agent_filter = match agent_id {
        Some(id) => format!("AND agent = '{id}'"),
        None => String::new(),
    };

    let sql_template = match evaluator {
        EvaluatorType::Latency => LATENCY_EVAL_SQL,
        EvaluatorType::ErrorRate => ERROR_RATE_EVAL_SQL,
        EvaluatorType::TurnCount => TURN_COUNT_EVAL_SQL,
        EvaluatorType::TokenEfficiency => TOKEN_EFFICIENCY_EVAL_SQL,
        EvaluatorType::Ttft => TTFT_EVAL_SQL,
        EvaluatorType::Cost => COST_EVAL_SQL,
        EvaluatorType::LlmJudge => unreachable!("LLM judge validated before SQL build"),
    };

    sql_template
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{interval}", interval_sql)
        .replace("{threshold}", &threshold.to_string())
        .replace("{agent_filter}", &agent_filter)
        .replace("{limit}", &limit.to_string())
}

// ── Result mapper ──

pub fn eval_result_from_rows(
    evaluator: &EvaluatorType,
    threshold: f64,
    time_window: String,
    agent_id: Option<String>,
    result: &QueryResult,
) -> EvalResult {
    let evaluator_name = match evaluator {
        EvaluatorType::Latency => "latency",
        EvaluatorType::ErrorRate => "error_rate",
        EvaluatorType::TurnCount => "turn_count",
        EvaluatorType::TokenEfficiency => "token_efficiency",
        EvaluatorType::Ttft => "ttft",
        EvaluatorType::Cost => "cost",
        EvaluatorType::LlmJudge => "llm-judge",
    };

    let sessions: Vec<SessionEval> = result
        .rows
        .iter()
        .map(|row| {
            let score_key = match evaluator {
                EvaluatorType::Latency => "max_latency_ms",
                EvaluatorType::ErrorRate => "error_rate",
                EvaluatorType::TurnCount => "turn_count",
                EvaluatorType::TokenEfficiency => "total_tokens",
                EvaluatorType::Ttft => "ttft_ms",
                EvaluatorType::Cost => "cost_usd",
                EvaluatorType::LlmJudge => "score",
            };
            let score = row
                .get(score_key)
                .and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                })
                .unwrap_or(0.0);
            let passed = row
                .get("passed")
                .and_then(|v| v.as_str())
                .map(|s| s == "true")
                .unwrap_or(false);
            let no_latency_data = row
                .get("no_latency_data")
                .and_then(|v| v.as_str())
                .map(|s| s == "true")
                .unwrap_or(false);

            SessionEval {
                session_id: row
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                agent: row
                    .get("agent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                passed,
                score,
                no_latency_data,
            }
        })
        .collect();

    let total = sessions.len() as u64;
    let passed_count = sessions.iter().filter(|s| s.passed).count() as u64;
    let failed_count = total - passed_count;
    let pass_rate = if total > 0 {
        passed_count as f64 / total as f64
    } else {
        1.0
    };

    EvalResult {
        evaluator: evaluator_name.into(),
        threshold,
        time_window,
        agent_id,
        total_sessions: total,
        passed: passed_count,
        failed: failed_count,
        pass_rate,
        sessions,
    }
}

// ── Data builder (shared by run + run_with_executor) ──

async fn build_eval_result(
    executor: &dyn QueryExecutor,
    evaluator: &EvaluatorType,
    threshold: f64,
    last: &str,
    agent_id: Option<&str>,
    limit: u32,
    config: &Config,
) -> Result<EvalResult> {
    if let Some(id) = agent_id {
        config::validate_agent_id(id)?;
    }

    let duration = config::parse_duration(last)?;
    let dataset_id = config.require_dataset_id()?;

    let sql = build_evaluate_query(
        evaluator,
        &config.project_id,
        dataset_id,
        &config.table,
        &duration.interval_sql,
        threshold,
        agent_id,
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

    Ok(eval_result_from_rows(
        evaluator,
        threshold,
        last.to_string(),
        agent_id.map(|s| s.to_string()),
        &result,
    ))
}

fn render_eval(eval_result: &EvalResult, format: &OutputFormat) -> Result<()> {
    if *format == OutputFormat::Text {
        output::text::render_evaluate(eval_result);
    } else {
        output::render(eval_result, format)?;
    }
    Ok(())
}

// ── Entry points ──

#[allow(clippy::too_many_arguments)]
pub async fn run(
    evaluator: EvaluatorType,
    threshold: f64,
    last: String,
    agent_id: Option<String>,
    exit_code: bool,
    criterion: String,
    limit: u32,
    strict: bool,
    endpoint: Option<String>,
    connection_id: Option<String>,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if matches!(evaluator, EvaluatorType::LlmJudge) {
        anyhow::bail!(
            "--evaluator=llm-judge is not yet supported. LLM-based evaluation \
             (AI.GENERATE) is planned for a future milestone. Use a code evaluator \
             (latency, error-rate, turn-count, token-efficiency, ttft, cost)."
        );
    }
    if endpoint.is_some() || connection_id.is_some() {
        anyhow::bail!(
            "--endpoint and --connection-id are not yet supported. \
             LLM-based evaluation (AI.GENERATE) is planned for a future milestone."
        );
    }
    if criterion != "correctness" {
        eprintln!(
            "Warning: --criterion={criterion} is accepted for CLI compatibility \
             but has no effect on code-based evaluators. It will apply once \
             llm-judge is implemented."
        );
    }
    if strict {
        eprintln!(
            "Warning: --strict is accepted for CLI compatibility but has no \
             effect on code-based evaluators. It will apply once llm-judge \
             is implemented."
        );
    }
    if let Some(ref id) = agent_id {
        config::validate_agent_id(id)?;
    }
    config::parse_duration(&last)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let eval_result = build_eval_result(
        &client,
        &evaluator,
        threshold,
        &last,
        agent_id.as_deref(),
        limit,
        config,
    )
    .await?;

    let failed_count = eval_result.failed;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&eval_result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            crate::output::render(&sanitize_result.content, &config.format)?;
            if exit_code && failed_count > 0 {
                return Err(BqxError::EvalFailed { exit_code: 1 }.into());
            }
            return Ok(());
        }
    }

    render_eval(&eval_result, &config.format)?;

    if exit_code && failed_count > 0 {
        return Err(BqxError::EvalFailed { exit_code: 1 }.into());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    evaluator: EvaluatorType,
    threshold: f64,
    last: String,
    agent_id: Option<String>,
    exit_code: bool,
    limit: u32,
    config: &Config,
) -> Result<()> {
    let eval_result = build_eval_result(
        executor,
        &evaluator,
        threshold,
        &last,
        agent_id.as_deref(),
        limit,
        config,
    )
    .await?;

    let failed_count = eval_result.failed;
    render_eval(&eval_result, &config.format)?;

    if exit_code && failed_count > 0 {
        return Err(BqxError::EvalFailed { exit_code: 1 }.into());
    }

    Ok(())
}
