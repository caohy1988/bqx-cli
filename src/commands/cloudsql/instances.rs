use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::cli::OutputFormat;
use crate::commands::common::maybe_sanitize_and_render;
use crate::sources::cloudsql::client::{CloudSqlClient, HttpCloudSqlClient};
use crate::sources::cloudsql::models::{
    CloudSqlInstanceGetCliResponse, CloudSqlInstancesCliResponse,
};

pub async fn run_list(
    project: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let resolved = auth::resolve(auth_opts).await?;
    let token = resolved.token().await?;
    let client = HttpCloudSqlClient::new(token);
    let instances = client.list_instances(project).await?;

    let response = CloudSqlInstancesCliResponse {
        project: project.to_string(),
        instances,
    };

    if *format == OutputFormat::Text && sanitize_template.is_none() {
        render_list_text(&response);
        return Ok(());
    }

    maybe_sanitize_and_render(&response, auth_opts, format, sanitize_template).await
}

pub async fn run_get(
    project: &str,
    instance: &str,
    auth_opts: &AuthOptions,
    format: &OutputFormat,
    sanitize_template: Option<&str>,
) -> Result<()> {
    let resolved = auth::resolve(auth_opts).await?;
    let token = resolved.token().await?;
    let client = HttpCloudSqlClient::new(token);
    let inst = client.get_instance(project, instance).await?;

    let response = CloudSqlInstanceGetCliResponse {
        project: project.to_string(),
        instance: inst,
    };

    if *format == OutputFormat::Text && sanitize_template.is_none() {
        render_get_text(&response);
        return Ok(());
    }

    maybe_sanitize_and_render(&response, auth_opts, format, sanitize_template).await
}

fn render_list_text(response: &CloudSqlInstancesCliResponse) {
    println!("Project: {}", response.project);
    println!("Instances: {}", response.instances.len());
    println!();
    for inst in &response.instances {
        let name = inst.name.as_deref().unwrap_or("?");
        let version = inst.database_version.as_deref().unwrap_or("-");
        let state = inst.state.as_deref().unwrap_or("?");
        let region = inst.region.as_deref().unwrap_or("-");
        let tier = inst
            .settings
            .as_ref()
            .and_then(|s| s.tier.as_deref())
            .unwrap_or("-");
        println!("  {name}  {version}  {state}  {region}  {tier}");
    }
}

fn render_get_text(response: &CloudSqlInstanceGetCliResponse) {
    let inst = &response.instance;
    let name = inst.name.as_deref().unwrap_or("?");
    println!("Instance: {name}");
    if let Some(ref ver) = inst.database_version {
        println!("  version:    {ver}");
    }
    if let Some(ref state) = inst.state {
        println!("  state:      {state}");
    }
    if let Some(ref region) = inst.region {
        println!("  region:     {region}");
    }
    if let Some(ref conn) = inst.connection_name {
        println!("  connection: {conn}");
    }
    if let Some(ref settings) = inst.settings {
        if let Some(ref tier) = settings.tier {
            println!("  tier:       {tier}");
        }
        if let Some(ref disk) = settings.data_disk_size_gb {
            println!("  disk:       {disk} GB");
        }
    }
    if let Some(ref ips) = inst.ip_addresses {
        for ip in ips {
            let ip_type = ip.ip_type.as_deref().unwrap_or("?");
            let addr = ip.ip_address.as_deref().unwrap_or("-");
            println!("  ip ({ip_type}): {addr}");
        }
    }
}
