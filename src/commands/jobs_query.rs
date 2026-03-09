use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::bigquery::client::{BigQueryClient, QueryRequest};
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

pub async fn run(
    query: String,
    use_legacy_sql: bool,
    dry_run: bool,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    if dry_run {
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
        output::render(&output, &config.format)?;
        return Ok(());
    }

    let resolved = auth::resolve(auth_opts).await?;
    let client = BigQueryClient::new(resolved);
    let req = QueryRequest {
        query,
        use_legacy_sql,
        location: config.location.clone(),
        max_results: None,
        timeout_ms: Some(30000),
    };

    let result = client.query(&config.project_id, req).await?;

    if config.format == OutputFormat::Text {
        let columns: Vec<String> = result
            .rows
            .first()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default();
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

    output::render(&output, &config.format)?;
    Ok(())
}
