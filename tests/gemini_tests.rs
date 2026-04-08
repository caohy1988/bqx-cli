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
    // Run `dcx meta gemini-tools --format json` and compare full tool objects
    // with the bundled manifest to catch any drift in names, parameters, or
    // command templates.
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

    // Parse the bundled manifest as raw JSON for field-level comparison.
    let bundled_json: serde_json::Value =
        serde_json::from_str(include_str!("../extensions/gemini/manifest.json"))
            .expect("Failed to parse bundled manifest");

    let gen_tools = generated["tools"].as_array().unwrap();
    let bun_tools = bundled_json["tools"].as_array().unwrap();

    assert_eq!(
        gen_tools.len(),
        bun_tools.len(),
        "Tool count differs. Regenerate with:\n  \
         dcx meta gemini-tools --format json > extensions/gemini/manifest.json"
    );

    // Compare each tool: name, parameters, and command template.
    for (i, (gen, bun)) in gen_tools.iter().zip(bun_tools.iter()).enumerate() {
        assert_eq!(
            gen["name"], bun["name"],
            "Tool #{i} name differs.\n  generated: {}\n  bundled:   {}\n\
             Regenerate with: dcx meta gemini-tools --format json > extensions/gemini/manifest.json",
            gen["name"], bun["name"]
        );
        assert_eq!(
            gen["parameters"], bun["parameters"],
            "Tool '{}' parameters differ.\n  generated: {}\n  bundled:   {}\n\
             Regenerate with: dcx meta gemini-tools --format json > extensions/gemini/manifest.json",
            gen["name"],
            serde_json::to_string_pretty(&gen["parameters"]).unwrap(),
            serde_json::to_string_pretty(&bun["parameters"]).unwrap()
        );
        assert_eq!(
            gen["command"], bun["command"],
            "Tool '{}' command differs.\n  generated: {}\n  bundled:   {}\n\
             Regenerate with: dcx meta gemini-tools --format json > extensions/gemini/manifest.json",
            gen["name"], gen["command"], bun["command"]
        );
    }
}
