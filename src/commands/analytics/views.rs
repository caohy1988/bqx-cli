use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

/// The 18 standard ADK event types.
const EVENT_TYPES: &[&str] = &[
    "LLM_REQUEST",
    "LLM_RESPONSE",
    "TOOL_STARTING",
    "TOOL_COMPLETED",
    "TOOL_ERROR",
    "TOOL_CALL",
    "TOOL_RESPONSE",
    "AGENT_RUN_START",
    "AGENT_RUN_END",
    "AGENT_RUN_ERROR",
    "INVOCATION_START",
    "INVOCATION_COMPLETED",
    "INVOCATION_ERROR",
    "HUMAN_INPUT_REQUIRED",
    "HUMAN_INPUT_RECEIVED",
    "SESSION_START",
    "SESSION_END",
    "SESSION_ERROR",
];

const CREATE_VIEW_SQL: &str = r#"
CREATE OR REPLACE VIEW `{project}.{dataset}.{view_name}` AS
SELECT *
FROM `{project}.{dataset}.{table}`
WHERE event_type = '{event_type}'
"#;

#[derive(Serialize)]
pub struct ViewsCreateResult {
    pub views: Vec<ViewStatus>,
    pub created: usize,
    pub failed: usize,
    pub prefix: String,
}

#[derive(Serialize)]
pub struct ViewStatus {
    pub view_name: String,
    pub event_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ── SQL builder ──

pub fn build_create_view_sql(
    project: &str,
    dataset: &str,
    table: &str,
    prefix: &str,
    event_type: &str,
) -> (String, String) {
    let view_name = format!("{}{}", prefix, event_type.to_lowercase());
    let sql = CREATE_VIEW_SQL
        .replace("{project}", project)
        .replace("{dataset}", dataset)
        .replace("{table}", table)
        .replace("{view_name}", &view_name)
        .replace("{event_type}", event_type);
    (view_name, sql)
}

/// Return the list of (view_name, sql) for all event types.
pub fn build_all_view_sqls(
    project: &str,
    dataset: &str,
    table: &str,
    prefix: &str,
) -> Vec<(String, String, &'static str)> {
    EVENT_TYPES
        .iter()
        .map(|et| {
            let (view_name, sql) = build_create_view_sql(project, dataset, table, prefix, et);
            (view_name, sql, *et)
        })
        .collect()
}

// ── Data builder ──

async fn build_views_create(
    executor: &dyn QueryExecutor,
    prefix: &str,
    config: &Config,
) -> Result<ViewsCreateResult> {
    let dataset_id = config.require_dataset_id()?;
    let views_to_create =
        build_all_view_sqls(&config.project_id, dataset_id, &config.table, prefix);

    let mut views = Vec::new();
    let mut created = 0usize;
    let mut failed = 0usize;

    for (view_name, sql, event_type) in &views_to_create {
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
                views.push(ViewStatus {
                    view_name: view_name.clone(),
                    event_type: event_type.to_string(),
                    status: "created".to_string(),
                    error: None,
                });
            }
            Err(e) => {
                failed += 1;
                views.push(ViewStatus {
                    view_name: view_name.clone(),
                    event_type: event_type.to_string(),
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(ViewsCreateResult {
        views,
        created,
        failed,
        prefix: prefix.to_string(),
    })
}

fn render_views_create(result: &ViewsCreateResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            output::text::render_views_create(result);
        }
        OutputFormat::Table => {
            let columns = vec!["view_name".into(), "event_type".into(), "status".into()];
            let rows: Vec<Vec<String>> = result
                .views
                .iter()
                .map(|v| vec![v.view_name.clone(), v.event_type.clone(), v.status.clone()])
                .collect();
            println!(
                "Created: {}  Failed: {}  Prefix: {}",
                result.created, result.failed, result.prefix
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

fn check_failures(result: &ViewsCreateResult) -> Result<()> {
    if result.failed > 0 {
        anyhow::bail!(
            "{} of {} views failed to create",
            result.failed,
            result.views.len()
        );
    }
    Ok(())
}

pub async fn run(prefix: String, auth_opts: &AuthOptions, config: &Config) -> Result<()> {
    config::validate_view_prefix(&prefix)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_views_create(&client, &prefix, config).await?;

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

    render_views_create(&result, config)?;
    check_failures(&result)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    prefix: String,
    config: &Config,
) -> Result<()> {
    let result = build_views_create(executor, &prefix, config).await?;
    render_views_create(&result, config)?;
    check_failures(&result)
}

// ── Single-view create ──

/// Check whether the given event type is one of the 18 standard types.
/// Returns true if known, false if custom. Does not reject custom types —
/// the SDK passes event_type through directly and lets BigQuery handle it.
pub fn is_known_event_type(event_type: &str) -> bool {
    let upper = event_type.to_uppercase();
    EVENT_TYPES.contains(&upper.as_str())
}

/// Warn on stderr if the event type is not one of the 18 standard types.
/// Does not error — custom event types are passed through to BigQuery.
fn warn_if_custom_event_type(event_type: &str) {
    if !is_known_event_type(event_type) {
        eprintln!(
            "Warning: '{}' is not a standard ADK event type. Standard types: {}",
            event_type,
            EVENT_TYPES.join(", ")
        );
    }
}

#[derive(Serialize)]
pub struct ViewCreateResult {
    pub view_name: String,
    pub event_type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

async fn build_view_create(
    executor: &dyn QueryExecutor,
    event_type: &str,
    prefix: &str,
    config: &Config,
) -> Result<ViewCreateResult> {
    let dataset_id = config.require_dataset_id()?;
    let canonical = event_type.to_uppercase();
    let (view_name, sql) = build_create_view_sql(
        &config.project_id,
        dataset_id,
        &config.table,
        prefix,
        &canonical,
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
        .await;

    match result {
        Ok(_) => Ok(ViewCreateResult {
            view_name,
            event_type: canonical,
            status: "created".to_string(),
            error: None,
        }),
        Err(e) => Ok(ViewCreateResult {
            view_name,
            event_type: canonical,
            status: "failed".to_string(),
            error: Some(e.to_string()),
        }),
    }
}

fn render_view_create(result: &ViewCreateResult, config: &Config) -> Result<()> {
    match config.format {
        OutputFormat::Text => {
            if result.status == "created" {
                println!("Created view: {}", result.view_name);
            } else {
                println!(
                    "Failed to create view {}: {}",
                    result.view_name,
                    result.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
        OutputFormat::Table => {
            let columns = vec![
                "view_name".into(),
                "event_type".into(),
                "status".into(),
            ];
            let rows = vec![vec![
                result.view_name.clone(),
                result.event_type.clone(),
                result.status.clone(),
            ]];
            output::render_rows_as_table(&columns, &rows)?;
        }
        OutputFormat::Json => {
            output::render(result, &config.format)?;
        }
    }
    Ok(())
}

pub async fn run_create(
    event_type: String,
    prefix: String,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    warn_if_custom_event_type(&event_type);
    config::validate_view_prefix(&prefix)?;
    config.require_dataset_id()?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    let result = build_view_create(&client, &event_type, &prefix, config).await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&result)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            crate::output::render(&sanitize_result.content, &config.format)?;
            if result.status == "failed" {
                anyhow::bail!("View creation failed");
            }
            return Ok(());
        }
    }

    render_view_create(&result, config)?;
    if result.status == "failed" {
        anyhow::bail!("View creation failed");
    }
    Ok(())
}

pub async fn run_create_with_executor(
    executor: &dyn QueryExecutor,
    event_type: String,
    prefix: String,
    config: &Config,
) -> Result<()> {
    warn_if_custom_event_type(&event_type);
    let result = build_view_create(executor, &event_type, &prefix, config).await?;
    render_view_create(&result, config)?;
    if result.status == "failed" {
        anyhow::bail!("View creation failed");
    }
    Ok(())
}
