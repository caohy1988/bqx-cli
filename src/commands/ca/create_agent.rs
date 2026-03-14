use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::ca::client::{parse_table_refs, CaAgentManager, CaClient};
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

pub async fn run(
    name: String,
    tables: Vec<String>,
    views: Option<Vec<String>>,
    verified_queries_path: Option<String>,
    instructions: Option<String>,
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

    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let resp = client
        .create_agent(
            &config.project_id,
            &config.location,
            &name,
            Some(&name),
            &table_refs,
            views_count,
            instructions.as_deref(),
            &vqs,
        )
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

    let resp = executor
        .create_agent(
            &config.project_id,
            &config.location,
            &name,
            Some(&name),
            &table_refs,
            views_count,
            instructions.as_deref(),
            &vqs,
        )
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
