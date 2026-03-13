use std::collections::HashMap;

use anyhow::{bail, Result};
use serde_json::json;

use crate::auth::{self, AuthOptions, ResolvedAuth};
use crate::cli::OutputFormat;

use super::model::GeneratedCommand;
use super::request_builder::{self, DynamicRequest};

/// Execute a dynamic (generated) BigQuery API command end-to-end.
///
/// 1. Validates required parameters
/// 2. In dry-run mode, prints the request and returns
/// 3. Resolves auth
/// 4. Sends the HTTP request
/// 5. Optionally sanitizes via Model Armor
/// 6. Renders the JSON response
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    cmd: &GeneratedCommand,
    args: &HashMap<String, String>,
    project_id: &str,
    base_url: &str,
    format: &OutputFormat,
    dry_run: bool,
    auth_opts: &AuthOptions,
    sanitize_template: Option<&str>,
) -> Result<()> {
    // Validate required params before any network/auth.
    if let Err(msg) = super::clap_tree::validate_required_params(args, cmd) {
        bail!("{msg}");
    }

    let request = request_builder::build_request(base_url, &cmd.method, project_id, args)?;

    if dry_run {
        return render_dry_run(&request, format);
    }

    let resolved = auth::resolve(auth_opts).await?;
    let body = send_request(&resolved, &request).await?;

    let body = if let Some(template) = sanitize_template {
        let result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &body).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&result);
        result.content
    } else {
        body
    };

    render_response(&body, format)
}

fn render_dry_run(request: &DynamicRequest, format: &OutputFormat) -> Result<()> {
    let mut url = request.url.clone();
    if !request.query_params.is_empty() {
        let qs: Vec<String> = request
            .query_params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        url = format!("{url}?{}", qs.join("&"));
    }

    let output = json!({
        "dry_run": true,
        "url": url,
        "method": request.http_method,
    });

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            println!("{} {}", request.http_method, url);
        }
    }
    Ok(())
}

async fn send_request(auth: &ResolvedAuth, request: &DynamicRequest) -> Result<serde_json::Value> {
    let token = auth.token().await?;
    let client = reqwest::Client::new();

    let mut builder = match request.http_method.as_str() {
        "GET" => client.get(&request.url),
        "POST" => client.post(&request.url),
        "PUT" => client.put(&request.url),
        "DELETE" => client.delete(&request.url),
        "PATCH" => client.patch(&request.url),
        other => bail!("Unsupported HTTP method: {other}"),
    };

    builder = builder.bearer_auth(&token);

    if !request.query_params.is_empty() {
        builder = builder.query(&request.query_params);
    }

    let resp = builder.send().await?;
    let status = resp.status();

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        // Try to extract structured error message from BigQuery.
        if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(msg) = err_json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                bail!("BigQuery API error {}: {}", status.as_u16(), msg);
            }
        }
        bail!("BigQuery API error {}: {}", status.as_u16(), body);
    }

    let body: serde_json::Value = resp.json().await?;
    Ok(body)
}

fn render_response(body: &serde_json::Value, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(body)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            // For table/text, flatten one level of common reference objects
            // and render as a table if the response contains a list.
            if let Some(items) = find_list_items(body) {
                render_items_as_table(items)?;
            } else {
                // Single object — render as key-value pairs.
                render_object_as_table(body)?;
            }
        }
    }
    Ok(())
}

/// Find a list-like array in the response for table rendering.
/// BigQuery list responses typically have a top-level array field
/// like "datasets", "tables", etc.
fn find_list_items(body: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    let obj = body.as_object()?;
    // Common list field names in BigQuery API responses.
    for key in ["datasets", "tables", "jobs", "routines", "models"] {
        if let Some(arr) = obj.get(key).and_then(|v| v.as_array()) {
            return Some(arr);
        }
    }
    None
}

fn render_items_as_table(items: &[serde_json::Value]) -> Result<()> {
    use comfy_table::presets::UTF8_FULL_CONDENSED;
    use comfy_table::Table;

    if items.is_empty() {
        println!("(no results)");
        return Ok(());
    }

    // Flatten one level of *Reference objects and collect columns from first item.
    let flattened: Vec<serde_json::Map<String, serde_json::Value>> =
        items.iter().map(flatten_references).collect();

    // Collect columns from all items to handle sparse data.
    let mut columns: Vec<String> = Vec::new();
    for item in &flattened {
        for key in item.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(&columns);

    for item in &flattened {
        let row: Vec<String> = columns
            .iter()
            .map(|col| format_cell(item.get(col)))
            .collect();
        table.add_row(row);
    }

    println!("{table}");
    Ok(())
}

fn render_object_as_table(body: &serde_json::Value) -> Result<()> {
    use comfy_table::presets::UTF8_FULL_CONDENSED;
    use comfy_table::Table;

    let flat = flatten_references(body);

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(["Field", "Value"]);

    let mut keys: Vec<&String> = flat.keys().collect();
    keys.sort();

    for key in keys {
        let val = format_cell(flat.get(key));
        table.add_row([key.as_str(), &val]);
    }

    println!("{table}");
    Ok(())
}

/// Flatten one level of `*Reference` objects (e.g. datasetReference, tableReference).
/// { "datasetReference": { "datasetId": "foo", "projectId": "bar" }, "kind": "..." }
/// becomes { "datasetId": "foo", "projectId": "bar", "kind": "..." }
fn flatten_references(value: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    let mut result = serde_json::Map::new();
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            if key.ends_with("Reference") {
                if let Some(inner) = val.as_object() {
                    for (inner_key, inner_val) in inner {
                        result.insert(inner_key.clone(), inner_val.clone());
                    }
                    continue;
                }
            }
            // Skip deeply nested objects/arrays for table rendering.
            if val.is_object() || val.is_array() {
                let summary = match val {
                    serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
                    serde_json::Value::Object(obj) => format!("{{{} fields}}", obj.len()),
                    _ => unreachable!(),
                };
                result.insert(key.clone(), serde_json::Value::String(summary));
            } else {
                result.insert(key.clone(), val.clone());
            }
        }
    }
    result
}

fn format_cell(value: Option<&serde_json::Value>) -> String {
    match value {
        None | Some(serde_json::Value::Null) => "-".to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(v) => v.to_string(),
    }
}
