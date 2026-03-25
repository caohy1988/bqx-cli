use anyhow::{bail, Result};

use crate::auth::AuthOptions;
use crate::ca::profiles::{self, SourceType};
use crate::cli::OutputFormat;
use crate::output;
use crate::sources::looker::client::HttpLookerClient;
use crate::sources::looker::client::LookerClient;
use crate::sources::looker::models::{DashboardGetResponse, DashboardsListResponse};

/// List all dashboards visible to the resolved Looker profile.
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

    let token = super::explores::resolve_looker_token(&profile, auth_opts).await?;
    let client = HttpLookerClient::new(token);
    let dashboards = client.list_dashboards(&profile).await?;

    let instance_url = profile.looker_instance_url.clone().unwrap_or_default();

    let response = DashboardsListResponse {
        instance_url,
        dashboards,
    };

    if *format == OutputFormat::Text {
        render_dashboards_text(&response);
        return Ok(());
    }

    output::render(&response, format)
}

/// Get detailed metadata for a single dashboard.
pub async fn run_get(
    profile_ref: &str,
    dashboard_id: &str,
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

    let token = super::explores::resolve_looker_token(&profile, auth_opts).await?;
    let client = HttpLookerClient::new(token);
    let detail = client.get_dashboard(&profile, dashboard_id).await?;

    let instance_url = profile.looker_instance_url.clone().unwrap_or_default();

    let response = DashboardGetResponse {
        instance_url,
        dashboard: detail,
    };

    if *format == OutputFormat::Text {
        render_dashboard_detail_text(&response);
        return Ok(());
    }

    output::render(&response, format)
}

fn render_dashboards_text(response: &DashboardsListResponse) {
    println!("Looker Instance: {}", response.instance_url);
    println!("Dashboards: {}", response.dashboards.len());
    println!();
    for d in &response.dashboards {
        let id = d.id.as_deref().unwrap_or("?");
        let title = d.title.as_deref().unwrap_or("(untitled)");
        let folder = d
            .folder
            .as_ref()
            .and_then(|f| f.name.as_deref())
            .unwrap_or("-");
        let hidden = if d.hidden == Some(true) {
            " [hidden]"
        } else {
            ""
        };
        println!("  [{id}] {title}  (folder: {folder}){hidden}");
        if let Some(ref desc) = d.description {
            if !desc.is_empty() {
                println!("    {}", desc);
            }
        }
    }
}

fn render_dashboard_detail_text(response: &DashboardGetResponse) {
    let d = &response.dashboard;
    let id = d.id.as_deref().unwrap_or("?");
    let title = d.title.as_deref().unwrap_or("(untitled)");
    println!("Dashboard: [{id}] {title}");
    if let Some(ref desc) = d.description {
        println!("  description: {}", desc);
    }
    if let Some(ref folder) = d.folder {
        if let Some(ref name) = folder.name {
            println!("  folder:      {}", name);
        }
    }
    if let Some(ref elements) = d.dashboard_elements {
        println!("  elements:    {}", elements.len());
        for el in elements {
            let el_title = el.title.as_deref().unwrap_or("(untitled)");
            let el_type = el.element_type.as_deref().unwrap_or("?");
            let model = el.model.as_deref().unwrap_or("-");
            let explore = el.explore.as_deref().unwrap_or("-");
            println!("    {el_title} ({el_type}) → {model}/{explore}");
        }
    }
}
