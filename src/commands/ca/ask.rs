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
/// Resolution order:
/// 1. If the value looks like a file path (contains '/' or ends in .yaml/.yml),
///    load it from disk directly.
/// 2. Look up by name in ~/.config/bqx/profiles/ (user-local).
/// 3. Look up by name in deploy/ca/profiles/ (repo-local fallback).
fn resolve_profile(profile_ref: &str) -> Result<CaProfile> {
    let path = Path::new(profile_ref);
    if path.extension().is_some_and(|e| e == "yaml" || e == "yml") || profile_ref.contains('/') {
        return crate::ca::profiles::load_profile(path);
    }

    // 1. User-local profiles directory.
    if let Some(config_dir) = dirs_for_profile_lookup() {
        let profiles = crate::ca::profiles::load_profiles_from_dir(&config_dir)?;
        if let Some(p) = profiles.into_iter().find(|p| p.name == profile_ref) {
            return Ok(p);
        }
    }

    // 2. Repo-local fallback (deploy/ca/profiles/).
    let repo_dir = Path::new("deploy/ca/profiles");
    if repo_dir.exists() {
        let profiles = crate::ca::profiles::load_profiles_from_dir(repo_dir)?;
        if let Some(p) = profiles.into_iter().find(|p| p.name == profile_ref) {
            return Ok(p);
        }
    }

    bail!(
        "Profile '{}' not found. Looked in ~/.config/bqx/profiles/ and deploy/ca/profiles/. \
         You can also pass a path: --profile path/to/profile.yaml",
        profile_ref
    )
}

/// Returns the user-local profiles directory (~/.config/bqx/profiles/) if it exists.
fn dirs_for_profile_lookup() -> Option<std::path::PathBuf> {
    let dir = directories::ProjectDirs::from("", "", "bqx")?;
    let profiles_dir = dir.config_dir().join("profiles");
    if profiles_dir.exists() {
        Some(profiles_dir)
    } else {
        None
    }
}

// ── Entry points ──

/// Profile-based entry point. Called from main.rs before Config::from_cli()
/// so that --project-id is not required when the profile supplies it.
pub async fn run_profile(
    question: String,
    profile_ref: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    if question.trim().is_empty() {
        bail!("Question cannot be empty");
    }

    let profile = resolve_profile(profile_ref)?;
    run_with_profile(question, &profile, auth_opts, format, sanitize_template).await
}

/// Legacy entry point (no --profile). Uses Config which requires --project-id.
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

/// Run CA ask using a resolved profile.
async fn run_with_profile(
    question: String,
    profile: &CaProfile,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    match profile.source_type.family() {
        ProfileFamily::ChatDataAgent => {
            run_chat_profile(question, profile, auth_opts, format, sanitize_template).await
        }
        ProfileFamily::QueryData => {
            run_querydata_profile(question, profile, auth_opts, format, sanitize_template).await
        }
    }
}

/// Run CA ask for Chat/DataAgent family sources via profile.
async fn run_chat_profile(
    question: String,
    profile: &CaProfile,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    match profile.source_type {
        SourceType::BigQuery => {
            let location = profile.location.as_deref().unwrap_or("US");
            let req = build_request(
                question,
                profile.agent.clone(),
                profile.tables.clone(),
                location,
            )?;

            let resolved = auth::resolve(auth_opts).await?;
            let client = CaClient::new(resolved.clone());
            let resp = client.ask(&profile.project, &req).await?;

            if let Some(template) = sanitize_template {
                let json_val = serde_json::to_value(&resp)?;
                let sanitize_result =
                    crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val)
                        .await?;
                crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
                if sanitize_result.sanitized {
                    return crate::output::render(&sanitize_result.content, format);
                }
            }

            render_response(&resp, format)
        }
        SourceType::Looker => {
            let resolved = auth::resolve(auth_opts).await?;
            let client = CaClient::new(resolved.clone());
            let resp = client.ask_looker(profile, &question).await?;

            if let Some(template) = sanitize_template {
                let json_val = serde_json::to_value(&resp)?;
                let sanitize_result =
                    crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val)
                        .await?;
                crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
                if sanitize_result.sanitized {
                    return crate::output::render(&sanitize_result.content, format);
                }
            }

            render_response(&resp, format)
        }
        SourceType::LookerStudio => {
            let resolved = auth::resolve(auth_opts).await?;
            let client = CaClient::new(resolved.clone());
            let resp = client.ask_studio(profile, &question).await?;

            if let Some(template) = sanitize_template {
                let json_val = serde_json::to_value(&resp)?;
                let sanitize_result =
                    crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val)
                        .await?;
                crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
                if sanitize_result.sanitized {
                    return crate::output::render(&sanitize_result.content, format);
                }
            }

            render_response(&resp, format)
        }
        _ => unreachable!("Non-ChatDataAgent source in run_chat_profile"),
    }
}

/// Run CA ask for QueryData family sources (AlloyDB, Spanner, Cloud SQL) via profile.
async fn run_querydata_profile(
    question: String,
    profile: &CaProfile,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved.clone());
    let resp = client.ask_querydata(profile, &question).await?;

    if let Some(template) = sanitize_template {
        let json_val = serde_json::to_value(&resp)?;
        let sanitize_result =
            crate::bigquery::sanitize::sanitize_response(&resolved, template, &json_val).await?;
        crate::bigquery::sanitize::print_sanitization_notice(&sanitize_result);
        if sanitize_result.sanitized {
            return crate::output::render(&sanitize_result.content, format);
        }
    }

    render_response(&resp, format)
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
