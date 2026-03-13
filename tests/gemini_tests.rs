use bqx::integrations::gemini;

#[test]
fn bundled_manifest_loads_successfully() {
    let manifest = gemini::load_manifest().expect("Failed to load bundled manifest");
    assert_eq!(manifest.name, "bqx");
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
    // Phase 2 curated subset: 10 tools (query, datasets list/get,
    // tables list/get, routines list, models list, analytics doctor/evaluate/get-trace).
    assert_eq!(
        manifest.tools.len(),
        10,
        "Expected 10 tools in Phase 2 manifest, got {}",
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
fn all_tool_names_use_bqx_prefix() {
    let manifest = gemini::load_manifest().unwrap();
    for tool in &manifest.tools {
        assert!(
            tool.name.starts_with("bqx_"),
            "Tool name '{}' should start with 'bqx_'",
            tool.name
        );
    }
}

#[test]
fn all_tool_commands_are_valid_bqx_invocations() {
    let manifest = gemini::load_manifest().unwrap();
    for tool in &manifest.tools {
        assert!(
            tool.command.starts_with("bqx "),
            "Tool '{}' command should start with 'bqx ': {}",
            tool.name,
            tool.command
        );
        assert!(
            tool.command.contains("--format json"),
            "Tool '{}' command should include --format json: {}",
            tool.name,
            tool.command
        );
    }
}
