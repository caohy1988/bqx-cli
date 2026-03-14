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

    // Merge tables and views into a single list of table refs
    let mut all_refs_str = tables;
    if let Some(v) = views {
        all_refs_str.extend(v);
    }
    let table_refs = parse_table_refs(&all_refs_str)?;

    let vqs = verified_queries::load(verified_queries_path.as_deref())?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved);
    let resp = client
        .create_agent(
            &config.project_id,
            &config.location,
            &name,
            Some(&name),
            &table_refs,
            instructions.as_deref(),
            &vqs,
        )
        .await?;

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
