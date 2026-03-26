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
fn top_level_help_includes_alloydb() {
    let output = run_dcx(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alloydb"),
        "Help should list 'alloydb' command"
    );
}

#[test]
fn alloydb_help_shows_subcommands() {
    let output = run_dcx(&["alloydb", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("clusters"),
        "Should show clusters subcommand"
    );
    assert!(
        stdout.contains("instances"),
        "Should show instances subcommand"
    );
}

#[test]
fn alloydb_clusters_help_shows_list() {
    let output = run_dcx(&["alloydb", "clusters", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
}

#[test]
fn alloydb_instances_help_shows_list() {
    let output = run_dcx(&["alloydb", "instances", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
}

// ── requires --project-id ──

#[test]
fn alloydb_clusters_list_requires_project_id() {
    let output = run_dcx(&["alloydb", "clusters", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}

#[test]
fn alloydb_instances_list_requires_project_id() {
    let output = run_dcx(&["alloydb", "instances", "list", "--cluster", "my-cluster"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}

// ── identifier validation ──

#[test]
fn alloydb_rejects_invalid_project_id() {
    let output = run_dcx(&[
        "alloydb",
        "clusters",
        "list",
        "--project-id",
        "bad proj",
        "--token",
        "test-token",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid project-id"),
        "Should reject invalid project-id locally, got: {stderr}"
    );
}

#[test]
fn alloydb_rejects_invalid_cluster_id() {
    let output = run_dcx(&[
        "alloydb",
        "instances",
        "list",
        "--project-id",
        "good-proj",
        "--cluster",
        "my/cluster",
        "--token",
        "test-token",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid cluster"),
        "Should reject invalid cluster locally, got: {stderr}"
    );
}

// ── instances list requires --cluster ──

#[test]
fn alloydb_instances_list_requires_cluster() {
    let output = run_dcx(&["alloydb", "instances", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--cluster"),
        "Should require --cluster flag, got: {stderr}"
    );
}
