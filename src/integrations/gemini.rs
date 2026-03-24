//! Gemini CLI extension manifest generation.
//!
//! Generates and validates the extension manifest used by
//! `gemini extensions install` to register dcx tools.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Bundled manifest shipped with the binary.
const BUNDLED_MANIFEST: &str = include_str!("../../extensions/gemini/manifest.json");

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub homepage: String,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub command: String,
}

/// Load and parse the bundled Gemini extension manifest.
pub fn load_manifest() -> Result<ExtensionManifest> {
    let manifest: ExtensionManifest = serde_json::from_str(BUNDLED_MANIFEST)?;
    Ok(manifest)
}

/// Validate the manifest structure: all tools have names, descriptions,
/// and non-empty command strings.
pub fn validate_manifest(manifest: &ExtensionManifest) -> Result<()> {
    if manifest.name.is_empty() {
        anyhow::bail!("Extension manifest is missing 'name'");
    }
    if manifest.tools.is_empty() {
        anyhow::bail!("Extension manifest has no tools");
    }
    for tool in &manifest.tools {
        if tool.name.is_empty() {
            anyhow::bail!("Tool is missing 'name'");
        }
        if tool.description.is_empty() {
            anyhow::bail!("Tool '{}' is missing 'description'", tool.name);
        }
        if tool.command.is_empty() {
            anyhow::bail!("Tool '{}' is missing 'command'", tool.name);
        }
        if !tool.command.starts_with("dcx ") {
            anyhow::bail!(
                "Tool '{}' command does not start with 'dcx': {}",
                tool.name,
                tool.command
            );
        }
    }
    Ok(())
}

/// Print the manifest as pretty-printed JSON (for debugging / inspection).
pub fn print_manifest() -> Result<()> {
    let manifest = load_manifest()?;
    validate_manifest(&manifest)?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_loads_and_validates() {
        let manifest = load_manifest().expect("Failed to load bundled manifest");
        validate_manifest(&manifest).expect("Manifest validation failed");
    }

    #[test]
    fn manifest_has_expected_tools() {
        let manifest = load_manifest().unwrap();

        let tool_names: Vec<&str> = manifest.tools.iter().map(|t| t.name.as_str()).collect();

        // Core tools expected in the Phase 2 curated subset.
        assert!(
            tool_names.contains(&"dcx_jobs_query"),
            "Missing dcx_jobs_query"
        );
        assert!(
            tool_names.contains(&"dcx_datasets_list"),
            "Missing dcx_datasets_list"
        );
        assert!(
            tool_names.contains(&"dcx_tables_list"),
            "Missing dcx_tables_list"
        );
        assert!(
            tool_names.contains(&"dcx_analytics_doctor"),
            "Missing dcx_analytics_doctor"
        );
    }

    #[test]
    fn all_commands_start_with_dcx() {
        let manifest = load_manifest().unwrap();
        for tool in &manifest.tools {
            assert!(
                tool.command.starts_with("dcx "),
                "Tool '{}' command doesn't start with 'dcx': {}",
                tool.name,
                tool.command
            );
        }
    }

    #[test]
    fn all_commands_use_json_format() {
        let manifest = load_manifest().unwrap();
        for tool in &manifest.tools {
            assert!(
                tool.command.contains("--format"),
                "Tool '{}' command should include --format flag: {}",
                tool.name,
                tool.command
            );
        }
    }
}
