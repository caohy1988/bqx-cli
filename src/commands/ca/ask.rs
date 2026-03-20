use std::path::Path;

use anyhow::{bail, Result};

use crate::auth::{self, AuthOptions};
use crate::ca::client::{parse_table_refs, CaClient, CaExecutor};
use crate::ca::models::{CaQuestionRequest, CaQuestionResponse};
use crate::ca::profiles::{CaProfile, ProfileFamily, SourceType};
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

/// Resolve a --profile value to a CaProfile.
///
/// If the value looks like a file path (contains '/' or '.yaml'/'.yml'),
/// load it from disk. Otherwise, look it up by name in the default
/// profiles directory (deploy/ca/profiles/).
fn resolve_profile(profile_ref: &str) -> Result<CaProfile> {
    let path = Path::new(profile_ref);
    if path.extension().map_or(false, |e| e == "yaml" || e == "yml") || profile_ref.contains('/') {
        return crate::ca::profiles::load_profile(path);
    }

    // Look up by name in the default profiles directory.
    let profiles_dir = Path::new("deploy/ca/profiles");
    if !profiles_dir.exists() {
        bail!(
            "Profile '{}' not found. No profiles directory at {}",
            profile_ref,
            profiles_dir.display()
        );
    }

    let profiles = crate::ca::profiles::load_profiles_from_dir(profiles_dir)?;
    profiles
        .into_iter()
        .find(|p| p.name == profile_ref)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found in {}", profile_ref, profiles_dir.display()))
}

// ── Entry points ──

pub async fn run(
    question: String,
    profile: Option<String>,
    agent: Option<String>,
    tables: Option<Vec<String>>,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    // If a profile is specified, extract source context from it.
    if let Some(ref profile_ref) = profile {
        let ca_profile = resolve_profile(profile_ref)?;
        return run_with_profile(question, &ca_profile, auth_opts, config).await;
    }

    // Legacy path: direct --agent / --tables flags (BigQuery only).
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

/// Run CA ask using a resolved profile.
async fn run_with_profile(
    question: String,
    profile: &CaProfile,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    match profile.source_type.family() {
        ProfileFamily::ChatDataAgent => {
            run_chat_profile(question, profile, auth_opts, config).await
        }
        ProfileFamily::QueryData => {
            bail!(
                "QueryData sources ({}) are not yet supported. \
                 This will be implemented in Phase 4 Milestone 3.",
                profile.source_type
            );
        }
    }
}

/// Run CA ask for Chat/DataAgent family sources via profile.
async fn run_chat_profile(
    question: String,
    profile: &CaProfile,
    auth_opts: &AuthOptions,
    config: &Config,
) -> Result<()> {
    match profile.source_type {
        SourceType::BigQuery => {
            // BigQuery profile: extract agent/tables and use existing path.
            let location = profile
                .location
                .as_deref()
                .unwrap_or(&config.location);
            let req = build_request(
                question,
                profile.agent.clone(),
                profile.tables.clone(),
                location,
            )?;

            let resolved = auth::resolve(auth_opts).await?;
            let client = CaClient::new(resolved.clone());
            let project = &profile.project;
            let resp = client.ask(project, &req).await?;

            if let Some(ref template) = config.sanitize_template {
                let json_val = serde_json::to_value(&resp)?;
                let sanitize_result =
                    crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val)
                        .await?;
                crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
                if sanitize_result.sanitized {
                    return crate::output::render(&sanitize_result.content, &config.format);
                }
            }

            render_response(&resp, &config.format)
        }
        SourceType::Looker | SourceType::LookerStudio => {
            bail!(
                "Looker/Looker Studio sources are not yet supported. \
                 This will be implemented in Phase 4 Milestone 2."
            );
        }
        _ => unreachable!("Non-ChatDataAgent source in run_chat_profile"),
    }
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
