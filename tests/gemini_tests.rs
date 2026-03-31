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
    // Phase 5: 28 tools (Phase 4's 17 + 11 Phase 5 tools).
    assert_eq!(
        manifest.tools.len(),
        28,
        "Expected 28 tools in Phase 5 manifest, got {}",
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
