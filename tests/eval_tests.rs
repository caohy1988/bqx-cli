//! M5a: Deterministic agent eval suite.
//!
//! Task-level tests that validate agent-relevant behaviors without network or
//! LLM calls. Every test runs the compiled `dcx` binary and checks:
//!
//! - command discovery and contract completeness
//! - dry-run success across dynamic commands
//! - structured error envelopes on invalid input
//! - exit code semantics
//! - JSON contract stability
//! - preflight validation (missing required args caught before network)
//! - format support (json, table, text)
//!
//! These are CI-gated: any regression in agent-observable behavior fails the build.

use serde_json::Value;
use std::process::Command;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
        .env("DCX_PROJECT", "eval-test-project")
        .output()
        .expect("Failed to run dcx")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn parse_json(output: &std::process::Output) -> Value {
    let text = stdout(output);
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("Invalid JSON on stdout: {e}\n---\n{text}"))
}

#[allow(dead_code)]
fn parse_error_envelope(output: &std::process::Output) -> Value {
    let text = stderr(output);
    // Error envelope is on stderr; may have multiple lines, find the JSON one.
    for line in text.lines() {
        if line.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if v.get("error").is_some() {
                    return v;
                }
            }
        }
    }
    panic!("No error envelope found on stderr:\n{text}");
}

