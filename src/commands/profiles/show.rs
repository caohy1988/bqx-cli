use anyhow::Result;

use crate::ca::profiles;
use crate::cli::OutputFormat;
use crate::output;

pub fn run(profile_ref: &str, format: &OutputFormat) -> Result<()> {
    let profile = profiles::resolve_profile(profile_ref)?;
    let redacted = profiles::redact_profile(&profile);

    if *format == OutputFormat::Text {
        render_text(&redacted);
        return Ok(());
    }

    output::render(&redacted, format)
}

fn render_text(profile: &profiles::CaProfile) {
    println!("Profile: {}", profile.name);
    println!("  source_type: {}", profile.source_type);
    println!("  family:      {:?}", profile.source_type.family());
    println!("  project:     {}", profile.project);

    if let Some(ref loc) = profile.location {
        println!("  location:    {}", loc);
    }

    match profile.source_type {
        profiles::SourceType::BigQuery => {
            if let Some(ref agent) = profile.agent {
                println!("  agent:       {}", agent);
            }
            if let Some(ref tables) = profile.tables {
                println!("  tables:      {}", tables.join(", "));
            }
        }
        profiles::SourceType::Looker => {
            if let Some(ref url) = profile.looker_instance_url {
                println!("  instance_url: {}", url);
            }
            if let Some(ref explores) = profile.looker_explores {
                println!("  explores:    {}", explores.join(", "));
            }
            if profile.looker_client_id.is_some() {
                println!("  client_id:   ***REDACTED***");
                println!("  client_secret: ***REDACTED***");
            }
        }
        profiles::SourceType::LookerStudio => {
            if let Some(ref ds) = profile.studio_datasource_id {
                println!("  datasource:  {}", ds);
            }
        }
        profiles::SourceType::AlloyDb => {
            if let Some(ref cid) = profile.cluster_id {
                println!("  cluster_id:  {}", cid);
            }
            if let Some(ref iid) = profile.instance_id {
                println!("  instance_id: {}", iid);
            }
            if let Some(ref did) = profile.database_id {
                println!("  database_id: {}", did);
            }
            if let Some(ref ctx) = profile.context_set_id {
                println!("  context_set: {}", ctx);
            }
        }
        profiles::SourceType::Spanner => {
            if let Some(ref iid) = profile.instance_id {
                println!("  instance_id: {}", iid);
            }
            if let Some(ref did) = profile.database_id {
                println!("  database_id: {}", did);
            }
            if let Some(ref ctx) = profile.context_set_id {
                println!("  context_set: {}", ctx);
            }
        }
        profiles::SourceType::CloudSql => {
            if let Some(ref iid) = profile.instance_id {
                println!("  instance_id: {}", iid);
            }
            if let Some(ref did) = profile.database_id {
                println!("  database_id: {}", did);
            }
            if let Some(ref dt) = profile.db_type {
                println!("  db_type:     {}", dt);
            }
            if let Some(ref ctx) = profile.context_set_id {
                println!("  context_set: {}", ctx);
            }
        }
    }
}
