use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::cli::OutputFormat;
use crate::commands::common::{maybe_sanitize_and_render, resource_id};
use crate::sources::alloydb::client::{AlloyDbClient, HttpAlloyDbClient};
use crate::sources::alloydb::models::AlloyDbClustersCliResponse;

pub async fn run_list(
    project: &str,
    location: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    // AlloyDB uses region-granularity locations. The global --location default
    // "US" is a BigQuery convention; for AlloyDB inventory, override to "-"
    // (all locations) when the user hasn't set an explicit region.
    let effective_location = if location == "US" { "-" } else { location };

    let resolved = auth::resolve(auth_opts).await?;
    let token = resolved.token().await?;
    let client = HttpAlloyDbClient::new(token);
    let clusters = client.list_clusters(project, effective_location).await?;

    let response = AlloyDbClustersCliResponse {
        project: project.to_string(),
        location: effective_location.to_string(),
        clusters,
    };

    if *format == OutputFormat::Text && sanitize_template.is_none() {
        render_text(&response);
        return Ok(());
    }

    maybe_sanitize_and_render(&response, auth_opts, format, sanitize_template).await
}

fn render_text(response: &AlloyDbClustersCliResponse) {
    println!(
        "Project: {}  Location: {}",
        response.project, response.location
    );
    println!("Clusters: {}", response.clusters.len());
    println!();
    for c in &response.clusters {
        let id = resource_id(&c.name);
        // Extract location from the resource name
        let loc = c.name.split('/').nth(3).unwrap_or("-");
        let state = c.state.as_deref().unwrap_or("?");
        let version = c.database_version.as_deref().unwrap_or("-");
        let ctype = c.cluster_type.as_deref().unwrap_or("-");
        println!("  {id}  {loc}  {state}  {version}  {ctype}");
    }
}
