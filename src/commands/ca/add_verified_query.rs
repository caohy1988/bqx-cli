use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::ca::client::{CaAgentManager, CaClient};
use crate::ca::models::AddVerifiedQueryResponse;
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

/// Validate add-verified-query inputs before any network call.
pub fn validate_inputs(agent: &str, question: &str, query: &str) -> Result<()> {
    config::validate_agent_id(agent)?;
    if question.trim().is_empty() {
        anyhow::bail!("--question cannot be empty");
    }
    if query.trim().is_empty() {
        anyhow::bail!("--query cannot be empty");
    }
    Ok(())
}

pub async fn run(
    agent: String,
    question: String,
    query: String,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    validate_inputs(&agent, &question, &query)?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let resp = client
        .add_verified_query(
            &config.project_id,
            &config.location,
            &agent,
            &question,
            &query,
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
    agent: String,
    question: String,
    query: String,
    config: &Config,
) -> Result<()> {
    validate_inputs(&agent, &question, &query)?;

    let resp = executor
        .add_verified_query(
            &config.project_id,
            &config.location,
            &agent,
            &question,
            &query,
        )
        .await?;

    render_response(&resp, &config.format)
}

fn render_response(resp: &AddVerifiedQueryResponse, format: &OutputFormat) -> Result<()> {
    if *format == OutputFormat::Text {
        output::text::render_add_verified_query(resp);
        return Ok(());
    }
    output::render(resp, format)
}
