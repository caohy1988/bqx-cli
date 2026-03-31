use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

// ── View definitions ──

/// Dashboard views created over categorical evaluation results.
///
/// NOTE: The upstream SDK's CategoricalViewManager is not fully public,
/// so these views are dcx's interpretation of useful dashboarding surfaces
/// over categorical results. This is documented as an intentional divergence
/// in the analytics SDK contract.
const CATEGORICAL_VIEWS: &[(&str, &str)] = &[
    (
        "summary",
        r#"
CREATE OR REPLACE VIEW `{project}.{dataset}.{prefix}categorical_summary` AS
SELECT
  metric_name,
  category,
  COUNT(*) AS count,
  COUNT(DISTINCT session_id) AS unique_sessions
FROM `{project}.{dataset}.{results_table}`
GROUP BY metric_name, category
ORDER BY metric_name, count DESC
"#,
    ),
    (
        "timeline",
        r#"
CREATE OR REPLACE VIEW `{project}.{dataset}.{prefix}categorical_timeline` AS
SELECT
  DATE(evaluated_at) AS eval_date,
  metric_name,
  category,
  COUNT(*) AS count
FROM `{project}.{dataset}.{results_table}`
GROUP BY eval_date, metric_name, category
ORDER BY eval_date DESC, metric_name, count DESC
"#,
    ),
    (
        "by_agent",
        r#"
CREATE OR REPLACE VIEW `{project}.{dataset}.{prefix}categorical_by_agent` AS
SELECT
  agent,
  metric_name,
  category,
  COUNT(*) AS count
FROM `{project}.{dataset}.{results_table}`
GROUP BY agent, metric_name, category
ORDER BY agent, metric_name, count DESC
"#,
    ),
    (
        "latest_per_session",
        r#"
CREATE OR REPLACE VIEW `{project}.{dataset}.{prefix}categorical_latest_per_session` AS
SELECT
  r.session_id,
  r.agent,
  r.metric_name,
  r.category,
  r.justification,
  r.prompt_version,
  r.evaluated_at
FROM `{project}.{dataset}.{results_table}` r
INNER JOIN (
  SELECT session_id, metric_name, MAX(evaluated_at) AS max_evaluated_at
  FROM `{project}.{dataset}.{results_table}`
  GROUP BY session_id, metric_name
) latest
ON r.session_id = latest.session_id
  AND r.metric_name = latest.metric_name
  AND r.evaluated_at = latest.max_evaluated_at
ORDER BY r.evaluated_at DESC
"#,
    ),
];

// ── Data types ──

#[derive(Serialize)]
pub struct CategoricalViewsResult {
    pub views: Vec<CategoricalViewStatus>,
    pub created: usize,
    pub failed: usize,
    pub results_table: String,
    pub prefix: String,
}

#[derive(Serialize)]
pub struct CategoricalViewStatus {
    pub view_name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ── SQL builder ──

pub fn build_categorical_view_sqls(
    project: &str,
    dataset: &str,
    results_table: &str,
    prefix: &str,
) -> Vec<(String, String)> {
    CATEGORICAL_VIEWS
        .iter()
        .map(|(name, template)| {
            let view_name = format!("{prefix}categorical_{name}");
            let sql = template
                .replace("{project}", project)
                .replace("{dataset}", dataset)
                .replace("{results_table}", results_table)
                .replace("{prefix}", prefix);
            (view_name, sql)
        })
        .collect()
}

// ── Data builder ──

async fn build_categorical_views(
    executor: &dyn QueryExecutor,
    results_table: &str,
    prefix: &str,
    config: &Config,
) -> Result<CategoricalViewsResult> {
    let dataset_id = config.require_dataset_id()?;
    let views_to_create =
        build_categorical_view_sqls(&config.project_id, dataset_id, results_table, prefix);

    let mut views = Vec::new();
    let mut created = 0usize;
    let mut failed = 0usize;

    for (view_name, sql) in &views_to_create {
        let result = executor
            .query(
                &config.project_id,
                QueryRequest {
                    query: sql.clone(),
                    use_legacy_sql: false,
                    location: config.location.clone(),
                    max_results: None,
                    timeout_ms: Some(30000),
                },
            )
            .await;

        match result {
            Ok(_) => {
                created += 1;
                views.push(CategoricalViewStatus {
                    view_name: view_name.clone(),
                    status: "created".to_string(),
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                views.push(CategoricalViewStatus {
                    view_name: view_name.clone(),
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(CategoricalViewsResult {
        views,
        created,
        failed,
        results_table: results_table.to_string(),
        prefix: prefix.to_string(),
    })
}

fn render_categorical_views(result: &CategoricalViewsResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            println!(
                "Categorical Views: {} created, {} failed (source: {})",
                result.created, result.failed, result.results_table
            );
            for v in &result.views {
                let status_str = if v.status == "created" {
                    "ok"
                } else {
                    "FAILED"
                };
                println!("  {} — {}", v.view_name, status_str);
            }
        }
        OutputFormat::Table => {
            let columns = vec!["view_name".into(), "status".into()];
            let rows: Vec<Vec<String>> = result
                .views
                .iter()
                .map(|v| vec![v.view_name.clone(), v.status.clone()])
                .collect();
            println!(
                "Created: {}  Failed: {}  Source: {}",
                result.created, result.failed, result.results_table
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

fn check_failures(result: &CategoricalViewsResult) -> Result<()> {
    if result.failed > 0 {
        anyhow::bail!(
            "{} of {} categorical views failed to create",
            result.failed,
            result.views.len()
        );
    }
    Ok(())
}

// ── Entry points ──

pub async fn run(
    results_table: String,
    prefix: String,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    config::validate_view_prefix(&prefix)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_categorical_views(&client, &results_table, &prefix, config).await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            crate::output::render(&sanitize_result.content, &config.format)?;
            return check_failures(&result);
        }
    }

    render_categorical_views(&result, config)?;
    check_failures(&result)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    results_table: String,
    prefix: String,
    config: &Config,
) -> Result<()> {
    let result = build_categorical_views(executor, &results_table, &prefix, config).await?;
    render_categorical_views(&result, config)?;
    check_failures(&result)
}
