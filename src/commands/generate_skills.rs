use std::path::Path;

use anyhow::Result;
use serde_json::json;

use crate::bigquery::discovery::{self, DiscoverySource};
use crate::bigquery::dynamic::{model, service};
use crate::cli::OutputFormat;
use crate::commands::meta;
use crate::skills::generator;

/// Run the `generate-skills` command.
///
/// Generates SKILL.md and agents/openai.yaml for each resource group
/// from the BigQuery v2 Discovery Document. Flags and constraints are
/// sourced from the command contract (same data as `dcx meta describe`).
pub fn run(
    app: &clap::Command,
    output_dir: &str,
    filter: &[String],
    format: &OutputFormat,
) -> Result<()> {
    let cfg = service::bigquery();
    let doc = discovery::load(&DiscoverySource::Bundled)?;
    let methods = model::extract_methods(&doc, cfg.use_flat_path);
    let allowed = model::filter_allowed(&methods, cfg.allowed_methods);
    let commands: Vec<model::GeneratedCommand> =
        allowed.iter().map(model::to_generated_command).collect();

    let contracts = meta::collect_all(app);
    let skills = generator::generate_all(&commands, &contracts);
    let skills = generator::filter_skills(skills, filter);

    if skills.is_empty() {
        match format {
            OutputFormat::Json | OutputFormat::JsonMinified => {
                println!("{}", json!({"generated": [], "count": 0}));
            }
            OutputFormat::Table | OutputFormat::Text => {
                println!("No skills matched the filter.");
            }
        }
        return Ok(());
    }

    let written = generator::write_skills(Path::new(output_dir), &skills)?;

    match format {
        OutputFormat::Json | OutputFormat::JsonMinified => {
            let output = json!({
                "generated": written,
                "count": written.len(),
                "output_dir": output_dir,
            });
            if *format == OutputFormat::JsonMinified {
                println!("{}", serde_json::to_string(&output)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
        }
        OutputFormat::Table | OutputFormat::Text => {
            println!("Generated {} skill(s) in {}:", written.len(), output_dir);
            for name in &written {
                println!("  {name}/SKILL.md");
                println!("  {name}/agents/openai.yaml");
            }
        }
    }

    Ok(())
}
