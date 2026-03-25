use anyhow::{bail, Result};

use crate::auth::{self, AuthOptions};
use crate::ca::profiles::{self, SourceType};
use crate::cli::OutputFormat;
use crate::output;
use crate::sources::looker::client::HttpLookerClient;
use crate::sources::looker::client::LookerClient;
use crate::sources::looker::models::{ExploreGetResponse, ExploresListResponse};

/// List all explores visible to the resolved Looker profile.
pub async fn run_list(
    profile_ref: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
) -> Result<()> {
    let profile = profiles::resolve_profile(profile_ref)?;
    if profile.source_type != SourceType::Looker {
        bail!(
            "Profile '{}' is source_type '{}', expected 'looker'",
            profile.name,
            profile.source_type
        );
    }

    let token = resolve_looker_token(&profile, auth_opts).await?;
    let client = HttpLookerClient::new(token);
    let explores = client.list_explores(&profile).await?;

    let instance_url = profile.looker_instance_url.clone().unwrap_or_default();

    let response = ExploresListResponse {
        instance_url,
        explores,
    };

    if *format == OutputFormat::Text {
        render_explores_text(&response);
        return Ok(());
    }

    output::render(&response, format)
}

/// Get detailed metadata for a single explore.
pub async fn run_get(
    profile_ref: &str,
    explore_ref: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
) -> Result<()> {
    let profile = profiles::resolve_profile(profile_ref)?;
    if profile.source_type != SourceType::Looker {
        bail!(
            "Profile '{}' is source_type '{}', expected 'looker'",
            profile.name,
            profile.source_type
        );
    }

    let (model, explore) = profiles::parse_looker_explore(explore_ref)?;

    let token = resolve_looker_token(&profile, auth_opts).await?;
    let client = HttpLookerClient::new(token);
    let detail = client.get_explore(&profile, model, explore).await?;

    let instance_url = profile.looker_instance_url.clone().unwrap_or_default();

    let response = ExploreGetResponse {
        instance_url,
        explore: detail,
    };

    if *format == OutputFormat::Text {
        render_explore_detail_text(&response);
        return Ok(());
    }

    output::render(&response, format)
}

/// Resolve a bearer token for the Looker API.
///
/// If the profile has `looker_client_id` / `looker_client_secret`, use Looker
/// API key auth to get a token. Otherwise fall back to GCP auth (for Looker
/// instances that accept Google-issued tokens).
pub(crate) async fn resolve_looker_token(
    profile: &profiles::CaProfile,
    auth_opts: &AuthOptions,
) -> Result<String> {
    if let (Some(client_id), Some(client_secret)) = (
        profile.looker_client_id.as_deref(),
        profile.looker_client_secret.as_deref(),
    ) {
        let base = profile
            .looker_instance_url
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches('/');
        let url = format!("{base}/api/4.0/login");
        let http = reqwest::Client::new();
        let resp = http
            .post(&url)
            .form(&[("client_id", client_id), ("client_secret", client_secret)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Looker API login failed: {status} — {body}");
        }

        let data: serde_json::Value = resp.json().await?;
        let token = data["access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No access_token in Looker login response"))?;
        Ok(token.to_string())
    } else {
        // Fall back to GCP auth
        let resolved = auth::resolve(auth_opts).await?;
        resolved.token().await
    }
}

fn render_explores_text(response: &ExploresListResponse) {
    println!("Looker Instance: {}", response.instance_url);
    println!("Explores: {}", response.explores.len());
    println!();
    for e in &response.explores {
        let label = e.label.as_deref().unwrap_or("-");
        let desc = e.description.as_deref().unwrap_or("");
        println!("  {}/{}  ({})", e.model_name, e.name, label);
        if !desc.is_empty() {
            println!("    {}", desc);
        }
    }
}

fn render_explore_detail_text(response: &ExploreGetResponse) {
    let d = &response.explore;
    println!("Explore: {}/{}", d.model_name, d.name);
    if let Some(ref label) = d.label {
        println!("  label:       {}", label);
    }
    if let Some(ref desc) = d.description {
        println!("  description: {}", desc);
    }

    if let Some(ref fields) = d.fields {
        if let Some(ref dims) = fields.dimensions {
            println!("  dimensions:  {}", dims.len());
            for dim in dims {
                let ft = dim.field_type.as_deref().unwrap_or("?");
                println!("    {} ({})", dim.name, ft);
            }
        }
        if let Some(ref measures) = fields.measures {
            println!("  measures:    {}", measures.len());
            for m in measures {
                let ft = m.field_type.as_deref().unwrap_or("?");
                println!("    {} ({})", m.name, ft);
            }
        }
    }
}
