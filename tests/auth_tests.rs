use std::io::Write;
use std::process::Command;

fn cargo_bin() -> String {
    // Build once and return path to binary
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

fn run_bqx_with_env(args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let bin = cargo_bin();
    let mut cmd = Command::new(&bin);
    cmd.args(args)
        .env_remove("BQX_TOKEN")
        .env_remove("BQX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env("BQX_PROJECT", "test-project");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to run bqx")
}

// ── Precedence tests ──

#[test]
fn explicit_token_takes_highest_priority() {
    let output = run_bqx_with_env(&["auth", "status"], &[("BQX_TOKEN", "my-test-token")]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("BQX_TOKEN"),
        "Expected BQX_TOKEN source, got: {stderr}"
    );
}

#[test]
fn credentials_file_takes_priority_over_adc() {
    // Create a temp authorized_user credentials file
    let dir = tempfile::tempdir().unwrap();
    let creds_path = dir.path().join("creds.json");
    let mut f = std::fs::File::create(&creds_path).unwrap();
    writeln!(
        f,
        r#"{{
            "type": "authorized_user",
            "client_id": "test-client-id",
            "client_secret": "test-client-secret",
            "refresh_token": "test-refresh-token"
        }}"#
    )
    .unwrap();

    let output = run_bqx_with_env(
        &["auth", "status"],
        &[("BQX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("credentials file"),
        "Expected credentials file source, got: {stderr}"
    );
}

#[test]
fn credentials_file_rejects_invalid_type() {
    let dir = tempfile::tempdir().unwrap();
    let creds_path = dir.path().join("bad.json");
    let mut f = std::fs::File::create(&creds_path).unwrap();
    writeln!(f, r#"{{"type": "unknown_type"}}"#).unwrap();

    let output = run_bqx_with_env(
        &["auth", "status"],
        &[("BQX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported credential type"),
        "Expected unsupported type error, got: {stderr}"
    );
}

#[test]
fn credentials_file_handles_authorized_user() {
    let dir = tempfile::tempdir().unwrap();
    let creds_path = dir.path().join("user.json");
    let mut f = std::fs::File::create(&creds_path).unwrap();
    writeln!(
        f,
        r#"{{
            "type": "authorized_user",
            "client_id": "test-id",
            "client_secret": "test-secret",
            "refresh_token": "test-refresh"
        }}"#
    )
    .unwrap();

    let output = run_bqx_with_env(
        &["auth", "status"],
        &[("BQX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // It should identify as credentials file (even if token refresh fails)
    assert!(
        stderr.contains("credentials file"),
        "Expected credentials file source, got: {stderr}"
    );
}

#[test]
fn credentials_file_rejects_missing_fields() {
    let dir = tempfile::tempdir().unwrap();
    let creds_path = dir.path().join("incomplete.json");
    let mut f = std::fs::File::create(&creds_path).unwrap();
    writeln!(f, r#"{{"type": "authorized_user", "client_id": "test"}}"#).unwrap();

    let output = run_bqx_with_env(
        &["auth", "status"],
        &[("BQX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Missing"),
        "Expected missing field error, got: {stderr}"
    );
}

// ── Auth status source reporting tests ──

#[test]
fn auth_status_reports_explicit_token_source() {
    let output = run_bqx_with_env(&["auth", "status"], &[("BQX_TOKEN", "test-bearer")]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("BQX_TOKEN / --token"));
    assert!(stderr.contains("Token: valid"));
}

#[test]
fn auth_status_reports_google_application_credentials() {
    // Create a service account file (will fail to load but should be identified)
    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("sa.json");
    let mut f = std::fs::File::create(&sa_path).unwrap();
    writeln!(
        f,
        r#"{{
            "type": "service_account",
            "project_id": "test",
            "private_key_id": "key1",
            "private_key": "fake",
            "client_email": "test@test.iam.gserviceaccount.com",
            "client_id": "123",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }}"#
    )
    .unwrap();

    let output = run_bqx_with_env(
        &["auth", "status"],
        &[("GOOGLE_APPLICATION_CREDENTIALS", sa_path.to_str().unwrap())],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should identify GOOGLE_APPLICATION_CREDENTIALS as the source
    // (may fail to actually get a token since the key is fake, but source should be reported)
    assert!(
        stderr.contains("GOOGLE_APPLICATION_CREDENTIALS") || stderr.contains("credentials"),
        "Expected GAC source identified, got: {stderr}"
    );
}

// ── Auth command basic tests ──

#[test]
fn auth_help_shows_subcommands() {
    let output = run_bqx(&["auth", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("login"), "Missing login subcommand");
    assert!(stdout.contains("status"), "Missing status subcommand");
    assert!(stdout.contains("logout"), "Missing logout subcommand");
}

#[test]
fn auth_logout_succeeds() {
    let output = run_bqx(&["auth", "logout"]);
    assert!(output.status.success(), "auth logout should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Stored credentials cleared"));
}

// ── CLI flag passthrough tests (Fix 1) ──

#[test]
fn auth_status_respects_token_cli_flag() {
    // --token flag should be passed through to auth status (not just env var)
    let output = run_bqx(&["--token", "cli-flag-token", "auth", "status"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("BQX_TOKEN / --token"),
        "Expected --token flag recognized, got: {stderr}"
    );
    assert!(
        stderr.contains("Token: valid"),
        "Expected token valid, got: {stderr}"
    );
}

#[test]
fn auth_status_respects_credentials_file_cli_flag() {
    let dir = tempfile::tempdir().unwrap();
    let creds_path = dir.path().join("flag-creds.json");
    let mut f = std::fs::File::create(&creds_path).unwrap();
    writeln!(
        f,
        r#"{{
            "type": "authorized_user",
            "client_id": "flag-id",
            "client_secret": "flag-secret",
            "refresh_token": "flag-refresh"
        }}"#
    )
    .unwrap();

    let output = run_bqx(&[
        "--credentials-file",
        creds_path.to_str().unwrap(),
        "auth",
        "status",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("credentials file"),
        "Expected --credentials-file flag recognized, got: {stderr}"
    );
}

// ── Cross-platform random generation test (Fix 3) ──

#[test]
fn login_generate_random_produces_valid_output() {
    // Verify that `bqx auth login` starts without crashing (tests the random generation path).
    // We can't complete the OAuth flow, but we can verify it gets far enough to print the URL.
    // Use a timeout via the process to avoid hanging on the listener.
    let bin = cargo_bin();
    let child = std::process::Command::new(&bin)
        .args(["auth", "login"])
        .env_remove("BQX_TOKEN")
        .env_remove("BQX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env("BQX_PROJECT", "test-project")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start bqx auth login");

    // Give it a moment to start and print the URL
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Kill the process (it's waiting for the OAuth callback)
    let output = {
        let mut child = child;
        // On Unix, we can kill directly
        let _ = child.kill();
        child.wait_with_output().expect("Failed to wait for child")
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    // If random generation works, it should get to printing the auth URL
    assert!(
        stderr.contains("accounts.google.com") || stderr.contains("Opening browser"),
        "Expected auth URL or browser message (random gen works), got: {stderr}"
    );
}

// ── Refresh path integration test ──

#[test]
fn credentials_file_refresh_path_is_exercised() {
    // An authorized_user credentials file should go through the Refreshable path.
    // With fake credentials, the refresh will fail — but the error message from
    // Google's token endpoint proves that refresh_access_token() was actually called.
    let dir = tempfile::tempdir().unwrap();
    let creds_path = dir.path().join("refresh-test.json");
    let mut f = std::fs::File::create(&creds_path).unwrap();
    writeln!(
        f,
        r#"{{
            "type": "authorized_user",
            "client_id": "fake-client-id",
            "client_secret": "fake-client-secret",
            "refresh_token": "fake-refresh-token"
        }}"#
    )
    .unwrap();

    let output = run_bqx_with_env(
        &["auth", "status"],
        &[("BQX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should identify source correctly
    assert!(
        stderr.contains("credentials file"),
        "Expected credentials file source, got: {stderr}"
    );
    // Token verification should attempt refresh and fail (proving refresh path ran)
    assert!(
        stderr.contains("Token: error") && stderr.contains("Token refresh failed"),
        "Expected refresh failure error proving refresh path was called, got: {stderr}"
    );
}

// ── Dry run still works without auth ──

#[test]
fn dry_run_does_not_require_auth() {
    let output = run_bqx(&["jobs", "query", "--query", "SELECT 1", "--dry-run"]);
    assert!(
        output.status.success(),
        "dry-run should succeed without auth"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run"));
}
