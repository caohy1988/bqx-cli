use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::ca::client::{CaClient, CaExecutor, parse_table_refs};
use crate::ca::models::{CaQuestionRequest, CaQuestionResponse};
use crate::cli::OutputFormat;
use crate::config::{self, Config};
use crate::output;

/// Build the CA question request from CLI args.
pub fn build_request(
    question: String,
    agent: Option<String>,
    tables: Option<Vec<String>>,
    location: &str,
) -> Result<CaQuestionRequest> {
    let parsed_tables = match tables {
        Some(ref t) => Some(parse_table_refs(t)?),
        None => None,
    };

    Ok(CaQuestionRequest {
        question,
        agent,
        tables: parsed_tables,
        location: location.to_string(),
    })
}

/// Validate CA ask inputs before making any network call.
pub fn validate_inputs(
    question: &str,
    agent: Option<&str>,
    tables: Option<&[String]>,
) -> Result<()> {
    if question.trim().is_empty() {
        anyhow::bail!("Question cannot be empty");
    }
    if let Some(agent) = agent {
        config::validate_agent_id(agent)?;
    }
    if agent.is_some() && tables.is_some() {
        anyhow::bail!(
            "--agent and --tables cannot be used together. \
             Use --agent for a data agent context or --tables for inline table context, not both."
        );
    }
    Ok(())
}

// ── Entry points ──

pub async fn run(
    question: String,
    agent: Option<String>,
    tables: Option<Vec<String>>,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    validate_inputs(&question, agent.as_deref(), tables.as_deref())?;
    let req = build_request(question, agent, tables, &config.location)?;

    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let resp = client.ask(&config.project_id, &req).await?;

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
    executor: &dyn CaExecutor,
    question: String,
    agent: Option<String>,
    tables: Option<Vec<String>>,
    location: &str,
    config: &Config,
) -> Result<()> {
    validate_inputs(&question, agent.as_deref(), tables.as_deref())?;
    let req = build_request(question, agent, tables, location)?;
    let resp = executor.ask(&config.project_id, &req).await?;
    render_response(&resp, &config.format)
}

fn render_response(resp: &CaQuestionResponse, format: &OutputFormat) -> Result<()> {
    if *format == OutputFormat::Text {
        output::text::render_ca_ask(resp);
        return Ok(());
    }
    output::render(resp, format)
}
