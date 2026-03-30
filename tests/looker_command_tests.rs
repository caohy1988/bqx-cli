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
        .env_remove("DCX_PROJECT")
        .output()
        .expect("Failed to run dcx")
}

// ── help ──

#[test]
fn top_level_help_includes_looker() {
    let output = run_dcx(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("looker"),
        "Help should list 'looker' command"
    );
}

#[test]
fn looker_help_shows_subcommands() {
    let output = run_dcx(&["looker", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("explores"),
        "Should show explores subcommand"
    );
    assert!(
        stdout.contains("dashboards"),
        "Should show dashboards subcommand"
    );
    assert!(
        stdout.contains("instances"),
        "Should show instances subcommand (Discovery-generated)"
    );
    assert!(
        stdout.contains("backups"),
        "Should show backups subcommand (Discovery-generated)"
    );
}

#[test]
fn looker_explores_help_shows_list_and_get() {
    let output = run_dcx(&["looker", "explores", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
    assert!(stdout.contains("get"), "Should show get subcommand");
}

#[test]
fn looker_dashboards_help_shows_list_and_get() {
    let output = run_dcx(&["looker", "dashboards", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
    assert!(stdout.contains("get"), "Should show get subcommand");
}

// ── source type validation ──

#[test]
fn looker_explores_rejects_non_looker_profile() {
    let output = run_dcx(&[
        "looker",
        "explores",
        "list",
        "--profile",
        "deploy/ca/profiles/bigquery-demo.yaml",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("expected 'looker'"),
        "Should reject non-Looker profile, got: {stderr}"
    );
}

#[test]
fn looker_dashboards_rejects_non_looker_profile() {
    let output = run_dcx(&[
        "looker",
        "dashboards",
        "list",
        "--profile",
        "deploy/ca/profiles/bigquery-demo.yaml",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("expected 'looker'"),
        "Should reject non-Looker profile, got: {stderr}"
    );
}

// ── missing profile ──

#[test]
fn looker_explores_missing_profile_fails() {
    let output = run_dcx(&[
        "looker",
        "explores",
        "list",
        "--profile",
        "nonexistent-profile",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "Should report profile not found, got: {stderr}"
    );
}

// ── explore get validation ──

#[test]
fn looker_explores_get_rejects_invalid_explore_ref() {
    let output = run_dcx(&[
        "looker",
        "explores",
        "get",
        "--profile",
        "deploy/ca/profiles/looker-sales.yaml",
        "--explore",
        "no_slash",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid Looker explore"),
        "Should reject malformed explore ref, got: {stderr}"
    );
}

// ── missing --profile flag ──

#[test]
fn looker_explores_list_requires_profile_flag() {
    let output = run_dcx(&["looker", "explores", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--profile"),
        "Should require --profile flag, got: {stderr}"
    );
}

#[test]
fn looker_dashboards_get_requires_dashboard_id() {
    let output = run_dcx(&[
        "looker",
        "dashboards",
        "get",
        "--profile",
        "deploy/ca/profiles/looker-sales.yaml",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--dashboard-id"),
        "Should require --dashboard-id flag, got: {stderr}"
    );
}

// ── Discovery-generated admin commands (instances, backups) ──

#[test]
fn looker_instances_help_shows_list_and_get() {
    let output = run_dcx(&["looker", "instances", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
    assert!(stdout.contains("get"), "Should show get subcommand");
}

#[test]
fn looker_backups_help_shows_list_and_get() {
    let output = run_dcx(&["looker", "backups", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
    assert!(stdout.contains("get"), "Should show get subcommand");
}

#[test]
fn looker_instances_list_requires_project_id() {
    let output = run_dcx(&["looker", "instances", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}

#[test]
fn looker_instances_get_requires_instance_id() {
    let output = run_dcx(&[
        "looker",
        "instances",
        "get",
        "--project-id",
        "test-proj",
        "--location",
        "us-central1",
    ]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("--instance"),
        "Should require --instance-id flag, got: {combined}"
    );
}

#[test]
fn looker_instances_list_dry_run_normalizes_default_location() {
    // When --location is omitted, the global default "US" should be
    // normalized to "-" (all locations) for Looker, not produce an
    // invalid ".../locations/US/..." path.
    let output = run_dcx(&[
        "looker",
        "instances",
        "list",
        "--project-id",
        "test-proj",
        "--dry-run",
    ]);
    assert!(
        output.status.success(),
        "dry-run should succeed without auth"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("locations/-/"),
        "Default location should be normalized to '-', got: {stdout}"
    );
    assert!(
        !stdout.contains("locations/US/"),
        "Should NOT contain literal 'locations/US/', got: {stdout}"
    );
}

#[test]
fn looker_instances_list_dry_run_preserves_explicit_location() {
    let output = run_dcx(&[
        "looker",
        "instances",
        "list",
        "--project-id",
        "test-proj",
        "--location",
        "us-central1",
        "--dry-run",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("locations/us-central1/"),
        "Explicit location should be preserved, got: {stdout}"
    );
}

#[test]
fn looker_backups_list_requires_project_id() {
    let output = run_dcx(&[
        "looker",
        "backups",
        "list",
        "--instance-id",
        "my-instance",
        "--location",
        "us-central1",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}
