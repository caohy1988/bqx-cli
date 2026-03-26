use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::cli::OutputFormat;
use crate::commands::common::{maybe_sanitize_and_render, resource_id};
use crate::sources::alloydb::client::{AlloyDbClient, HttpAlloyDbClient};
use crate::sources::alloydb::models::AlloyDbInstancesCliResponse;

pub async fn run_list(
    project: &str,
    location: &str,
    cluster: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let effective_location = if location == "US" { "-" } else { location };

    let resolved = auth::resolve(auth_opts).await?;
    let token = resolved.token().await?;
    let client = HttpAlloyDbClient::new(token);
    let instances = client
        .list_instances(project, effective_location, cluster)
        .await?;

    let response = AlloyDbInstancesCliResponse {
        project: project.to_string(),
        location: effective_location.to_string(),
        cluster: cluster.to_string(),
        instances,
    };

    if *format == OutputFormat::Text && sanitize_template.is_none() {
        render_text(&response);
        return Ok(());
    }

    maybe_sanitize_and_render(&response, auth_opts, format, sanitize_template).await
}

fn render_text(response: &AlloyDbInstancesCliResponse) {
    println!(
        "Project: {}  Cluster: {}",
        response.project, response.cluster
    );
    println!("Instances: {}", response.instances.len());
    println!();
    for inst in &response.instances {
        let id = resource_id(&inst.name);
        let state = inst.state.as_deref().unwrap_or("?");
        let itype = inst.instance_type.as_deref().unwrap_or("-");
        let cpus = inst
            .machine_config
            .as_ref()
            .and_then(|m| m.cpu_count)
            .map(|c| format!("{c} vCPU"))
            .unwrap_or_else(|| "-".to_string());
        let zone = inst.gce_zone.as_deref().unwrap_or("-");
        println!("  {id}  {state}  {itype}  {cpus}  {zone}");
    }
}
