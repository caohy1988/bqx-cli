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
fn top_level_help_includes_cloudsql() {
    let output = run_dcx(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("cloudsql"),
        "Help should list 'cloudsql' command"
    );
}

#[test]
fn cloudsql_help_shows_subcommands() {
    let output = run_dcx(&["cloudsql", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("instances"),
        "Should show instances subcommand"
    );
    assert!(
        stdout.contains("databases"),
        "Should show databases subcommand"
    );
}

#[test]
fn cloudsql_instances_help_shows_list_and_get() {
    let output = run_dcx(&["cloudsql", "instances", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
    assert!(stdout.contains("get"), "Should show get subcommand");
}

#[test]
fn cloudsql_databases_help_shows_list() {
    let output = run_dcx(&["cloudsql", "databases", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"), "Should show list subcommand");
}

// ── requires --project-id ──

#[test]
fn cloudsql_instances_list_requires_project_id() {
    let output = run_dcx(&["cloudsql", "instances", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}

#[test]
fn cloudsql_instances_get_requires_project_id() {
    let output = run_dcx(&["cloudsql", "instances", "get", "--instance", "my-inst"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}

#[test]
fn cloudsql_databases_list_requires_project_id() {
    let output = run_dcx(&["cloudsql", "databases", "list", "--instance", "my-inst"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--project-id") || stderr.contains("DCX_PROJECT"),
        "Should require --project-id, got: {stderr}"
    );
}

// ── instances get requires --instance ──

#[test]
fn cloudsql_instances_get_requires_instance() {
    let output = run_dcx(&["cloudsql", "instances", "get"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--instance"),
        "Should require --instance flag, got: {stderr}"
    );
}

// ── databases list requires --instance ──

#[test]
fn cloudsql_databases_list_requires_instance() {
    let output = run_dcx(&["cloudsql", "databases", "list"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--instance"),
        "Should require --instance flag, got: {stderr}"
    );
}
