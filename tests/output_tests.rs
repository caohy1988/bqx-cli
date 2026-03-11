use std::process::Command;

fn cargo_bin() -> String {
    let output = Command::new(env!("CARGO"))
        .args(["build", "--quiet"])
        .output()
        .expect("Failed to build");
    assert!(output.status.success(), "Build failed");

    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| String::from("target"));
    format!("{target_dir}/debug/bqx")
}

fn run_bqx(args: &[&str]) -> std::process::Output {
    let bin = cargo_bin();
    Command::new(&bin)
        .args(args)
        .env_remove("BQX_TOKEN")
        .env_remove("BQX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env("BQX_PROJECT", "test-project")
        .output()
        .expect("Failed to run bqx")
}

// ── Text format dry-run rendering ──

#[test]
fn text_dry_run_shows_structured_output() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT session_id, agent FROM events",
        "--dry-run",
        "--format",
        "text",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 4, "Expected 4 lines, got: {stdout}");
    assert!(lines[0].starts_with("Dry run: POST https://"));
    assert!(lines[1].starts_with("Query: SELECT session_id"));
    assert_eq!(lines[2], "Legacy SQL: false");
    assert_eq!(lines[3], "Location: US");
}

#[test]
fn text_dry_run_with_legacy_sql() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--use-legacy-sql",
        "--format",
        "text",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Legacy SQL: true"),
        "Expected legacy sql true, got: {stdout}"
    );
}

// ── JSON format stability (unchanged by this PR) ──

#[test]
fn json_dry_run_shape_unchanged() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "json",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("Invalid JSON: {e}\n{stdout}"));

    assert_eq!(json["dry_run"], true);
    assert_eq!(json["method"], "POST");
    assert!(json["url"].as_str().unwrap().contains("bigquery"));
    assert_eq!(json["body"]["query"], "SELECT 1");
    assert_eq!(json["body"]["useLegacySql"], false);
}

// ── --format accepts all three values ──

#[test]
fn format_flag_accepts_json() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "json",
    ]);
    assert!(output.status.success());
}

#[test]
fn format_flag_accepts_table() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "table",
    ]);
    assert!(output.status.success());
}

#[test]
fn format_flag_accepts_text() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "text",
    ]);
    assert!(output.status.success());
}

#[test]
fn format_flag_rejects_invalid() {
    let output = run_bqx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "csv",
    ]);
    assert!(!output.status.success(), "csv should not be accepted");
}
