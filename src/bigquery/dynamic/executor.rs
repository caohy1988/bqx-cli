use std::collections::HashMap;

use anyhow::{bail, Result};
use serde_json::json;

use crate::auth::{self, AuthOptions, ResolvedAuth};
use crate::cli::OutputFormat;

use super::model::GeneratedCommand;
use super::request_builder::{self, DynamicRequest};
use super::service::ServiceConfig;

/// Execute a dynamic (generated) API command end-to-end.
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
    config: &ServiceConfig,
) -> Result<()> {
    let global_params = config.global_param_names();

    // Validate required params before any network/auth.
    if let Err(msg) = super::clap_tree::validate_required_params(args, cmd, &global_params) {
        bail!("{msg}");
    }

    let project_id_params: Vec<&str> = config
        .global_params
        .iter()
        .filter(|(_, cli_flag)| *cli_flag == "project_id")
        .map(|(api_name, _)| *api_name)
        .collect();

    let request = request_builder::build_request(
        base_url,
        &cmd.method,
        project_id,
        args,
        &project_id_params,
    )?;

    if dry_run {
        return render_dry_run(&request, format);
    }

    let resolved = auth::resolve(auth_opts).await?;
    let body = send_request(&resolved, &request, config.service_label).await?;

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

async fn send_request(
    auth: &ResolvedAuth,
    request: &DynamicRequest,
    service_label: &str,
) -> Result<serde_json::Value> {
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
        // Try to extract structured error message.
        if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(msg) = err_json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                bail!("{service_label} API error {}: {}", status.as_u16(), msg);
            }
        }
        bail!("{service_label} API error {}: {}", status.as_u16(), body);
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
            if let Some(items) = find_list_items(body) {
                render_items_as_table(items)?;
            } else {
                render_object_as_table(body)?;
            }
        }
    }
    Ok(())
}

/// Find a list-like array in the response for table rendering.
///
/// Dynamically scans top-level fields for the first array of objects,
/// skipping known metadata fields.
fn find_list_items(body: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    let obj = body.as_object()?;
    const SKIP_FIELDS: &[&str] = &[
        "kind",
        "etag",
        "nextPageToken",
        "totalItems",
        "selfLink",
        "unreachable",
    ];
    for (key, value) in obj {
        if SKIP_FIELDS.contains(&key.as_str()) {
            continue;
        }
        if let Some(arr) = value.as_array() {
            if arr.is_empty() || arr[0].is_object() {
                return Some(arr);
            }
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

    // Flatten one level of *Reference objects and collect columns from all items.
    let flattened: Vec<serde_json::Map<String, serde_json::Value>> =
        items.iter().map(flatten_references).collect();

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
