use anyhow::Result;
use serde::Serialize;

use crate::bigquery::client::{QueryRequest, BigQueryClient};
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
    config: &Config,
) -> Result<()> {
    if dry_run {
        let output = DryRunOutput {
            dry_run: true,
            url: format!(
                "https://bigquery.googleapis.com/bigquery/v2/projects/{}/queries",
                config.project_id
            ),
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

    let client = BigQueryClient::new().await?;
    let req = QueryRequest {
        query,
        use_legacy_sql,
        location: config.location.clone(),
        max_results: None,
        timeout_ms: Some(30000),
    };

    let result = client.query(&config.project_id, req).await?;

    let output = QueryOutput {
        total_rows: result.total_rows,
        rows: result.rows,
    };

    output::render(&output, &config.format)?;
    Ok(())
}
