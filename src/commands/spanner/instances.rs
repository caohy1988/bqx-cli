use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::cli::OutputFormat;
use crate::commands::common::{maybe_sanitize_and_render, resource_id};
use crate::sources::spanner::client::{HttpSpannerClient, SpannerClient};
use crate::sources::spanner::models::SpannerInstancesCliResponse;

pub async fn run_list(
    project: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let resolved = auth::resolve(auth_opts).await?;
    let token = resolved.token().await?;
    let client = HttpSpannerClient::new(token);
    let instances = client.list_instances(project).await?;

    let response = SpannerInstancesCliResponse {
        project: project.to_string(),
        instances,
    };

    if *format == OutputFormat::Text && sanitize_template.is_none() {
        render_text(&response);
        return Ok(());
    }

    maybe_sanitize_and_render(&response, auth_opts, format, sanitize_template).await
}

fn render_text(response: &SpannerInstancesCliResponse) {
    println!("Project: {}", response.project);
    println!("Instances: {}", response.instances.len());
    println!();
    for inst in &response.instances {
        let id = resource_id(&inst.name);
        let display = inst.display_name.as_deref().unwrap_or("-");
        let state = inst.state.as_deref().unwrap_or("?");
        let config = inst.config.as_deref().map(resource_id).unwrap_or("-");
        let capacity = if let Some(nodes) = inst.node_count.filter(|&n| n > 0) {
            format!("{nodes} nodes")
        } else if let Some(pu) = inst.processing_units.filter(|&p| p > 0) {
            format!("{pu} PU")
        } else {
            "-".to_string()
        };
        println!("  {id}  ({state})  {config}  {capacity}  {display}");
    }
}
