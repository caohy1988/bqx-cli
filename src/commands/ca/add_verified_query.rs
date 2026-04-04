use anyhow::Result;
use serde::Serialize;

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

#[derive(Serialize)]
struct DryRunOutput {
    dry_run: bool,
    steps: Vec<DryRunStep>,
}

#[derive(Serialize)]
struct DryRunStep {
    method: String,
    url: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<serde_json::Value>,
}

pub async fn run(
    agent: String,
    question: String,
    query: String,
    dry_run: bool,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    validate_inputs(&agent, &question, &query)?;

    if dry_run {
        return run_dry_run(&agent, &question, &query, config);
    }

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

fn run_dry_run(agent: &str, question: &str, query: &str, config: &Config) -> Result<()> {
    let agent_name = format!(
        "projects/{}/locations/{}/dataAgents/{agent}",
        config.project_id, config.location
    );
    let base = "https://datacatalog.googleapis.com/v1";

    // The PATCH body includes the full exampleQueries array (existing + new).
    // At dry-run time we cannot resolve the existing queries without a network
    // call, so we show the new entry that will be appended and note the
    // dependency on the live GET result.
    let new_query = serde_json::json!({
        "naturalLanguageQuestion": question,
        "sqlQuery": query,
    });

    let patch_body = serde_json::json!({
        "name": agent_name,
        "dataAnalyticsAgent": {
            "publishedContext": {
                "exampleQueries": {
                    "_note": "Array will contain existing queries (from GET) plus the entry below",
                    "appended_entry": new_query,
                }
            }
        }
    });

    let output = DryRunOutput {
        dry_run: true,
        steps: vec![
            DryRunStep {
                method: "GET".into(),
                url: format!("{base}/{agent_name}"),
                description: "Fetch current agent to read existing exampleQueries".into(),
                body: None,
            },
            DryRunStep {
                method: "PATCH".into(),
                url: format!("{base}/{agent_name}:updateSync?updateMask=dataAnalyticsAgent.publishedContext.exampleQueries"),
                description: "Update agent with existing queries plus appended entry".into(),
                body: Some(patch_body),
            },
        ],
    };
    crate::output::render(&output, &config.format)
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
