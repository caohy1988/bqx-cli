use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest};
use crate::cli::OutputFormat;
use crate::config::Config;
use crate::output;

#[derive(Serialize)]
struct DryRunOutput {
    dry_run: bool,
    url: String,
    method: String,
    body: DryRunBody,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DryRunBody {
    query: String,
    use_legacy_sql: bool,
    location: String,
}

#[derive(Serialize)]
struct QueryOutput {
    total_rows: u64,
    rows: Vec<serde_json::Map<String, serde_json::Value>>,
}

// ── SQL builder ──

pub fn build_query_request(query: String, use_legacy_sql: bool, location: String) -> QueryRequest {
    QueryRequest {
        query,
        use_legacy_sql,
        location,
        max_results: None,
        timeout_ms: Some(30000),
    }
}

// ── Entry points ──

pub async fn run(
    query: String,
    use_legacy_sql: bool,
    dry_run: bool,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if dry_run {
        return run_dry_run(query, use_legacy_sql, config);
    }

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved.clone());
    run_with_executor_and_sanitize(&client, query, use_legacy_sql, config, &resolved).await
}

async fn run_with_executor_and_sanitize(
    executor: &dyn QueryExecutor,
    query: String,
    use_legacy_sql: bool,
    config: &Config,
    auth: &crate::auth::ResolvedAuth,
) -> Result<()> {
    let req = build_query_request(query, use_legacy_sql, config.location.clone());
    let result = executor.query(&config.project_id, req).await?;

    let output = QueryOutput {
        total_rows: result.total_rows,
        rows: result.rows.clone(),
    };

    // Sanitize output if template is configured.
    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&output)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(auth, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);

        if sanitize_result.sanitized {
            // If content was flagged, render the redacted output instead.
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    // No sanitization or no flags — render normally.
    if config.format == OutputFormat::Text {
        let columns: Vec<String> = result
            .schema
            .fields
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let rows: Vec<Vec<String>> = result
            .rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|col| {
                        row.get(col)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Null => "null".into(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        output::text::render_query(result.total_rows, &columns, &rows);
        return Ok(());
    }

    crate::output::render(&output, &config.format)
}

pub async fn run_with_executor(
    executor: &dyn QueryExecutor,
    query: String,
    use_legacy_sql: bool,
    config: &Config,
) -> Result<()> {
    let req = build_query_request(query, use_legacy_sql, config.location.clone());
    let result = executor.query(&config.project_id, req).await?;

    if config.format == OutputFormat::Text {
        let columns: Vec<String> = result
            .schema
            .fields
            .iter()
            .map(|f| f.name.clone())
            .collect();
        let rows: Vec<Vec<String>> = result
            .rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|col| {
                        row.get(col)
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Null => "null".into(),
                                other => other.to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .collect();
        output::text::render_query(result.total_rows, &columns, &rows);
        return Ok(());
    }

    let output = QueryOutput {
        total_rows: result.total_rows,
        rows: result.rows,
    };
    output::render(&output, &config.format)
}

fn run_dry_run(query: String, use_legacy_sql: bool, config: &Config) -> Result<()> {
    let url = format!(
        "https://bigquery.googleapis.com/bigquery/v2/projects/{}/queries",
        config.project_id
    );

    if config.format == OutputFormat::Text {
        output::text::render_query_dry_run(&url, &query, use_legacy_sql, &config.location);
        return Ok(());
    }

    let output = DryRunOutput {
        dry_run: true,
        url,
        method: "POST".into(),
        body: DryRunBody {
            query,
            use_legacy_sql,
            location: config.location.clone(),
        },
    };
    output::render(&output, &config.format)
}
