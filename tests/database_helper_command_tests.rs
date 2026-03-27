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

#[test]
fn spanner_schema_describe_help_shows_profile() {
    let output = run_dcx(&["spanner", "schema", "describe", "--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("Describe database schema from a source profile"));
}

#[test]
fn cloudsql_schema_describe_help_shows_profile() {
    let output = run_dcx(&["cloudsql", "schema", "describe", "--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("Describe database schema from a source profile"));
}

#[test]
fn alloydb_databases_list_help_shows_profile() {
    let output = run_dcx(&["alloydb", "databases", "list", "--help"]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--profile"));
    assert!(stdout.contains("List databases visible to the AlloyDB profile"));
}

#[test]
fn spanner_schema_describe_rejects_wrong_profile_type_before_network() {
    let output = run_dcx(&[
        "spanner",
        "schema",
        "describe",
        "--profile",
        "deploy/ca/profiles/alloydb-ops.yaml",
        "--token",
        "test-token",
    ]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expected 'spanner'"));
}

#[test]
fn cloudsql_schema_describe_rejects_wrong_profile_type_before_network() {
    let output = run_dcx(&[
        "cloudsql",
        "schema",
        "describe",
        "--profile",
        "deploy/ca/profiles/spanner-finance.yaml",
        "--token",
        "test-token",
    ]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expected 'cloud_sql'"));
}

#[test]
fn alloydb_databases_list_rejects_wrong_profile_type_before_network() {
    let output = run_dcx(&[
        "alloydb",
        "databases",
        "list",
        "--profile",
        "deploy/ca/profiles/spanner-finance.yaml",
        "--token",
        "test-token",
    ]);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expected 'alloydb'"));
}
