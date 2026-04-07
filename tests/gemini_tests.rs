use dcx::integrations::gemini;

#[test]
fn bundled_manifest_loads_successfully() {
    let manifest = gemini::load_manifest().expect("Failed to load bundled manifest");
    assert_eq!(manifest.name, "dcx");
    assert!(!manifest.tools.is_empty());
}

#[test]
fn bundled_manifest_validates() {
    let manifest = gemini::load_manifest().unwrap();
    gemini::validate_manifest(&manifest).expect("Manifest validation failed");
}

#[test]
fn manifest_has_expected_tool_count() {
    let manifest = gemini::load_manifest().unwrap();
    assert_eq!(
        manifest.tools.len(),
        28,
        "Expected 28 tools in manifest, got {}",
        manifest.tools.len()
    );
}

#[test]
fn manifest_version_matches_cargo() {
    let manifest = gemini::load_manifest().unwrap();
    let cargo_version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        manifest.version, cargo_version,
        "Manifest version ({}) does not match Cargo.toml ({})",
        manifest.version, cargo_version
    );
}

#[test]
fn all_tool_names_use_dcx_prefix() {
    let manifest = gemini::load_manifest().unwrap();
    for tool in &manifest.tools {
        assert!(
            tool.name.starts_with("dcx_"),
            "Tool name '{}' should start with 'dcx_'",
            tool.name
        );
    }
}

#[test]
fn all_tool_commands_are_valid_dcx_invocations() {
    let manifest = gemini::load_manifest().unwrap();
    for tool in &manifest.tools {
        assert!(
            tool.command.starts_with("dcx "),
            "Tool '{}' command should start with 'dcx ': {}",
            tool.name,
            tool.command
        );
        assert!(
            tool.command.contains("--format"),
            "Tool '{}' command should include --format flag: {}",
            tool.name,
            tool.command
        );
    }
}

// ── Contract drift detection ──

#[test]
fn bundled_manifest_matches_generated_contract() {
    // Run `dcx meta gemini-tools --format json` and compare tool names + count
    // with the bundled manifest to catch drift.
    let output = std::process::Command::new(env!("CARGO"))
        .args(["build", "--quiet"])
        .output()
        .expect("Failed to build");
    assert!(output.status.success(), "Build failed");

    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| String::from("target"));
    let bin = format!("{target_dir}/debug/dcx");

    let output = std::process::Command::new(&bin)
        .args(["meta", "gemini-tools", "--format", "json"])
        .output()
        .expect("Failed to run dcx meta gemini-tools");
    assert!(output.status.success(), "meta gemini-tools failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let generated: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON from meta gemini-tools: {e}"));

    let bundled = gemini::load_manifest().unwrap();

    // Compare tool names in order.
    let generated_names: Vec<&str> = generated["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    let bundled_names: Vec<&str> = bundled.tools.iter().map(|t| t.name.as_str()).collect();

    assert_eq!(
        generated_names, bundled_names,
        "Bundled manifest tool names differ from generated contract.\n\
         Regenerate with: dcx meta gemini-tools --format json > extensions/gemini/manifest.json"
    );

    // Compare tool count.
    assert_eq!(
        generated["tools"].as_array().unwrap().len(),
        bundled.tools.len(),
        "Bundled manifest tool count differs from generated contract"
    );
}
