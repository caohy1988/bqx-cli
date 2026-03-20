use std::path::Path;

use bqx::ca::profiles::{self, parse_looker_explore, ProfileFamily, SourceType};

#[test]
fn load_bigquery_demo_profile() {
    let p = profiles::load_profile(Path::new("deploy/ca/profiles/bigquery-demo.yaml")).unwrap();
    assert_eq!(p.name, "bigquery-demo");
    assert_eq!(p.source_type, SourceType::BigQuery);
    assert_eq!(p.project, "my-project");
    assert_eq!(p.location.as_deref(), Some("US"));
    assert_eq!(p.agent.as_deref(), Some("agent-analytics"));
    assert_eq!(p.source_type.family(), ProfileFamily::ChatDataAgent);
}

#[test]
fn load_looker_sales_profile() {
    let p = profiles::load_profile(Path::new("deploy/ca/profiles/looker-sales.yaml")).unwrap();
    assert_eq!(p.name, "looker-sales");
    assert_eq!(p.source_type, SourceType::Looker);
    assert_eq!(
        p.looker_instance_url.as_deref(),
        Some("https://looker.example.com")
    );
    assert_eq!(p.looker_explores.as_ref().unwrap().len(), 2);
    assert_eq!(p.source_type.family(), ProfileFamily::ChatDataAgent);

    // Verify explore parsing
    let explores = p.looker_explores.as_ref().unwrap();
    let (model, explore) = parse_looker_explore(&explores[0]).unwrap();
    assert_eq!(model, "sales_model");
    assert_eq!(explore, "orders");
}

#[test]
fn load_alloydb_ops_profile() {
    let p = profiles::load_profile(Path::new("deploy/ca/profiles/alloydb-ops.yaml")).unwrap();
    assert_eq!(p.name, "alloydb-ops");
    assert_eq!(p.source_type, SourceType::AlloyDb);
    assert_eq!(p.context_set_id.as_deref(), Some("ctx-ops-alloydb"));
    assert_eq!(p.cluster_id.as_deref(), Some("ops"));
    assert_eq!(p.instance_id.as_deref(), Some("primary"));
    assert_eq!(p.database_id.as_deref(), Some("opsdb"));
    assert_eq!(p.source_type.family(), ProfileFamily::QueryData);
}

#[test]
fn load_spanner_finance_profile() {
    let p = profiles::load_profile(Path::new("deploy/ca/profiles/spanner-finance.yaml")).unwrap();
    assert_eq!(p.name, "spanner-finance");
    assert_eq!(p.source_type, SourceType::Spanner);
    assert_eq!(p.context_set_id.as_deref(), Some("ctx-finance-spanner"));
    assert_eq!(p.instance_id.as_deref(), Some("finance"));
    assert_eq!(p.database_id.as_deref(), Some("ledger"));
    assert_eq!(p.source_type.family(), ProfileFamily::QueryData);
}

#[test]
fn load_all_profiles_from_dir() {
    let profiles = profiles::load_profiles_from_dir(Path::new("deploy/ca/profiles")).unwrap();
    assert_eq!(profiles.len(), 4);
    // Sorted by name
    assert_eq!(profiles[0].name, "alloydb-ops");
    assert_eq!(profiles[1].name, "bigquery-demo");
    assert_eq!(profiles[2].name, "looker-sales");
    assert_eq!(profiles[3].name, "spanner-finance");
}

#[test]
fn load_profiles_from_missing_dir_returns_empty() {
    let profiles =
        profiles::load_profiles_from_dir(Path::new("deploy/ca/profiles/nonexistent")).unwrap();
    assert!(profiles.is_empty());
}

#[test]
fn invalid_profile_yaml_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.yaml");
    std::fs::write(&path, "not: valid: yaml: [[[").unwrap();
    assert!(profiles::load_profile(&path).is_err());
}

#[test]
fn profile_missing_required_field_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing-cluster.yaml");
    std::fs::write(&path, "name: bad\nsource_type: alloy_db\nproject: p\n").unwrap();
    let err = profiles::load_profile(&path).unwrap_err();
    assert!(err.to_string().contains("cluster_id"));
}

#[test]
fn profile_does_not_require_project_id() {
    // --profile supplies its own project, so --project-id should NOT be required.
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "ca",
            "ask",
            "--profile",
            "deploy/ca/profiles/bigquery-demo.yaml",
            "test question",
        ])
        .env("BQX_TOKEN", "fake-token")
        .output()
        .expect("failed to run bqx");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Must NOT fail with "project-id is required"
    assert!(
        !stderr.contains("--project-id or BQX_PROJECT is required"),
        "Profile should supply project, but got: {stderr}"
    );
    // Must NOT fail with "unexpected argument"
    assert!(
        !stderr.contains("unexpected argument"),
        "CLI should accept --profile flag, got: {stderr}"
    );
}

#[test]
fn profile_rejects_conflicting_agent_flag() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "ca",
            "ask",
            "--profile",
            "deploy/ca/profiles/bigquery-demo.yaml",
            "--agent",
            "some-agent",
            "test question",
        ])
        .env("BQX_TOKEN", "fake-token")
        .output()
        .expect("failed to run bqx");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--profile cannot be combined with --agent or --tables"),
        "Should reject --profile + --agent, got: {stderr}"
    );
}

#[test]
fn looker_profile_does_not_return_unsupported() {
    // Looker profiles should NOT fail with "not yet supported" (M2 implemented).
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "ca",
            "ask",
            "--profile",
            "deploy/ca/profiles/looker-sales.yaml",
            "test question",
        ])
        .env("BQX_TOKEN", "fake-token")
        .output()
        .expect("failed to run bqx");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Must NOT fail with the old "not yet supported" message
    assert!(
        !stderr.contains("not yet supported"),
        "Looker should be supported now, got: {stderr}"
    );
    // Must NOT fail with "unexpected argument"
    assert!(
        !stderr.contains("unexpected argument"),
        "CLI should accept Looker profile, got: {stderr}"
    );
}

#[test]
fn looker_profile_rejects_invalid_explore_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad-explore.yaml");
    std::fs::write(
        &path,
        "name: bad\nsource_type: looker\nproject: p\nlooker_instance_url: https://x.com\nlooker_explores:\n  - no_slash\n",
    )
    .unwrap();
    let err = profiles::load_profile(&path).unwrap_err();
    assert!(err.to_string().contains("invalid explore format"));
}

#[test]
fn profile_rejects_conflicting_tables_flag() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "ca",
            "ask",
            "--profile",
            "deploy/ca/profiles/bigquery-demo.yaml",
            "--tables",
            "p.d.t",
            "test question",
        ])
        .env("BQX_TOKEN", "fake-token")
        .output()
        .expect("failed to run bqx");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--profile cannot be combined with --agent or --tables"),
        "Should reject --profile + --tables, got: {stderr}"
    );
}
