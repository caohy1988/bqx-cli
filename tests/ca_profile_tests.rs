use std::path::Path;

use bqx::ca::profiles::{self, ProfileFamily, SourceType};

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
}

#[test]
fn load_alloydb_ops_profile() {
    let p = profiles::load_profile(Path::new("deploy/ca/profiles/alloydb-ops.yaml")).unwrap();
    assert_eq!(p.name, "alloydb-ops");
    assert_eq!(p.source_type, SourceType::AlloyDb);
    assert_eq!(p.context_set_id.as_deref(), Some("ctx-ops-alloydb"));
    assert!(p.datasource_ref.is_some());
    assert_eq!(p.source_type.family(), ProfileFamily::QueryData);
}

#[test]
fn load_spanner_finance_profile() {
    let p = profiles::load_profile(Path::new("deploy/ca/profiles/spanner-finance.yaml")).unwrap();
    assert_eq!(p.name, "spanner-finance");
    assert_eq!(p.source_type, SourceType::Spanner);
    assert_eq!(p.context_set_id.as_deref(), Some("ctx-finance-spanner"));
    assert!(p.datasource_ref.is_some());
    assert_eq!(p.source_type.family(), ProfileFamily::QueryData);
}

#[test]
fn load_all_profiles_from_dir() {
    let profiles =
        profiles::load_profiles_from_dir(Path::new("deploy/ca/profiles")).unwrap();
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
    let path = dir.path().join("missing-context.yaml");
    std::fs::write(
        &path,
        "name: bad\nsource_type: alloy_db\nproject: p\n",
    )
    .unwrap();
    let err = profiles::load_profile(&path).unwrap_err();
    assert!(err.to_string().contains("context_set_id"));
}

#[test]
fn bigquery_profile_works_with_profile_flag() {
    // Verify the CLI accepts --profile alongside ca ask
    let result = std::process::Command::new("cargo")
        .args([
            "run", "--", "ca", "ask", "--profile", "deploy/ca/profiles/bigquery-demo.yaml",
            "test question", "--project-id", "test-proj",
        ])
        .env("BQX_TOKEN", "fake-token")
        .output();

    // We expect it to fail at the API call level (not at CLI parsing).
    // The important thing is it doesn't fail with "unknown flag --profile".
    if let Ok(output) = result {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should not complain about --profile being unknown
        assert!(
            !stderr.contains("unexpected argument"),
            "CLI should accept --profile flag, got: {stderr}"
        );
    }
}