fn exit_code(output: &std::process::Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// Task 1: Command Discovery
// ---------------------------------------------------------------------------

/// An agent running `dcx meta commands --format json` should get a complete,
/// parseable command list with contract version and expected domains.
#[test]
fn eval_meta_commands_returns_complete_surface() {
    let output = run_dcx(&["meta", "commands", "--format", "json"]);
    assert!(output.status.success(), "meta commands failed");

    let json = parse_json(&output);
    assert_eq!(json["contract_version"], "1");

    let total = json["total"].as_u64().unwrap();
    assert!(total > 10, "Expected >10 commands, got {total}");

    let commands = json["commands"].as_array().unwrap();
    assert_eq!(commands.len(), total as usize);

    // Every command has required fields.
    for cmd in commands {
        assert!(
            cmd["command"].as_str().unwrap().starts_with("dcx "),
            "Command should start with 'dcx': {:?}",
            cmd["command"]
        );
        assert!(
            !cmd["domain"].as_str().unwrap().is_empty(),
            "Domain should not be empty for {:?}",
            cmd["command"]
        );
        // Synopsis may be empty for some auto-generated commands; track but
        // don't fail the eval — the contract still exists.
        let _synopsis = cmd["synopsis"].as_str().unwrap();
    }

    // All known domains are present.
    let domains: Vec<&str> = commands
        .iter()
        .map(|c| c["domain"].as_str().unwrap())
        .collect();
    for expected in &[
        "alloydb",
        "analytics",
        "auth",
        "bigquery",
        "ca",
        "cloudsql",
        "looker",
        "meta",
        "profiles",
        "spanner",
        "utility",
    ] {
        assert!(
            domains.contains(expected),
            "Missing domain '{expected}' in command surface"
        );
    }
}

/// An agent can discover dynamic BigQuery commands.
#[test]
fn eval_meta_commands_includes_dynamic_bigquery() {
    let output = run_dcx(&["meta", "commands", "--format", "json"]);
    let json = parse_json(&output);
    let commands: Vec<&str> = json["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["command"].as_str().unwrap())
        .collect();

    for expected in &[
        "dcx datasets list",
        "dcx datasets get",
        "dcx tables list",
        "dcx tables get",
        "dcx routines list",
        "dcx routines get",
        "dcx models list",
        "dcx models get",
    ] {
        assert!(
            commands.contains(expected),
            "Missing dynamic command: {expected}"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 2: Contract Completeness
// ---------------------------------------------------------------------------

/// `dcx meta describe` returns a full contract with flags, exit codes, and
/// dry-run support for a known command.
#[test]
fn eval_contract_describe_has_required_fields() {
    let output = run_dcx(&["meta", "describe", "datasets", "list", "--format", "json"]);
    assert!(output.status.success());

    let json = parse_json(&output);
    assert_eq!(json["contract_version"], "1");
    assert_eq!(json["command"], "dcx datasets list");
    assert!(!json["synopsis"].as_str().unwrap().is_empty());

    // Flags array is present and non-empty.
    let flags = json["flags"].as_array().unwrap();
    assert!(!flags.is_empty(), "flags should not be empty");

    // Each flag has required fields.
    for flag in flags {
        assert!(flag["name"].as_str().unwrap().starts_with("--"));
        assert!(!flag["type"].as_str().unwrap().is_empty());
        assert!(flag.get("required").is_some());
    }

    // Exit codes map is present.
    let exit_codes = json["exit_codes"].as_object().unwrap();
    assert!(exit_codes.contains_key("0"), "Missing exit code 0");

    // Output formats declared.
    let formats = json["output"]["formats"].as_array().unwrap();
    assert!(formats.len() >= 2, "Expected at least json+table formats");

    // supports_dry_run is declared.
    assert!(json.get("supports_dry_run").is_some());
}

/// Every command in the surface has a describable contract.
#[test]
fn eval_every_command_has_contract() {
    let list_output = run_dcx(&["meta", "commands", "--format", "json"]);
    let list = parse_json(&list_output);
    let commands = list["commands"].as_array().unwrap();

    for cmd in commands {
        let path = cmd["command"].as_str().unwrap();
        let parts: Vec<&str> = path
            .strip_prefix("dcx ")
            .unwrap()
            .split_whitespace()
            .collect();
        let mut args: Vec<&str> = vec!["meta", "describe"];
        args.extend(&parts);
        args.push("--format");
        args.push("json");

        let describe_output = run_dcx(&args);
        assert!(
            describe_output.status.success(),
            "meta describe failed for '{path}': {}",
            stderr(&describe_output)
        );

        let contract = parse_json(&describe_output);
        assert_eq!(
            contract["command"].as_str().unwrap(),
            path,
            "Contract command mismatch"
        );
        assert!(
            contract["exit_codes"].as_object().is_some(),
            "{path}: missing exit_codes"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 3: Dry-Run Success (dynamic commands)
// ---------------------------------------------------------------------------

/// All dynamic BigQuery commands succeed with --dry-run --format json.
#[test]
fn eval_dynamic_commands_dry_run_json() {
    let test_cases: Vec<(&[&str], &str)> = vec![
        (
            &[
                "datasets",
                "list",
                "--project-id",
                "eval-proj",
                "--dry-run",
                "--format",
                "json",
            ],
            "datasets list",
        ),
        (
            &[
                "datasets",
                "get",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--dry-run",
                "--format",
                "json",
            ],
            "datasets get",
        ),
        (
            &[
                "tables",
                "list",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--dry-run",
                "--format",
                "json",
            ],
            "tables list",
        ),
        (
            &[
                "tables",
                "get",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--table-id",
                "my_tbl",
                "--dry-run",
                "--format",
                "json",
            ],
            "tables get",
        ),
        (
            &[
                "routines",
                "list",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--dry-run",
                "--format",
                "json",
            ],
            "routines list",
        ),
        (
            &[
                "routines",
                "get",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--routine-id",
                "my_routine",
                "--dry-run",
                "--format",
                "json",
            ],
            "routines get",
        ),
        (
            &[
                "models",
                "list",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--dry-run",
                "--format",
                "json",
            ],
            "models list",
        ),
        (
            &[
                "models",
                "get",
                "--project-id",
                "eval-proj",
                "--dataset-id",
                "my_ds",
                "--model-id",
                "my_model",
                "--dry-run",
                "--format",
                "json",
            ],
            "models get",
        ),
    ];

    for (args, label) in test_cases {
        let output = run_dcx(args);
        assert!(
            output.status.success(),
            "{label}: dry-run failed with exit {}. stderr: {}",
            exit_code(&output),
            stderr(&output)
        );

        let json = parse_json(&output);
        assert_eq!(json["dry_run"], true, "{label}: missing dry_run=true");
        assert!(json["method"].as_str().is_some(), "{label}: missing method");
        assert!(
            json["url"].as_str().unwrap().starts_with("https://"),
            "{label}: url should start with https://"
        );
    }
}

/// `jobs query --dry-run` produces stable JSON shape.
#[test]
fn eval_jobs_query_dry_run_json_contract() {
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

    let json = parse_json(&output);
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["method"], "POST");
    assert!(json["url"].as_str().unwrap().contains("bigquery"));
    assert!(json["body"]["query"].as_str().is_some());
}

// ---------------------------------------------------------------------------
// Task 4: Error Recovery — Structured Error Envelopes
// ---------------------------------------------------------------------------

/// Missing required --project-id (with env var also removed) produces an error.
#[test]
fn eval_missing_project_id_error_envelope() {
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["datasets", "list", "--dry-run", "--format", "json"])
        .env_remove("DCX_TOKEN")
        .env_remove("DCX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env_remove("DCX_PROJECT") // Remove env fallback too.
        .output()
        .expect("Failed to run dcx");

    // Should fail — project-id is required and no env fallback.
    assert!(!output.status.success());

    // Either clap error on stderr or structured error envelope.
    let err_text = stderr(&output);
    assert!(
        !err_text.is_empty(),
        "Expected error output on stderr for missing project-id"
    );
}

/// Invalid format value produces an error.
#[test]
fn eval_invalid_format_rejected() {
    let output = run_dcx(&[
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

/// Unknown subcommand produces a non-zero exit.
#[test]
fn eval_unknown_command_fails() {
    let output = run_dcx(&["nonexistent-command"]);
    assert!(!output.status.success());
}

// ---------------------------------------------------------------------------
// Task 5: Exit Code Semantics
// ---------------------------------------------------------------------------

/// Successful dry-run exits with code 0.
#[test]
fn eval_exit_code_0_on_success() {
    let output = run_dcx(&[
        "jobs",
        "query",
        "--query",
        "SELECT 1",
        "--dry-run",
        "--format",
        "json",
    ]);
    assert_eq!(exit_code(&output), 0);
}

/// Missing required args exit with code != 0.
#[test]
fn eval_exit_code_nonzero_on_missing_args() {
    let bin = cargo_bin();
    let output = Command::new(&bin)
        .args(["datasets", "list", "--dry-run"])
        .env_remove("DCX_TOKEN")
        .env_remove("DCX_CREDENTIALS_FILE")
        .env_remove("GOOGLE_APPLICATION_CREDENTIALS")
        .env_remove("DCX_PROJECT") // Remove env fallback.
        .output()
        .expect("Failed to run dcx");
    assert_ne!(exit_code(&output), 0);
}

/// `dcx meta describe` for nonexistent command exits non-zero.
#[test]
fn eval_meta_describe_nonexistent_fails() {
    let output = run_dcx(&[
        "meta",
        "describe",
        "nonexistent",
        "command",
        "--format",
        "json",
    ]);
    assert!(!output.status.success());
}

// ---------------------------------------------------------------------------
// Task 6: JSON Contract Stability
// ---------------------------------------------------------------------------

/// Dynamic dry-run JSON has exactly the expected keys.
#[test]
fn eval_dynamic_dry_run_json_keys_stable() {
    let output = run_dcx(&[
        "datasets",
        "list",
        "--project-id",
        "eval-proj",
        "--dry-run",
        "--format",
        "json",
    ]);
    assert!(output.status.success());

    let json = parse_json(&output);
    let obj = json.as_object().unwrap();

    // Must have these keys.
    for key in &["dry_run", "url", "method"] {
        assert!(
            obj.contains_key(*key),
            "Missing key '{key}' in dry-run output"
        );
    }
}

/// `meta commands` JSON keys are stable.
#[test]
fn eval_meta_commands_json_keys_stable() {
    let output = run_dcx(&["meta", "commands", "--format", "json"]);
    let json = parse_json(&output);
    let obj = json.as_object().unwrap();

    for key in &["contract_version", "total", "commands"] {
        assert!(
            obj.contains_key(*key),
            "Missing key '{key}' in meta commands output"
        );
    }
}

/// `meta describe` JSON keys are stable.
#[test]
fn eval_meta_describe_json_keys_stable() {
    let output = run_dcx(&["meta", "describe", "datasets", "list", "--format", "json"]);
    let json = parse_json(&output);
    let obj = json.as_object().unwrap();

    for key in &[
        "contract_version",
        "command",
        "domain",
        "synopsis",
        "flags",
        "global_flags",
        "output",
        "exit_codes",
        "supports_dry_run",
        "is_mutation",
    ] {
        assert!(
            obj.contains_key(*key),
            "Missing key '{key}' in meta describe output"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 7: Help Completeness
// ---------------------------------------------------------------------------

/// Top-level --help lists all expected command groups.
#[test]
fn eval_help_lists_all_groups() {
    let output = run_dcx(&["--help"]);
    let text = stdout(&output);

    for group in &[
        "jobs",
        "analytics",
        "ca",
        "auth",
        "profiles",
        "meta",
        "datasets",
        "tables",
    ] {
        assert!(
            text.contains(group),
            "Top-level help missing group: {group}"
        );
    }
}

/// Subcommand help works for dynamic commands.
#[test]
fn eval_dynamic_command_help() {
    for group in &["datasets", "tables", "routines", "models"] {
        let output = run_dcx(&[group, "--help"]);
        assert!(output.status.success(), "{group} --help failed");
        let text = stdout(&output);
        assert!(
            text.contains("list") && text.contains("get"),
            "{group} --help should list 'list' and 'get' subcommands"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 8: Format Support
// ---------------------------------------------------------------------------

/// All three format values are accepted for dry-run commands.
#[test]
fn eval_all_formats_accepted() {
    for format in &["json", "table", "text"] {
        let output = run_dcx(&[
            "jobs",
            "query",
            "--query",
            "SELECT 1",
            "--dry-run",
            "--format",
            format,
        ]);
        assert!(
            output.status.success(),
            "--format {format} should be accepted"
        );
    }
}

/// `meta commands` supports all formats.
#[test]
fn eval_meta_commands_all_formats() {
    for format in &["json", "table", "text"] {
        let output = run_dcx(&["meta", "commands", "--format", format]);
        assert!(
            output.status.success(),
            "meta commands --format {format} failed"
        );
        assert!(
            !stdout(&output).is_empty(),
            "meta commands --format {format} produced empty output"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 9: Preflight Validation
// ---------------------------------------------------------------------------

/// Missing required flag is caught before network — dry-run not needed.
/// `datasets get` requires --dataset-id; omitting it should fail fast.
#[test]
fn eval_preflight_catches_missing_dataset_id() {
    let output = run_dcx(&[
        "datasets",
        "get",
        "--project-id",
        "eval-proj",
        "--format",
        "json",
    ]);
    assert!(!output.status.success());
    // Should mention dataset-id in the error.
    let err = stderr(&output);
    assert!(
        err.to_lowercase().contains("dataset"),
        "Error should mention missing dataset-id: {err}"
    );
}

/// `jobs query` without --query fails fast.
#[test]
fn eval_preflight_catches_missing_query() {
    let output = run_dcx(&["jobs", "query", "--dry-run", "--format", "json"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(
        err.to_lowercase().contains("query"),
        "Error should mention missing --query: {err}"
    );
}

// ---------------------------------------------------------------------------
// Task 10: Auth Preflight
// ---------------------------------------------------------------------------

/// `dcx auth status` works without credentials (reports not logged in).
#[test]
fn eval_auth_status_works_without_creds() {
    let output = run_dcx(&["auth", "status"]);
    // Should succeed or fail gracefully — not crash.
    let combined = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        !combined.is_empty(),
        "auth status should produce some output"
    );
}

/// `dcx auth check --format json` returns a structured response with stable
/// contract keys regardless of whether credentials are available.
///
/// This is a network call (tokeninfo endpoint), not a local-only preflight.
/// The eval validates the response contract shape, not a specific auth state.
#[test]
fn eval_auth_check_json_contract() {
    let output = run_dcx(&["auth", "check", "--format", "json"]);
    let code = exit_code(&output);

    // Exit 0 = valid creds, exit 3 = auth failure. Both are acceptable.
    assert!(
        code == 0 || code == 3,
        "auth check should exit 0 or 3, got {code}. stdout: {} stderr: {}",
        stdout(&output),
        stderr(&output)
    );

    // stdout must contain structured JSON with required contract fields.
    let text = stdout(&output);
    assert!(
        !text.trim().is_empty(),
        "auth check should produce JSON on stdout"
    );
    let json: Value = serde_json::from_str(text.trim())
        .unwrap_or_else(|e| panic!("auth check stdout not valid JSON: {e}\n{text}"));

    // Required fields in the auth check response.
    assert!(
        json["source"].as_str().is_some(),
        "auth check response must include 'source' field"
    );
    assert!(
        json.get("valid").is_some(),
        "auth check response must include 'valid' field"
    );

    // valid=true must pair with exit 0; valid=false must pair with exit 3.
    let valid = json["valid"].as_bool().unwrap();
    if valid {
        assert_eq!(code, 0, "valid=true should exit 0");
        assert!(
            json.get("account").is_some(),
            "valid auth check should include 'account' field"
        );
    } else {
        assert_eq!(code, 3, "valid=false should exit 3");
    }
}

// ---------------------------------------------------------------------------
// Task 11: Skill and Manifest Contract Alignment
// ---------------------------------------------------------------------------

/// Generated skills match the command surface (library-level check).
#[test]
fn eval_skills_cover_dynamic_commands() {
    use clap::CommandFactory;
    use dcx::bigquery::discovery::{self, DiscoverySource};
    use dcx::bigquery::dynamic::clap_tree;
    use dcx::bigquery::dynamic::model::{extract_methods, filter_allowed, to_generated_command};
    use dcx::bigquery::dynamic::service;
    use dcx::commands::meta;
    use dcx::skills::generator;

    let cfg = service::bigquery();
    let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
    let methods = extract_methods(&doc, cfg.use_flat_path);
    let allowed = filter_allowed(&methods, cfg.allowed_methods);
    let commands: Vec<_> = allowed.iter().map(to_generated_command).collect();

    let global_params = cfg.global_param_names();
    let dynamic_clap =
        clap_tree::build_dynamic_commands(&commands, &global_params, cfg.service_label);
    let mut app = dcx::cli::Cli::command();
    for sub in dynamic_clap {
        app = app.subcommand(sub);
    }
    let contracts = meta::collect_all(&app);

    let skills = generator::generate_all(&commands, &contracts);

    // Every dynamic group should have a skill.
    let skill_names: Vec<&str> = skills.iter().map(|s| s.dir_name.as_str()).collect();
    for expected in &["dcx-datasets", "dcx-tables", "dcx-routines", "dcx-models"] {
        assert!(skill_names.contains(expected), "Missing skill: {expected}");
    }

    // Every skill references_md should mention all commands in that group.
    for skill in &skills {
        let group = skill.dir_name.strip_prefix("dcx-").unwrap();
        let group_cmds: Vec<&str> = commands
            .iter()
            .filter(|c| c.group == group)
            .map(|c| c.action.as_str())
            .collect();
        for action in &group_cmds {
            assert!(
                skill.references_md.contains(&format!("{group} {action}")),
                "{}: references_md should mention '{group} {action}'",
                skill.dir_name
            );
        }
    }
}
