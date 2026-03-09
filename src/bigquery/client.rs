use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::auth::AuthProvider;

const BQ_BASE_URL: &str = "https://bigquery.googleapis.com/bigquery/v2";

pub struct BigQueryClient {
    http: reqwest::Client,
    auth: AuthProvider,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub query: String,
    pub use_legacy_sql: bool,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    pub job_complete: Option<bool>,
    pub job_reference: Option<JobReference>,
    pub schema: Option<TableSchema>,
    pub rows: Option<Vec<TableRow>>,
    pub total_rows: Option<String>,
    pub page_token: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobReference {
    pub job_id: String,
    #[allow(dead_code)]
    pub project_id: Option<String>,
    #[allow(dead_code)]
    pub location: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TableSchema {
    pub fields: Vec<SchemaField>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SchemaField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub mode: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TableRow {
    pub f: Vec<TableCell>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TableCell {
    pub v: Option<serde_json::Value>,
}

pub struct QueryResult {
    #[allow(dead_code)]
    pub schema: TableSchema,
    pub rows: Vec<serde_json::Map<String, serde_json::Value>>,
    pub total_rows: u64,
}

#[derive(Deserialize, Debug)]
struct BqErrorResponse {
    error: Option<BqErrorDetail>,
}

#[derive(Deserialize, Debug)]
struct BqErrorDetail {
    code: Option<u16>,
    message: Option<String>,
}

impl BigQueryClient {
    pub async fn new() -> Result<Self> {
        let auth = AuthProvider::new().await?;
        let http = reqwest::Client::new();
        Ok(Self { http, auth })
    }

    pub async fn query(&self, project: &str, req: QueryRequest) -> Result<QueryResult> {
        let url = format!("{BQ_BASE_URL}/projects/{project}/queries");
        let token = self.auth.token().await?;

        let resp = self
            .http
            .post(&url)
            .bearer_auth(token.as_str())
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body: BqErrorResponse = resp.json().await.unwrap_or(BqErrorResponse {
                error: Some(BqErrorDetail {
                    code: Some(status.as_u16()),
                    message: Some("Unknown BigQuery error".into()),
                }),
            });
            let detail = body.error.unwrap_or(BqErrorDetail {
                code: Some(status.as_u16()),
                message: Some("Unknown error".into()),
            });
            bail!(
                "BigQuery API error {}: {}",
                detail.code.unwrap_or(status.as_u16()),
                detail.message.unwrap_or_default()
            );
        }

        let mut response: QueryResponse = resp.json().await?;
        let location = req.location.clone();

        // Poll if job not complete
        if response.job_complete == Some(false) {
            if let Some(ref job_ref) = response.job_reference {
                let job_id = job_ref.job_id.clone();
                response = self.poll_results(project, &job_id, &location).await?;
            }
        }

        let schema = response
            .schema
            .ok_or_else(|| anyhow::anyhow!("No schema in response"))?;

        let mut all_rows = response.rows.unwrap_or_default();

        // Paginate
        let mut page_token = response.page_token;
        while let Some(ref token) = page_token {
            if let Some(ref job_ref) = response.job_reference {
                let url = format!(
                    "{BQ_BASE_URL}/projects/{project}/queries/{}",
                    job_ref.job_id
                );
                let tkn = self.auth.token().await?;
                let page_resp: QueryResponse = self
                    .http
                    .get(&url)
                    .bearer_auth(tkn.as_str())
                    .query(&[("location", &location), ("pageToken", token)])
                    .send()
                    .await?
                    .json()
                    .await?;

                if let Some(rows) = page_resp.rows {
                    all_rows.extend(rows);
                }
                page_token = page_resp.page_token;
            } else {
                break;
            }
        }

        let total_rows = response
            .total_rows
            .as_deref()
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap_or(0);

        let rows = convert_rows(&schema, &all_rows);

        Ok(QueryResult {
            schema,
            rows,
            total_rows,
        })
    }

    async fn poll_results(
        &self,
        project: &str,
        job_id: &str,
        location: &str,
    ) -> Result<QueryResponse> {
        let url = format!("{BQ_BASE_URL}/projects/{project}/queries/{job_id}");
        let mut delay = std::time::Duration::from_millis(100);
        let max_delay = std::time::Duration::from_secs(5);

        loop {
            tokio::time::sleep(delay).await;
            let token = self.auth.token().await?;
            let resp: QueryResponse = self
                .http
                .get(&url)
                .bearer_auth(token.as_str())
                .query(&[
                    ("location", location),
                    ("timeoutMs", "30000"),
                ])
                .send()
                .await?
                .json()
                .await?;

            if resp.job_complete == Some(true) {
                return Ok(resp);
            }

            delay = std::cmp::min(delay * 2, max_delay);
        }
    }
}

fn convert_rows(
    schema: &TableSchema,
    rows: &[TableRow],
) -> Vec<serde_json::Map<String, serde_json::Value>> {
    rows.iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, field) in schema.fields.iter().enumerate() {
                let value = row
                    .f
                    .get(i)
                    .and_then(|cell| cell.v.clone())
                    .unwrap_or(serde_json::Value::Null);
                let value = coerce_value(&field.field_type, value);
                map.insert(field.name.clone(), value);
            }
            map
        })
        .collect()
}

/// Convert BigQuery REST API values to more useful representations.
/// TIMESTAMP comes as epoch seconds (float); convert to ISO 8601.
fn coerce_value(field_type: &str, value: serde_json::Value) -> serde_json::Value {
    match field_type {
        "TIMESTAMP" => {
            if let Some(s) = value.as_str() {
                if let Ok(epoch) = s.parse::<f64>() {
                    let secs = epoch as i64;
                    let nanos = ((epoch - secs as f64) * 1_000_000_000.0) as u32;
                    if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
                        return serde_json::Value::String(
                            dt.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string(),
                        );
                    }
                }
            }
            value
        }
        _ => value,
    }
}
