use anyhow::Result;
use serde::Serialize;

use crate::auth::{self, AuthOptions};
use crate::ca::client::{parse_table_refs, CaAgentManager, CaClient, CreateAgentParams};
use crate::ca::models::CreateAgentResponse;
use crate::ca::verified_queries;
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

/// Validate create-agent inputs before any network call.
pub fn validate_inputs(
    name: &str,
    tables: &[String],
    verified_queries_path: Option<&str>,
) -> Result<()> {
    config::validate_agent_id(name)?;
    if tables.is_empty() {
        anyhow::bail!(
            "--tables is required: provide at least one table reference (project.dataset.table)"
        );
    }
    // Validate table refs parse correctly
    parse_table_refs(tables)?;
    // Validate verified queries file if provided
    if let Some(path) = verified_queries_path {
        verified_queries::load(Some(path))?;
    }
    Ok(())
}

#[derive(Serialize)]
struct DryRunOutput {
    dry_run: bool,
    url: String,
    method: String,
    body: serde_json::Value,
}

pub async fn run(
    name: String,
    tables: Vec<String>,
    views: Option<Vec<String>>,
    verified_queries_path: Option<String>,
    instructions: Option<String>,
    dry_run: bool,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    validate_inputs(&name, &tables, verified_queries_path.as_deref())?;

    let views_count = views.as_ref().map_or(0, |v| v.len());
    let mut all_refs_str = tables;
    if let Some(v) = views {
        all_refs_str.extend(v);
    }
    let table_refs = parse_table_refs(&all_refs_str)?;

    let vqs = verified_queries::load(verified_queries_path.as_deref())?;

    if dry_run {
        let params = CreateAgentParams {
            agent_id: &name,
            display_name: Some(&name),
            tables: &table_refs,
            views_count,
            instructions: instructions.as_deref(),
            verified_queries: &vqs,
        };
        return run_dry_run(&params, config);
    }

    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let params = CreateAgentParams {
        agent_id: &name,
        display_name: Some(&name),
        tables: &table_refs,
        views_count,
        instructions: instructions.as_deref(),
        verified_queries: &vqs,
    };
    let resp = client
        .create_agent(&config.project_id, &config.location, &params)
        .await?;

    if let Some(ref template) = config.sanitize_template {
        let json_val = serde_json::to_value(&resp)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, &config.format);
        }
    }

    render_response(&resp, &config.format)
}

fn run_dry_run(params: &CreateAgentParams<'_>, config: &Config) -> Result<()> {
    let url = format!(
        "https://datacatalog.googleapis.com/v1/projects/{}/locations/{}/dataAgents:createSync?dataAgentId={}",
        config.project_id, config.location, params.agent_id
    );

    let table_refs: Vec<serde_json::Value> = params
        .tables
        .iter()
        .map(|t| {
            serde_json::json!({
                "projectId": t.project_id,
                "datasetId": t.dataset_id,
                "tableId": t.table_id,
            })
        })
        .collect();

    let example_queries: Vec<serde_json::Value> = params
        .verified_queries
        .iter()
        .map(|vq| {
            serde_json::json!({
                "naturalLanguageQuestion": vq.question,
                "sqlQuery": vq.query,
            })
        })
        .collect();

    let mut published_context = serde_json::json!({
        "datasourceReferences": {
            "bq": {
                "tableReferences": table_refs,
            }
        },
        "exampleQueries": example_queries,
    });

    if let Some(instr) = params.instructions {
        published_context["systemInstruction"] = serde_json::json!(instr);
    }

    let mut body = serde_json::json!({
        "dataAnalyticsAgent": {
            "publishedContext": published_context,
        }
    });

    let display_name = params.display_name.unwrap_or(params.agent_id);
    body["displayName"] = serde_json::json!(display_name);

    let output = DryRunOutput {
        dry_run: true,
        url,
        method: "POST".into(),
        body,
    };
    crate::output::render(&output, &config.format)
}

pub async fn run_with_executor(
    executor: &dyn CaAgentManager,
    name: String,
    tables: Vec<String>,
    views: Option<Vec<String>>,
    verified_queries_path: Option<String>,
    instructions: Option<String>,
    config: &Config,
) -> Result<()> {
    validate_inputs(&name, &tables, verified_queries_path.as_deref())?;

    let views_count = views.as_ref().map_or(0, |v| v.len());
    let mut all_refs_str = tables;
    if let Some(v) = views {
        all_refs_str.extend(v);
    }
    let table_refs = parse_table_refs(&all_refs_str)?;

    let vqs = verified_queries::load(verified_queries_path.as_deref())?;

    let params = CreateAgentParams {
        agent_id: &name,
        display_name: Some(&name),
        tables: &table_refs,
        views_count,
        instructions: instructions.as_deref(),
        verified_queries: &vqs,
    };
    let resp = executor
        .create_agent(&config.project_id, &config.location, &params)
        .await?;

    render_response(&resp, &config.format)
}

fn render_response(resp: &CreateAgentResponse, format: &OutputFormat) -> Result<()> {
    if *format == OutputFormat::Text {
        output::text::render_create_agent(resp);
        return Ok(());
    }
    output::render(resp, format)
}
