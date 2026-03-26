use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::cli::OutputFormat;
use crate::commands::common::maybe_sanitize_and_render;
use crate::sources::cloudsql::client::{CloudSqlClient, HttpCloudSqlClient};
use crate::sources::cloudsql::models::CloudSqlDatabasesCliResponse;

pub async fn run_list(
    project: &str,
    instance: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let resolved = auth::resolve(auth_opts).await?;
    let token = resolved.token().await?;
    let client = HttpCloudSqlClient::new(token);
    let databases = client.list_databases(project, instance).await?;

    let response = CloudSqlDatabasesCliResponse {
        project: project.to_string(),
        instance: instance.to_string(),
        databases,
    };

    if *format == OutputFormat::Text && sanitize_template.is_none() {
        render_text(&response);
        return Ok(());
    }

    maybe_sanitize_and_render(&response, auth_opts, format, sanitize_template).await
}

fn render_text(response: &CloudSqlDatabasesCliResponse) {
    println!(
        "Project: {}  Instance: {}",
        response.project, response.instance
    );
    println!("Databases: {}", response.databases.len());
    println!();
    for db in &response.databases {
        let name = db.name.as_deref().unwrap_or("?");
        let charset = db.charset.as_deref().unwrap_or("-");
        let collation = db.collation.as_deref().unwrap_or("-");
        println!("  {name}  {charset}  {collation}");
    }
}
