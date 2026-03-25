use std::process::Command;

fn cargo_bin() -> String {
    let output = Command::new(env!("CARGO"))
        .args(["build", "--quiet"])
        .output()
        .expect("Failed to build");
    assert!(output.status.success(), "Build failed");

    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| String::from("target"));
    format!("{target_dir}/debug/dcx")
}

fn run_dcx(args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .env_remove("DCX_TOKEN")
        .env_remove("DCX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .output()
        .expect("Failed to run dcx")
}

// ── profiles list ──

#[test]
fn profiles_list_json_returns_array() {
    let output = run_dcx(&["profiles", "list", "--format", "json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let profiles = parsed["profiles"].as_array().unwrap();
    assert!(
        !profiles.is_empty(),
        "Expected at least one profile from repo fixtures"
    );

    let first = &profiles[0];
    assert!(first["name"].is_string());
    assert!(first["source_type"].is_string());
    assert!(first["family"].is_string());
    assert!(first["project"].is_string());
    assert!(first["origin"].is_string());
}

#[test]
fn profiles_list_text_renders_table() {
    let output = run_dcx(&["profiles", "list", "--format", "text"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Name"));
    assert!(stdout.contains("Source Type"));
    assert!(stdout.contains("bigquery"));
}

#[test]
fn profiles_list_includes_all_fixture_profiles() {
    let output = run_dcx(&["profiles", "list", "--format", "json"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let profiles = parsed["profiles"].as_array().unwrap();

    let names: Vec<&str> = profiles
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"bigquery-demo"), "Missing bigquery-demo");
    assert!(names.contains(&"looker-sales"), "Missing looker-sales");
    assert!(
        names.contains(&"spanner-finance"),
        "Missing spanner-finance"
    );
    assert!(names.contains(&"alloydb-ops"), "Missing alloydb-ops");
}

// ── profiles show ──

#[test]
fn profiles_show_by_path_json() {
    let output = run_dcx(&[
        "profiles",
        "show",
        "--profile",
        "deploy/ca/profiles/bigquery-demo.yaml",
        "--format",
        "json",
    ]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["name"], "bigquery-demo");
    assert_eq!(parsed["source_type"], "bigquery");
    assert_eq!(parsed["project"], "my-project");
}

#[test]
fn profiles_show_by_path_text() {
    let output = run_dcx(&[
        "profiles",
        "show",
        "--profile",
        "deploy/ca/profiles/looker-sales.yaml",
        "--format",
        "text",
    ]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Profile: looker-sales"));
    assert!(stdout.contains("source_type: looker"));
    assert!(stdout.contains("instance_url:"));
}

#[test]
fn profiles_show_redacts_looker_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let profile_path = dir.path().join("creds.yaml");
    std::fs::write(
        &profile_path,
        r#"
name: creds-test
source_type: looker
project: my-project
looker_instance_url: https://looker.example.com
looker_explores:
  - sales/orders
looker_client_id: super-secret-id
looker_client_secret: super-secret-value
"#,
    )
    .unwrap();

    let output = run_dcx(&[
        "profiles",
        "show",
        "--profile",
        profile_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("super-secret-id"),
        "Client ID should be redacted"
    );
    assert!(
        !stdout.contains("super-secret-value"),
        "Client secret should be redacted"
    );
    assert!(stdout.contains("***REDACTED***"));
}

#[test]
fn profiles_show_missing_profile_fails() {
    let output = run_dcx(&["profiles", "show", "--profile", "nonexistent-profile-xyz"]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// ── profiles validate ──

#[test]
fn profiles_validate_valid_profile_json() {
    let output = run_dcx(&[
        "profiles",
        "validate",
        "--profile",
        "deploy/ca/profiles/spanner-finance.yaml",
        "--format",
        "json",
    ]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["valid"], true);
    assert_eq!(parsed["name"], "spanner-finance");
    assert_eq!(parsed["source_type"], "spanner");
}

#[test]
fn profiles_validate_valid_profile_text() {
    let output = run_dcx(&[
        "profiles",
        "validate",
        "--profile",
        "deploy/ca/profiles/alloydb-ops.yaml",
        "--format",
        "text",
    ]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OK"));
    assert!(stdout.contains("alloydb-ops"));
}

#[test]
fn profiles_validate_invalid_profile_fails() {
    let dir = tempfile::tempdir().unwrap();
    let profile_path = dir.path().join("bad.yaml");
    std::fs::write(
        &profile_path,
        r#"
name: bad-profile
source_type: cloud_sql
project: my-project
instance_id: my-instance
database_id: mydb
"#,
    )
    .unwrap();

    let output = run_dcx(&[
        "profiles",
        "validate",
        "--profile",
        profile_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["valid"], false);
    assert!(parsed["error"].as_str().unwrap().contains("db_type"));
}

// ── profiles help ──

#[test]
fn profiles_help_shows_subcommands() {
    let output = run_dcx(&["profiles", "--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"));
    assert!(stdout.contains("show"));
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("Manage and inspect source profiles"));
}

#[test]
fn top_level_help_includes_profiles() {
    let output = run_dcx(&["--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("profiles"),
        "Top-level --help should list profiles command"
    );
}
