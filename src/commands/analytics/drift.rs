use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest, QueryResult};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::models::BqxError;
use crate::output;

/// Compares current agent behaviour against a golden question dataset.
///
/// The golden dataset table is expected to have columns:
///   question (STRING), expected_answer (STRING)
///
/// For each golden question we look for a matching session in the analytics
/// table (within the time window) that contains the question text.
/// Coverage = matched / total golden questions.
const DRIFT_SQL: &str = r#"
WITH golden AS (
  SELECT
    question,
    expected_answer
  FROM `{project}.{dataset}.{golden_dataset}`
),
recent_sessions AS (
  SELECT
    session_id,
    agent,
    IFNULL(JSON_VALUE(content, '$.question'), '') AS question_text,
    IFNULL(JSON_VALUE(content, '$.answer'), '') AS answer_text
  FROM `{project}.{dataset}.{table}`
  WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), {interval})
    {agent_filter}
    AND event_type IN ('HUMAN_INPUT_RECEIVED', 'INVOCATION_COMPLETED', 'LLM_RESPONSE')
),
ranked AS (
  SELECT
    g.question AS golden_question,
    g.expected_answer,
    r.session_id,
    r.answer_text AS actual_answer,
    CASE WHEN r.session_id IS NOT NULL THEN true ELSE false END AS covered,
    ROW_NUMBER() OVER (PARTITION BY g.question ORDER BY r.session_id) AS rn
  FROM golden g
  LEFT JOIN recent_sessions r
    ON LOWER(r.question_text) = LOWER(g.question)
)
SELECT
  golden_question,
  expected_answer,
  session_id,
  actual_answer,
  covered
FROM ranked
WHERE rn = 1
ORDER BY covered ASC, golden_question
"#;

#[derive(Serialize)]
pub struct DriftResult {
    pub golden_dataset: String,
    pub time_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub total_golden: usize,
    pub covered: usize,
    pub uncovered: usize,
    pub coverage: f64,
    pub min_coverage: f64,
    pub passed: bool,
    pub questions: Vec<DriftQuestion>,
}

#[derive(Serialize)]
pub struct DriftQuestion {
    pub golden_question: String,
    pub expected_answer: String,
    pub covered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_answer: Option<String>,
}

// ── SQL builder ──

pub fn build_drift_query(
    project: &str,
    dataset: &str,
    table: &str,
    golden_dataset: &str,
    interval_sql: &str,
    agent_id: Option<&str>,
) -> String {
    let agent_filter = match agent_id {
        Some(id) => format!("AND agent = '{id}'"),
        None => String::new(),
    };
    DRIFT_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{golden_dataset}", golden_dataset)
        .replace("{interval}", interval_sql)
        .replace("{agent_filter}", &agent_filter)
}

// ── Result mapper ──

pub fn drift_from_rows(result: &QueryResult) -> Vec<DriftQuestion> {
    result
        .rows
        .iter()
        .map(|row| {
            let get_str = |key: &str| -> Option<String> {
                row.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
            };
            let covered = row
                .get("covered")
                .map(|v| v.as_bool().unwrap_or(false) || v.as_str() == Some("true"))
                .unwrap_or(false);

            DriftQuestion {
                golden_question: get_str("golden_question").unwrap_or_default(),
                expected_answer: get_str("expected_answer").unwrap_or_default(),
                covered,
                session_id: if covered { get_str("session_id") } else { None },
                actual_answer: if covered {
                    get_str("actual_answer")
                } else {
                    None
                },
            }
        })
        .collect()
}

// ── Data builder ──

async fn build_drift(
    executor: &dyn QueryExecutor,
    golden_dataset: &str,
    last: &str,
    agent_id: Option<&str>,
    min_coverage: f64,
    config: &Config,
) -> Result<DriftResult> {
    if let Some(id) = agent_id {
        config::validate_agent_id(id)?;
    }
    config::validate_threshold_ratio(min_coverage, "min-coverage")?;
    let parsed = config::parse_duration(last)?;
    let dataset_id = config.require_dataset_id()?;
    config::validate_identifier(golden_dataset, "golden-dataset")?;

    let sql = build_drift_query(
        &config.project_id,
        dataset_id,
        &config.table,
        golden_dataset,
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

    let questions = drift_from_rows(&result);
    let total = questions.len();
    let covered_count = questions.iter().filter(|q| q.covered).count();
    let coverage = if total > 0 {
        covered_count as f64 / total as f64
    } else {
        1.0
    };

    Ok(DriftResult {
        golden_dataset: golden_dataset.to_string(),
        time_window: last.to_string(),
        agent_id: agent_id.map(|s| s.to_string()),
        total_golden: total,
        covered: covered_count,
        uncovered: total - covered_count,
        coverage,
        min_coverage,
        passed: coverage >= min_coverage,
        questions,
    })
}

fn render_drift(result: &DriftResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_drift(result);
        }
        OutputFormat::Table => {
            println!(
                "Drift: golden={}  coverage={:.2}  min={:.2}  {}",
                result.golden_dataset,
                result.coverage,
                result.min_coverage,
                if result.passed { "PASSED" } else { "FAILED" }
            );
            if let Some(ref agent) = result.agent_id {
                println!("Agent: {agent}");
            }
            println!();
            let columns = vec!["question".into(), "covered".into(), "session_id".into()];
            let rows: Vec<Vec<String>> = result
                .questions
                .iter()
                .map(|q| {
                    vec![
                        q.golden_question.clone(),
                        q.covered.to_string(),
                        q.session_id.clone().unwrap_or("-".into()),
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
    golden_dataset: String,
    last: String,
    agent_id: Option<String>,
    min_coverage: f64,
    exit_code: bool,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if let Some(ref id) = agent_id {
        config::validate_agent_id(id)?;
    }
    config::validate_threshold_ratio(min_coverage, "min-coverage")?;
    config::parse_duration(&last)?;
    config.require_dataset_id()?;
    config::validate_identifier(&golden_dataset, "golden-dataset")?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_drift(
        &client,
        &golden_dataset,
        &last,
        agent_id.as_deref(),
        min_coverage,
        config,
    )
    .await?;

    let passed = result.passed;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            crate::output::render(&sanitize_result.content, &config.format)?;
            if exit_code && !passed {
                return Err(BqxError::EvalFailed { exit_code: 1 }.into());
            }
            return Ok(());
        }
    }

    render_drift(&result, config)?;

    if exit_code && !passed {
        return Err(BqxError::EvalFailed { exit_code: 1 }.into());
    }

    Ok(())
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    golden_dataset: String,
    last: String,
    agent_id: Option<String>,
    min_coverage: f64,
    exit_code: bool,
    config: &Config,
) -> Result<()> {
    let result = build_drift(
        executor,
        &golden_dataset,
        &last,
        agent_id.as_deref(),
        min_coverage,
        config,
    )
    .await?;

    let passed = result.passed;
    render_drift(&result, config)?;

    if exit_code && !passed {
        return Err(BqxError::EvalFailed { exit_code: 1 }.into());
    }

    Ok(())
}
