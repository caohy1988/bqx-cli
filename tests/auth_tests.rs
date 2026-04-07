use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;

fn cargo_bin() -> String {
    // Build once and return path to binary
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
        .env("DCX_PROJECT", "test-project")
        .output()
        .expect("Failed to run dcx")
}

fn run_dcx_with_env(args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
    let bin = cargo_bin();
    let mut cmd = Command::new(&bin);
    cmd.args(args)
        .env_remove("DCX_TOKEN")
        .env_remove("DCX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env("DCX_PROJECT", "test-project");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to run dcx")
}

// ── Precedence tests ──

#[test]
fn explicit_token_takes_highest_priority() {
    let output = run_dcx_with_env(&["auth", "status"], &[("DCX_TOKEN", "my-test-token")]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DCX_TOKEN"),
        "Expected DCX_TOKEN source, got: {stderr}"
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

    let output = run_dcx_with_env(
        &["auth", "status"],
        &[("DCX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
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

    let output = run_dcx_with_env(
        &["auth", "status"],
        &[("DCX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
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

    let output = run_dcx_with_env(
        &["auth", "status"],
        &[("DCX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
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

    let output = run_dcx_with_env(
        &["auth", "status"],
        &[("DCX_CREDENTIALS_FILE", creds_path.to_str().unwrap())],
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
    let output = run_dcx_with_env(&["auth", "status"], &[("DCX_TOKEN", "test-bearer")]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("DCX_TOKEN / --token"));
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

    let output = run_dcx_with_env(
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
    let output = run_dcx(&["auth", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("login"), "Missing login subcommand");
    assert!(stdout.contains("status"), "Missing status subcommand");
    assert!(stdout.contains("logout"), "Missing logout subcommand");
    assert!(stdout.contains("check"), "Missing check subcommand");
}

#[test]
fn auth_logout_succeeds() {
    let output = run_dcx(&["auth", "logout"]);
    assert!(output.status.success(), "auth logout should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Stored credentials cleared"));
}

// ── CLI flag passthrough tests (Fix 1) ──

#[test]
fn auth_status_respects_token_cli_flag() {
    // --token flag should be passed through to auth status (not just env var)
    let output = run_dcx(&["--token", "cli-flag-token", "auth", "status"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DCX_TOKEN / --token"),
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

    let output = run_dcx(&[
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
fn login_rejects_non_interactive_terminal() {
    // `dcx auth login` requires a TTY. In CI / piped stdin, it should exit 3
    // with a structured error explaining alternative auth methods.
    let bin = cargo_bin();
    let output = std::process::Command::new(&bin)
        .args(["auth", "login"])
        .env_remove("DCX_TOKEN")
        .env_remove("DCX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env("DCX_PROJECT", "test-project")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to run dcx auth login");

    assert_eq!(output.status.code(), Some(3), "should exit 3 (auth error)");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("interactive terminal"),
        "Expected non-interactive rejection message, got: {stderr}"
    );
    assert!(
        stderr.contains("--token") && stderr.contains("--credentials-file"),
        "Should suggest alternative auth methods, got: {stderr}"
    );
}

// ── auth check preflight tests ──

/// Start a mock tokeninfo server that rejects all tokens with 400.
/// Returns the base URL (e.g. "http://127.0.0.1:PORT/tokeninfo").
fn start_mock_tokeninfo_reject() -> (String, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/tokeninfo");
    listener
        .set_nonblocking(false)
        .expect("set_nonblocking failed");
    (url, listener)
}

/// Accept one request on the mock server and respond with 400 (invalid token).
fn serve_one_reject(listener: &TcpListener) {
    let (mut stream, _) = listener.accept().unwrap();
    // Read the request (we don't care about contents).
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf);
    let body = r#"{"error":"invalid_token","error_description":"Invalid Value"}"#;
    let response = format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Accept one request and respond with 200 (valid token).
fn serve_one_accept(listener: &TcpListener, email: &str) {
    let (mut stream, _) = listener.accept().unwrap();
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf);
    let body = format!(r#"{{"email":"{email}","expires_in":3600}}"#);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[test]
fn auth_check_with_bogus_token_reports_invalid() {
    let (mock_url, listener) = start_mock_tokeninfo_reject();

    // Spawn dcx in a thread so we can serve the mock response.
    let mock_url_clone = mock_url.clone();
    let handle = std::thread::spawn(move || {
        run_dcx_with_env(
            &["auth", "check"],
            &[
                ("DCX_TOKEN", "definitely-not-real"),
                ("DCX_TOKENINFO_URL", &mock_url_clone),
            ],
        )
    });

    serve_one_reject(&listener);
    let output = handle.join().unwrap();

    assert_eq!(
        output.status.code(),
        Some(3),
        "auth check with bogus token should exit 3"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON on stdout, got: {stdout}\nparse error: {e}"));
    assert_eq!(json["source"], "DCX_TOKEN / --token");
    assert_eq!(json["valid"], false);
    assert!(json["error"].is_string(), "Expected error message");
}

#[test]
fn auth_check_with_valid_token_reports_valid() {
    let (mock_url, listener) = start_mock_tokeninfo_reject();

    let mock_url_clone = mock_url.clone();
    let handle = std::thread::spawn(move || {
        run_dcx_with_env(
            &["auth", "check"],
            &[
                ("DCX_TOKEN", "good-token"),
                ("DCX_TOKENINFO_URL", &mock_url_clone),
            ],
        )
    });

    serve_one_accept(&listener, "user@example.com");
    let output = handle.join().unwrap();

    assert!(output.status.success(), "auth check should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["valid"], true);
    assert_eq!(json["source"], "DCX_TOKEN / --token");
    assert_eq!(json["account"], "user@example.com");
}

#[test]
fn auth_check_with_bad_credentials_file_exits_nonzero() {
    // Bad credentials file fails at resolve() before tokeninfo is called,
    // so no mock server needed.
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("bad.json");
    let mut f = std::fs::File::create(&bad_path).unwrap();
    writeln!(f, r#"{{"type": "unknown_type"}}"#).unwrap();

    let output = run_dcx_with_env(
        &["auth", "check"],
        &[("DCX_CREDENTIALS_FILE", bad_path.to_str().unwrap())],
    );
    assert!(
        !output.status.success(),
        "auth check with bad credentials should fail"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Expected JSON on stdout, got: {stdout}\nparse error: {e}"));
    assert_eq!(json["valid"], false);
    assert!(json["error"].is_string());
}

#[test]
fn auth_check_text_format_with_bogus_token() {
    let (mock_url, listener) = start_mock_tokeninfo_reject();

    let mock_url_clone = mock_url.clone();
    let handle = std::thread::spawn(move || {
        run_dcx_with_env(
            &["--format", "text", "auth", "check"],
            &[
                ("DCX_TOKEN", "text-check-bogus"),
                ("DCX_TOKENINFO_URL", &mock_url_clone),
            ],
        )
    });

    serve_one_reject(&listener);
    let output = handle.join().unwrap();

    assert_eq!(
        output.status.code(),
        Some(3),
        "text format with bogus token should exit 3"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("FAIL"),
        "Expected FAIL in text output, got: {stdout}"
    );
    assert!(
        stdout.contains("DCX_TOKEN"),
        "Expected source in text output, got: {stdout}"
    );
}

#[test]
fn auth_check_identifies_source_from_cli_flag() {
    let (mock_url, listener) = start_mock_tokeninfo_reject();

    let mock_url_clone = mock_url.clone();
    let handle = std::thread::spawn(move || {
        run_dcx_with_env(
            &["--token", "flag-check-bogus", "auth", "check"],
            &[("DCX_TOKENINFO_URL", &mock_url_clone)],
        )
    });

    serve_one_reject(&listener);
    let output = handle.join().unwrap();

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["source"], "DCX_TOKEN / --token");
    assert_eq!(json["valid"], false);
}

// ── Refresh path tests ──
// The actual refresh token exchange is tested deterministically via wiremock
// in src/auth/resolver.rs unit tests. Integration tests here only verify
// source identification without depending on live Google endpoints.

// ── Dry run and output format tests ──

#[test]
fn dry_run_does_not_require_auth() {
    let output = run_dcx(&["jobs", "query", "--query", "SELECT 1", "--dry-run"]);
    assert!(
        output.status.success(),
        "dry-run should succeed without auth"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry_run"));
}

#[test]
fn dry_run_json_format() {
    let output = run_dcx(&[
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
    assert!(stdout.contains("\"dry_run\""));
    assert!(stdout.contains("\"method\""));
}

#[test]
fn dry_run_table_format() {
    let output = run_dcx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "table",
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table format renders as key-value table with Field/Value headers
    assert!(
        stdout.contains("dry_run") || stdout.contains("Field"),
        "Expected table output, got: {stdout}"
    );
}

#[test]
fn dry_run_text_format() {
    let output = run_dcx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "text",
    ]);
    assert!(
        output.status.success(),
        "text format should work for dry-run"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dry run: POST"),
        "Expected text dry run header, got: {stdout}"
    );
    assert!(
        stdout.contains("Query: SELECT 1"),
        "Expected query in text output, got: {stdout}"
    );
    assert!(
        stdout.contains("Location: US"),
        "Expected location in text output, got: {stdout}"
    );
}

#[test]
fn format_text_is_accepted_by_help() {
    let output = run_dcx(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("text"),
        "Expected 'text' in --format help, got: {stdout}"
    );
}
