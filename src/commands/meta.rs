use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::cli::OutputFormat;
use crate::output;

/// Contract version — additive-only within a major version.
const CONTRACT_VERSION: &str = "1";

// ---------------------------------------------------------------------------
// Contract types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ContractList {
    contract_version: &'static str,
    total: usize,
    commands: Vec<CommandSummary>,
}

#[derive(Serialize)]
struct CommandSummary {
    command: String,
    domain: String,
    synopsis: String,
}

#[derive(Serialize)]
pub struct CommandContract {
    contract_version: &'static str,
    command: String,
    domain: String,
    synopsis: String,
    flags: Vec<FlagContract>,
    global_flags: Vec<FlagContract>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    constraints: Vec<FlagConstraint>,
    output: OutputContract,
    exit_codes: BTreeMap<String, String>,
    supports_dry_run: bool,
    is_mutation: bool,
}

#[derive(Serialize, Clone)]
pub struct FlagConstraint {
    #[serde(rename = "type")]
    constraint_type: &'static str,
    flags: Vec<String>,
    description: String,
}

#[derive(Serialize, Clone)]
pub struct FlagContract {
    name: String,
    #[serde(rename = "type")]
    flag_type: String,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<String>,
}

#[derive(Serialize)]
struct OutputContract {
    formats: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// `dcx meta commands` — list all commands with domain and synopsis.
pub fn run_commands(app: &clap::Command, format: &OutputFormat) -> Result<()> {
    let contracts = collect_all(app);
    let list = ContractList {
        contract_version: CONTRACT_VERSION,
        total: contracts.len(),
        commands: contracts
            .into_iter()
            .map(|c| CommandSummary {
                command: c.command,
                domain: c.domain,
                synopsis: c.synopsis,
            })
            .collect(),
    };
    match format {
        OutputFormat::Json => output::render(&list, format),
        OutputFormat::Table | OutputFormat::Text => {
            let cols = vec!["Command".into(), "Domain".into(), "Synopsis".into()];
            let rows: Vec<Vec<String>> = list
                .commands
                .iter()
                .map(|c| vec![c.command.clone(), c.domain.clone(), c.synopsis.clone()])
                .collect();
            output::render_rows_as_table(&cols, &rows)
        }
    }
}

/// `dcx meta describe <path>` — full contract for one command.
pub fn run_describe(app: &clap::Command, path: &[String], format: &OutputFormat) -> Result<()> {
    if path.is_empty() {
        bail!("Provide a command path, e.g.: dcx meta describe analytics evaluate");
    }
    let target = format!("dcx {}", path.join(" "));
    let contracts = collect_all(app);
    match contracts.into_iter().find(|c| c.command == target) {
        Some(contract) => match format {
            OutputFormat::Json => output::render(&contract, format),
            OutputFormat::Table | OutputFormat::Text => {
                print_describe_text(&contract);
                Ok(())
            }
        },
        None => bail!("Unknown command: {target}"),
    }
}

/// `dcx meta gemini-tools` — generate Gemini manifest from the command contract.
pub fn run_gemini_tools(app: &clap::Command, format: &OutputFormat) -> Result<()> {
    let contracts = collect_all(app);
    let manifest = generate_gemini_manifest(&contracts);
    match format {
        OutputFormat::Json => output::render(&manifest, format),
        OutputFormat::Table | OutputFormat::Text => {
            // Pretty-print tool listing
            println!(
                "{} – {} tools (v{})",
                manifest.display_name,
                manifest.tools.len(),
                manifest.version
            );
            println!();
            for tool in &manifest.tools {
                println!("  {} – {}", tool.name, tool.description);
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Gemini manifest generation from contract
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct GeminiManifest {
    name: String,
    display_name: String,
    description: String,
    version: String,
    homepage: String,
    tools: Vec<GeminiTool>,
}

#[derive(serde::Serialize)]
struct GeminiTool {
    name: String,
    description: String,
    parameters: serde_json::Value,
    command: String,
}

/// Commands to include in the Gemini manifest, in presentation order.
/// Each entry is (command path, optional tool name override).
/// A tool name override of `None` means derive from the command path.
const GEMINI_TOOL_ALLOWLIST: &[(&str, Option<&str>)] = &[
    ("dcx jobs query", None),
    ("dcx datasets list", None),
    ("dcx datasets get", None),
    ("dcx tables list", None),
    ("dcx tables get", None),
    ("dcx routines list", None),
    ("dcx models list", None),
    ("dcx analytics doctor", None),
    ("dcx analytics evaluate", None),
    ("dcx analytics get-trace", None),
    ("dcx analytics list-traces", None),
    ("dcx analytics insights", None),
    ("dcx analytics drift", None),
    ("dcx analytics distribution", None),
    ("dcx analytics hitl-metrics", None),
    ("dcx ca ask", None),
    ("dcx ca ask", Some("dcx_ca_ask_profile")), // profile variant
    ("dcx spanner instances list", None),
    ("dcx spanner databases list", None),
    ("dcx spanner schema describe", None),
    ("dcx alloydb clusters list", None),
    ("dcx alloydb schema describe", None),
    ("dcx cloudsql instances list", None),
    ("dcx cloudsql schema describe", None),
    ("dcx looker instances list", None),
    ("dcx looker explores list", None),
    ("dcx profiles list", None),
    ("dcx profiles validate", None),
];

/// Global flags to promote into tool parameters (when the command reads them).
const PROMOTED_GLOBALS: &[&str] = &[
    "--project-id",
    "--dataset-id",
    "--location",
    // --format is intentionally excluded: every Gemini tool hardcodes --format json
    // so the agent cannot override it.
    "--profile",
];

fn generate_gemini_manifest(contracts: &[CommandContract]) -> GeminiManifest {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let mut tools = Vec::new();

    for (cmd_path, name_override) in GEMINI_TOOL_ALLOWLIST {
        let contract = match contracts.iter().find(|c| c.command == *cmd_path) {
            Some(c) => c,
            None => continue,
        };

        let is_profile_variant =
            name_override.is_some() && name_override.unwrap().contains("profile");

        let tool = if is_profile_variant {
            build_profile_variant_tool(contract, name_override.unwrap())
        } else {
            build_gemini_tool(contract, name_override.as_ref())
        };

        tools.push(tool);
    }

    GeminiManifest {
        name: "dcx".to_string(),
        display_name: "dcx \u{2013} Agent-Native Data Cloud CLI".to_string(),
        description: "Data Cloud CLI optimized for AI agents. Provides query execution, \
            dataset/table/routine/model inspection, agent analytics, and conversational \
            analytics across BigQuery, Looker, AlloyDB, Spanner, and Cloud SQL."
            .to_string(),
        version,
        homepage: "https://github.com/bigquery/dcx".to_string(),
        tools,
    }
}

fn build_gemini_tool(contract: &CommandContract, name_override: Option<&&str>) -> GeminiTool {
    let tool_name = match name_override {
        Some(n) => n.to_string(),
        None => contract
            .command
            .replace("dcx ", "dcx_")
            .replace([' ', '-'], "_"),
    };

    let mut params = serde_json::Map::new();
    let mut cmd_parts = vec![contract.command.clone()];

    // Add command-specific flags as parameters.
    // Every advertised parameter must appear in the command template so
    // Gemini can actually substitute the value when the agent provides it.
    for flag in &contract.flags {
        if flag.name == "--dry-run" || flag.name == "--yes" {
            continue; // internal flags, not for Gemini tools
        }
        let param = flag_to_gemini_param(flag);
        let param_name = flag.name.trim_start_matches("--").replace('-', "_");
        cmd_parts.push(format!("{} {{{}}}", flag.name, param_name));
        params.insert(param_name, param);
    }

    // Promote relevant global flags.
    for gflag in &contract.global_flags {
        if !PROMOTED_GLOBALS.contains(&gflag.name.as_str()) {
            continue;
        }
        if gflag.name == "--profile" {
            continue; // handled separately in profile variant
        }
        let param = flag_to_gemini_param(gflag);
        let param_name = gflag.name.trim_start_matches("--").replace('-', "_");
        cmd_parts.push(format!("{} {{{}}}", gflag.name, param_name));
        params.insert(param_name, param);
    }

    // Always end with --format json.
    cmd_parts.push("--format json".to_string());

    GeminiTool {
        name: tool_name,
        description: contract.synopsis.clone(),
        parameters: serde_json::Value::Object(params),
        command: cmd_parts.join(" "),
    }
}

/// Build the `dcx_ca_ask_profile` variant: uses --profile instead of --agent/--tables.
fn build_profile_variant_tool(contract: &CommandContract, tool_name: &str) -> GeminiTool {
    let mut params = serde_json::Map::new();

    // The profile variant takes --profile (required) and question (positional).
    // Find the question flag from the command flags.
    for flag in &contract.flags {
        if flag.name == "--agent" || flag.name == "--tables" {
            continue; // excluded in profile variant
        }
        let param = flag_to_gemini_param(flag);
        let param_name = flag.name.trim_start_matches("--").replace('-', "_");
        params.insert(param_name, param);
    }

    // Add --profile as required.
    params.insert(
        "profile".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "Path to a CA source profile YAML file",
            "required": true
        }),
    );

    GeminiTool {
        name: tool_name.to_string(),
        description: "Ask a natural language question over a Data Cloud source \
            (Looker, AlloyDB, Spanner, Cloud SQL) using a YAML profile."
            .to_string(),
        parameters: serde_json::Value::Object(params),
        command: "dcx ca ask {question} --profile {profile} --format json".to_string(),
    }
}

fn flag_to_gemini_param(flag: &FlagContract) -> serde_json::Value {
    let mut param = serde_json::Map::new();

    let param_type = match flag.flag_type.as_str() {
        "boolean" => "boolean",
        _ => "string",
    };
    param.insert("type".to_string(), serde_json::json!(param_type));
    param.insert(
        "description".to_string(),
        serde_json::json!(flag.description),
    );

    if flag.required {
        param.insert("required".to_string(), serde_json::json!(true));
    }
    if let Some(ref default) = flag.default {
        // Parse numbers for numeric defaults.
        if let Ok(n) = default.parse::<f64>() {
            param.insert("default".to_string(), serde_json::json!(n));
        } else if default == "true" || default == "false" {
            param.insert("default".to_string(), serde_json::json!(default == "true"));
        } else {
            param.insert("default".to_string(), serde_json::json!(default));
        }
    }

    serde_json::Value::Object(param)
}

// ---------------------------------------------------------------------------
// Text renderer for describe
// ---------------------------------------------------------------------------

fn print_describe_text(c: &CommandContract) {
    println!("{}", c.command);
    println!("  Domain:   {}", c.domain);
    println!("  Synopsis: {}", c.synopsis);
    println!();

    if !c.flags.is_empty() {
        println!("  Flags:");
        for f in &c.flags {
            print_flag(f);
        }
        println!();
    }

    if !c.global_flags.is_empty() {
        println!("  Global flags:");
        for f in &c.global_flags {
            print_flag(f);
        }
        println!();
    }

    if !c.constraints.is_empty() {
        println!("  Constraints:");
        for con in &c.constraints {
            println!(
                "    [{}] {} — {}",
                con.constraint_type,
                con.flags.join(", "),
                con.description
            );
        }
        println!();
    }

    if c.output.formats.is_empty() {
        println!("  Output formats: (none — native output)");
    } else {
        println!("  Output formats: {}", c.output.formats.join(", "));
    }
    println!();

    println!(
        "  Dry run:  {}",
        if c.supports_dry_run { "yes" } else { "no" }
    );
    println!("  Mutation: {}", if c.is_mutation { "yes" } else { "no" });
    println!();

    println!("  Exit codes:");
    for (code, desc) in &c.exit_codes {
        println!("    {code}: {desc}");
    }
}

fn print_flag(f: &FlagContract) {
    let req = if f.required { " (required)" } else { "" };
    let def = f
        .default
        .as_ref()
        .map(|d| format!(" [default: {d}]"))
        .unwrap_or_default();
    let env = f
        .env
        .as_ref()
        .map(|e| format!(" [env: {e}]"))
        .unwrap_or_default();
    println!("    {} <{}>{}{}{}", f.name, f.flag_type, req, def, env);
    if !f.description.is_empty() {
        println!("      {}", f.description);
    }
    if let Some(vals) = &f.values {
        println!("      values: {}", vals.join(", "));
    }
}

// ---------------------------------------------------------------------------
// Introspection engine
// ---------------------------------------------------------------------------

fn collect_all(app: &clap::Command) -> Vec<CommandContract> {
    let global_flags = extract_global_flags(app);
    let mut contracts = Vec::new();
    walk_commands(app, &[], &global_flags, &mut contracts);
    contracts
}

fn walk_commands(
    cmd: &clap::Command,
    prefix: &[&str],
    global_flags: &[FlagContract],
    out: &mut Vec<CommandContract>,
) {
    let subs: Vec<_> = cmd
        .get_subcommands()
        .filter(|s| {
            let name = s.get_name();
            name != "help" && name != "version"
        })
        .collect();

    if subs.is_empty() && !prefix.is_empty() {
        // Leaf command — extract contract.
        out.push(extract_contract(cmd, prefix, global_flags));
    } else {
        for sub in subs {
            let mut new_prefix = prefix.to_vec();
            new_prefix.push(sub.get_name());
            walk_commands(sub, &new_prefix, global_flags, out);
        }
    }
}

fn extract_contract(
    cmd: &clap::Command,
    path: &[&str],
    all_global_flags: &[FlagContract],
) -> CommandContract {
    let command = format!("dcx {}", path.join(" "));
    let domain = infer_domain(path);
    let synopsis = cmd.get_about().map(|s| s.to_string()).unwrap_or_default();
    let flags = extract_flags(cmd);
    let behavior = runtime_behavior(path);

    // Check if this dynamic command supports pagination (marked by _paginated arg).
    let is_paginated = cmd
        .get_arguments()
        .any(|a| a.get_id().as_str() == "_paginated");

    // Detect --dry-run support from the clap arg tree.
    let supports_dry_run = cmd
        .get_arguments()
        .any(|a| a.get_id().as_str() == "dry_run" || a.get_id().as_str() == "dry-run");

    // Only include global flags that this command actually reads.
    // Pagination flags are added only for commands that actually support them.
    let relevant_globals: Vec<FlagContract> = all_global_flags
        .iter()
        .filter(|f| {
            if f.name == "--page-token" || f.name == "--page-all" {
                return is_paginated;
            }
            behavior.relevant_globals.iter().any(|&g| f.name == g)
        })
        .cloned()
        .collect();

    CommandContract {
        contract_version: CONTRACT_VERSION,
        command,
        domain,
        synopsis,
        flags,
        global_flags: relevant_globals,
        constraints: behavior.constraints,
        output: OutputContract {
            formats: behavior.formats.iter().map(|s| s.to_string()).collect(),
        },
        exit_codes: behavior.exit_codes,
        supports_dry_run,
        is_mutation: behavior.is_mutation,
    }
}

// ---------------------------------------------------------------------------
// Flag extraction
// ---------------------------------------------------------------------------

fn extract_global_flags(app: &clap::Command) -> Vec<FlagContract> {
    app.get_arguments()
        .filter(|a| {
            let id = a.get_id().as_str();
            id != "help" && id != "version" && id != "_paginated"
        })
        .map(arg_to_flag)
        .collect()
}

fn extract_flags(cmd: &clap::Command) -> Vec<FlagContract> {
    cmd.get_arguments()
        .filter(|a| {
            let id = a.get_id().as_str();
            id != "help" && id != "version" && id != "_paginated"
        })
        .map(arg_to_flag)
        .collect()
}

fn arg_to_flag(a: &clap::Arg) -> FlagContract {
    let name = match a.get_long() {
        Some(long) => format!("--{long}"),
        None => a.get_id().as_str().to_string(),
    };

    // Detect boolean flags first — clap reports possible_values ["true","false"]
    // for SetTrue/SetFalse actions, which we want to surface as "boolean" not "enum".
    let is_bool = matches!(
        a.get_action(),
        clap::ArgAction::SetTrue | clap::ArgAction::SetFalse
    );

    let (flag_type, values) = if is_bool {
        ("boolean".to_string(), None)
    } else {
        let possible_values: Vec<String> = a
            .get_possible_values()
            .iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_string())
            .collect();
        if !possible_values.is_empty() {
            ("enum".to_string(), Some(possible_values))
        } else {
            ("string".to_string(), None)
        }
    };

    let default = a
        .get_default_values()
        .first()
        .map(|v| v.to_string_lossy().to_string());

    let description = a.get_help().map(|s| s.to_string()).unwrap_or_default();

    let env = a.get_env().map(|s| s.to_string_lossy().to_string());

    FlagContract {
        name,
        flag_type,
        required: a.is_required_set(),
        default,
        description,
        values,
        env,
    }
}

// ---------------------------------------------------------------------------
// Domain and exit-code mapping
// ---------------------------------------------------------------------------

fn infer_domain(path: &[&str]) -> String {
    if path.is_empty() {
        return "unknown".to_string();
    }
    match path[0] {
        "analytics" => "analytics",
        "ca" => "ca",
        "jobs" | "datasets" | "tables" | "routines" | "models" => "bigquery",
        "spanner" => "spanner",
        "alloydb" => "alloydb",
        "cloudsql" => "cloudsql",
        "looker" => "looker",
        "profiles" => "profiles",
        "auth" => "auth",
        "meta" => "meta",
        "generate-skills" | "completions" => "utility",
        _ => "unknown",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Runtime behavior — derived from actual main.rs error-handler routing
// ---------------------------------------------------------------------------

/// All global flags relevant to data commands (jobs, ca, analytics, dynamic).
const DATA_GLOBALS: &[&str] = &[
    "--project-id",
    "--dataset-id",
    "--location",
    "--table",
    "--format",
    "--token",
    "--credentials-file",
    "--sanitize",
];

/// Global flags relevant to namespace helpers (profile-based, no project/dataset).
const HELPER_GLOBALS: &[&str] = &["--format", "--token", "--credentials-file", "--sanitize"];

/// Global flags relevant to auth commands.
const AUTH_GLOBALS: &[&str] = &["--token", "--credentials-file"];

/// Global flags relevant to auth check / profiles test (auth + format).
const AUTH_FORMAT_GLOBALS: &[&str] = &["--format", "--token", "--credentials-file"];

struct RuntimeBehavior {
    /// Output formats this command actually supports.
    formats: Vec<&'static str>,
    /// Exit codes this command can produce, per actual error handlers.
    exit_codes: BTreeMap<String, String>,
    /// Which global flag names this command actually reads.
    relevant_globals: &'static [&'static str],
    /// Flag relationship constraints (mutual exclusion, one-of-required).
    constraints: Vec<FlagConstraint>,
    /// Whether this command mutates state (creates, updates, or deletes resources).
    is_mutation: bool,
}

fn exit_codes(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Map each leaf command to its actual runtime behavior based on the error
/// handler that routes it in main.rs.
fn runtime_behavior(path: &[&str]) -> RuntimeBehavior {
    let first = path.first().copied().unwrap_or("");
    let second = path.get(1).copied().unwrap_or("");

    match first {
        // ── completions: shell-native output, never errors ──────────
        "completions" => RuntimeBehavior {
            formats: vec![],
            exit_codes: exit_codes(&[("0", "success")]),
            relevant_globals: &[],
            constraints: vec![],
            is_mutation: false,
        },

        // ── auth status: always exits 0 (reports status to stderr) ──
        "auth" if second == "status" => RuntimeBehavior {
            formats: vec![],
            exit_codes: exit_codes(&[("0", "success")]),
            relevant_globals: AUTH_GLOBALS,
            constraints: vec![],
            is_mutation: false,
        },

        // ── auth check: structured preflight, exit 0/3 ────────────
        "auth" if second == "check" => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[("0", "success"), ("3", "authentication error")]),
            relevant_globals: AUTH_FORMAT_GLOBALS,
            constraints: vec![],
            is_mutation: false,
        },

        // ── auth login/logout: no structured output, exit 0/3 ───────
        "auth" => RuntimeBehavior {
            formats: vec![],
            exit_codes: exit_codes(&[("0", "success"), ("3", "authentication error")]),
            relevant_globals: &[],
            constraints: vec![],
            is_mutation: second == "logout",
        },

        // ── profiles test: structural + auth verification, exit 0/1 ──
        "profiles" if second == "test" => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[("0", "success"), ("1", "validation or auth error")]),
            relevant_globals: AUTH_FORMAT_GLOBALS,
            constraints: vec![],
            is_mutation: false,
        },

        // ── utility / admin: format-only, early return with exit 1 ──
        "generate-skills" | "profiles" | "meta" => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[("0", "success"), ("1", "error")]),
            relevant_globals: &["--format"],
            constraints: vec![],
            is_mutation: false,
        },

        // ── analytics evaluate / drift: SDK-aligned exit codes ──────
        "analytics" if second == "evaluate" || second == "drift" => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[
                ("0", "success"),
                ("1", "evaluation failure (with --exit-code)"),
                ("2", "infrastructure error"),
            ]),
            relevant_globals: DATA_GLOBALS,
            constraints: vec![],
            is_mutation: false,
        },

        // ── analytics get-trace: one-of-required constraint ─────────
        "analytics" if second == "get-trace" => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[
                ("0", "success"),
                ("1", "validation error"),
                ("2", "infrastructure error"),
                ("3", "authentication error"),
                ("4", "not found"),
            ]),
            relevant_globals: DATA_GLOBALS,
            constraints: vec![FlagConstraint {
                constraint_type: "one_of_required",
                flags: vec!["--session-id".into(), "--trace-id".into()],
                description: "Provide --session-id or --trace-id".into(),
            }],
            is_mutation: false,
        },

        // ── ca ask: --profile is mutually exclusive with --agent/--tables
        "ca" if second == "ask" => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[
                ("0", "success"),
                ("1", "validation error"),
                ("2", "infrastructure error"),
                ("3", "authentication error"),
                ("4", "not found"),
            ]),
            relevant_globals: DATA_GLOBALS,
            constraints: vec![FlagConstraint {
                constraint_type: "mutually_exclusive",
                flags: vec!["--profile".into(), "--agent".into(), "--tables".into()],
                description: "--profile cannot be combined with --agent or --tables".into(),
            }],
            is_mutation: false,
        },

        // ── namespace helpers: profile-based ──────────────────────
        _ if is_namespace_helper(path) => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[
                ("0", "success"),
                ("2", "infrastructure error"),
                ("3", "authentication error"),
                ("4", "not found"),
            ]),
            relevant_globals: HELPER_GLOBALS,
            constraints: vec![],
            is_mutation: false,
        },

        // ── all other data commands: general handler ────────────────
        // Includes: jobs, ca (non-ask), analytics (non-evaluate/drift/get-trace), dynamic
        // Pagination flags (--page-token, --page-all) are added by extract_contract
        // based on the _paginated marker, not by runtime_behavior.
        _ => RuntimeBehavior {
            formats: vec!["json", "table", "text"],
            exit_codes: exit_codes(&[
                ("0", "success"),
                ("1", "validation error"),
                ("2", "infrastructure error"),
                ("3", "authentication error"),
                ("4", "not found"),
                ("5", "conflict / already exists"),
            ]),
            relevant_globals: DATA_GLOBALS,
            constraints: vec![],
            is_mutation: is_mutation_command(path),
        },
    }
}

/// Detect whether a command path represents an unconditional mutation.
///
/// Commands that only mutate conditionally (e.g. categorical-eval with
/// --persist) are not listed here — `is_mutation` should reflect the
/// default behaviour, not an optional flag.
fn is_mutation_command(path: &[&str]) -> bool {
    let first = path.first().copied().unwrap_or("");
    let second = path.get(1).copied().unwrap_or("");
    let third = path.get(2).copied().unwrap_or("");

    matches!(
        (first, second, third),
        ("ca", "create-agent", _)
            | ("ca", "add-verified-query", _)
            | ("analytics", "views", "create" | "create-all")
            | ("analytics", "categorical-views", _)
    )
}

/// Namespace helper commands that are routed through
/// `try_run_namespace_helper` with an early return (exit 0 or 1).
fn is_namespace_helper(path: &[&str]) -> bool {
    if path.len() < 3 {
        return false;
    }
    matches!(
        (path[0], path[1], path[2]),
        ("spanner", "schema", "describe")
            | ("alloydb", "schema", "describe")
            | ("alloydb", "databases", "list")
            | ("cloudsql", "schema", "describe")
            | ("looker", "explores", "list")
            | ("looker", "explores", "get")
            | ("looker", "dashboards", "list")
            | ("looker", "dashboards", "get")
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_app() -> clap::Command {
        clap::Command::new("dcx")
            .arg(
                clap::Arg::new("project_id")
                    .long("project-id")
                    .global(true)
                    .env("DCX_PROJECT")
                    .help("GCP project ID"),
            )
            .arg(
                clap::Arg::new("format")
                    .long("format")
                    .global(true)
                    .default_value("json")
                    .value_parser(["json", "table", "text"])
                    .help("Output format"),
            )
            .arg(
                clap::Arg::new("token")
                    .long("token")
                    .global(true)
                    .env("DCX_TOKEN")
                    .help("Bearer token"),
            )
            .arg(
                clap::Arg::new("credentials_file")
                    .long("credentials-file")
                    .global(true)
                    .env("DCX_CREDENTIALS_FILE")
                    .help("Path to service account credentials JSON file"),
            )
            .subcommand(clap::Command::new("datasets").subcommand(
                clap::Command::new("list").about("Lists all datasets in the specified project"),
            ))
            .subcommand(
                clap::Command::new("jobs").subcommand(
                    clap::Command::new("query")
                        .about("Execute a SQL query")
                        .arg(clap::Arg::new("query").long("query").required(true))
                        .arg(
                            clap::Arg::new("dry_run")
                                .long("dry-run")
                                .action(clap::ArgAction::SetTrue),
                        ),
                ),
            )
            .subcommand(
                clap::Command::new("completions")
                    .about("Generate shell completion scripts")
                    .arg(clap::Arg::new("shell").required(true)),
            )
            .subcommand(
                clap::Command::new("auth")
                    .subcommand(clap::Command::new("login").about("Authenticate with Google OAuth"))
                    .subcommand(
                        clap::Command::new("check").about(
                            "Preflight credential check (structured output for CI / agents)",
                        ),
                    ),
            )
            .subcommand(
                clap::Command::new("profiles")
                    .subcommand(clap::Command::new("list").about("List all discoverable profiles"))
                    .subcommand(
                        clap::Command::new("test")
                            .about("Validate structure and test authentication (network call)"),
                    ),
            )
            .subcommand(
                clap::Command::new("ca")
                    .subcommand(
                        clap::Command::new("ask")
                            .about("Ask a natural language question via Conversational Analytics")
                            .arg(clap::Arg::new("question").required(true))
                            .arg(clap::Arg::new("profile").long("profile"))
                            .arg(clap::Arg::new("agent").long("agent"))
                            .arg(clap::Arg::new("tables").long("tables").value_delimiter(',')),
                    )
                    .subcommand(
                        clap::Command::new("create-agent")
                            .about("Create a new data agent")
                            .arg(clap::Arg::new("name").long("name").required(true))
                            .arg(
                                clap::Arg::new("dry_run")
                                    .long("dry-run")
                                    .action(clap::ArgAction::SetTrue),
                            ),
                    ),
            )
            .subcommand(
                clap::Command::new("analytics")
                    .subcommand(
                        clap::Command::new("evaluate")
                            .about("Evaluate agent sessions against a threshold")
                            .arg(
                                clap::Arg::new("evaluator")
                                    .long("evaluator")
                                    .required(true)
                                    .value_parser(["latency", "error-rate"])
                                    .help("Evaluator type"),
                            )
                            .arg(
                                clap::Arg::new("threshold")
                                    .long("threshold")
                                    .required(true)
                                    .help("Pass/fail threshold"),
                            )
                            .arg(
                                clap::Arg::new("exit-code")
                                    .long("exit-code")
                                    .action(clap::ArgAction::SetTrue)
                                    .help("Return exit code 1 on failure"),
                            ),
                    )
                    .subcommand(
                        clap::Command::new("get-trace")
                            .about("Retrieve a session trace")
                            .arg(clap::Arg::new("session-id").long("session-id"))
                            .arg(clap::Arg::new("trace-id").long("trace-id")),
                    ),
            )
    }

    #[test]
    fn collect_all_finds_leaf_commands() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let paths: Vec<&str> = contracts.iter().map(|c| c.command.as_str()).collect();
        assert!(paths.contains(&"dcx analytics evaluate"));
        assert!(paths.contains(&"dcx datasets list"));
        assert!(paths.contains(&"dcx completions"));
        assert!(paths.contains(&"dcx auth login"));
    }

    #[test]
    fn analytics_evaluate_has_sdk_exit_codes() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let eval = contracts
            .iter()
            .find(|c| c.command == "dcx analytics evaluate")
            .unwrap();
        assert_eq!(
            eval.exit_codes.get("1").unwrap(),
            "evaluation failure (with --exit-code)"
        );
        assert_eq!(eval.exit_codes.get("2").unwrap(), "infrastructure error");
    }

    #[test]
    fn data_commands_have_exit_2() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let ds = contracts
            .iter()
            .find(|c| c.command == "dcx datasets list")
            .unwrap();
        assert_eq!(ds.exit_codes.get("1").unwrap(), "validation error");
        assert_eq!(
            ds.exit_codes.get("2").unwrap(),
            "infrastructure error",
            "data commands should advertise exit 2"
        );
        assert!(
            ds.exit_codes.contains_key("3"),
            "data commands should advertise exit 3 (auth)"
        );
        assert!(
            ds.exit_codes.contains_key("4"),
            "data commands should advertise exit 4 (not found)"
        );
        assert!(
            ds.exit_codes.contains_key("5"),
            "data commands should advertise exit 5 (conflict)"
        );
    }

    #[test]
    fn completions_has_no_format_support() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let comp = contracts
            .iter()
            .find(|c| c.command == "dcx completions")
            .unwrap();
        assert!(
            comp.output.formats.is_empty(),
            "completions should not advertise json/table/text"
        );
        assert!(
            comp.global_flags.is_empty(),
            "completions should have no global flags"
        );
        assert!(!comp.exit_codes.contains_key("2"));
    }

    #[test]
    fn auth_login_has_no_format_support() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let login = contracts
            .iter()
            .find(|c| c.command == "dcx auth login")
            .unwrap();
        assert!(
            login.output.formats.is_empty(),
            "auth login should not advertise json/table/text"
        );
        assert!(
            login.global_flags.is_empty(),
            "auth login should have no global flags"
        );
        assert!(
            login.exit_codes.contains_key("3"),
            "auth login can exit 3 (authentication error)"
        );
    }

    #[test]
    fn auth_status_always_exits_zero() {
        let app = sample_app();
        // Add auth status to the sample app
        let app = app.mut_subcommand("auth", |auth| {
            auth.subcommand(
                clap::Command::new("status").about("Show current authentication status"),
            )
        });
        let contracts = collect_all(&app);
        let status = contracts
            .iter()
            .find(|c| c.command == "dcx auth status")
            .unwrap();
        assert!(
            !status.exit_codes.contains_key("1"),
            "auth status always returns Ok — should not advertise exit 1"
        );
        assert_eq!(status.exit_codes.get("0").unwrap(), "success");
        // auth status reads --token and --credentials-file
        let global_names: Vec<&str> = status
            .global_flags
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert!(global_names.contains(&"--token"));
    }

    #[test]
    fn profiles_has_format_only_globals() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let prof = contracts
            .iter()
            .find(|c| c.command == "dcx profiles list")
            .unwrap();
        let global_names: Vec<&str> = prof.global_flags.iter().map(|f| f.name.as_str()).collect();
        assert!(
            global_names.contains(&"--format"),
            "profiles should use --format"
        );
        assert!(
            !global_names.contains(&"--project-id"),
            "profiles should not include --project-id"
        );
        assert!(
            !global_names.contains(&"--token"),
            "profiles should not include --token"
        );
        assert!(!prof.exit_codes.contains_key("2"));
    }

    #[test]
    fn domain_inference() {
        assert_eq!(infer_domain(&["analytics", "evaluate"]), "analytics");
        assert_eq!(infer_domain(&["datasets", "list"]), "bigquery");
        assert_eq!(infer_domain(&["spanner", "instances", "list"]), "spanner");
        assert_eq!(infer_domain(&["ca", "ask"]), "ca");
        assert_eq!(infer_domain(&["profiles", "list"]), "profiles");
        assert_eq!(infer_domain(&["auth", "login"]), "auth");
        assert_eq!(infer_domain(&["meta", "commands"]), "meta");
    }

    #[test]
    fn flag_extraction_detects_types() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let eval = contracts
            .iter()
            .find(|c| c.command == "dcx analytics evaluate")
            .unwrap();

        let evaluator_flag = eval.flags.iter().find(|f| f.name == "--evaluator").unwrap();
        assert_eq!(evaluator_flag.flag_type, "enum");
        assert!(evaluator_flag.required);
        assert!(evaluator_flag.values.is_some());

        let exit_code_flag = eval.flags.iter().find(|f| f.name == "--exit-code").unwrap();
        assert_eq!(exit_code_flag.flag_type, "boolean");
        assert!(!exit_code_flag.required);
    }

    #[test]
    fn data_commands_include_all_globals() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let eval = contracts
            .iter()
            .find(|c| c.command == "dcx analytics evaluate")
            .unwrap();
        let global_names: Vec<&str> = eval.global_flags.iter().map(|f| f.name.as_str()).collect();
        assert!(global_names.contains(&"--project-id"));
        assert!(global_names.contains(&"--format"));
        assert!(global_names.contains(&"--token"));
    }

    #[test]
    fn env_var_captured() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let eval = contracts
            .iter()
            .find(|c| c.command == "dcx analytics evaluate")
            .unwrap();
        let project_flag = eval
            .global_flags
            .iter()
            .find(|f| f.name == "--project-id")
            .unwrap();
        assert_eq!(project_flag.env.as_deref(), Some("DCX_PROJECT"));
    }

    #[test]
    fn contract_version_is_set() {
        let app = sample_app();
        let contracts = collect_all(&app);
        for c in &contracts {
            assert_eq!(c.contract_version, "1");
        }
    }

    #[test]
    fn describe_rejects_empty_path() {
        let app = sample_app();
        let result = run_describe(&app, &[], &OutputFormat::Json);
        assert!(result.is_err());
    }

    #[test]
    fn describe_rejects_unknown_command() {
        let app = sample_app();
        let result = run_describe(
            &app,
            &["nonexistent".into(), "command".into()],
            &OutputFormat::Json,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown command"));
    }

    #[test]
    fn namespace_helper_detection() {
        assert!(is_namespace_helper(&["spanner", "schema", "describe"]));
        assert!(is_namespace_helper(&["looker", "explores", "list"]));
        assert!(is_namespace_helper(&["alloydb", "databases", "list"]));
        assert!(!is_namespace_helper(&["spanner", "instances", "list"]));
        assert!(!is_namespace_helper(&["datasets", "list"]));
    }

    #[test]
    fn ca_ask_has_mutual_exclusion_constraint() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let ask = contracts
            .iter()
            .find(|c| c.command == "dcx ca ask")
            .unwrap();
        assert_eq!(ask.constraints.len(), 1);
        assert_eq!(ask.constraints[0].constraint_type, "mutually_exclusive");
        assert!(ask.constraints[0].flags.contains(&"--profile".to_string()));
        assert!(ask.constraints[0].flags.contains(&"--agent".to_string()));
        assert!(ask.constraints[0].flags.contains(&"--tables".to_string()));
    }

    #[test]
    fn analytics_get_trace_has_one_of_required_constraint() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let trace = contracts
            .iter()
            .find(|c| c.command == "dcx analytics get-trace")
            .unwrap();
        assert_eq!(trace.constraints.len(), 1);
        assert_eq!(trace.constraints[0].constraint_type, "one_of_required");
        assert!(trace.constraints[0]
            .flags
            .contains(&"--session-id".to_string()));
        assert!(trace.constraints[0]
            .flags
            .contains(&"--trace-id".to_string()));
    }

    #[test]
    fn unconstrained_commands_have_no_constraints() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let ds = contracts
            .iter()
            .find(|c| c.command == "dcx datasets list")
            .unwrap();
        assert!(ds.constraints.is_empty());
    }

    #[test]
    fn is_mutation_detected_from_path() {
        let app = sample_app();
        let contracts = collect_all(&app);

        let create = contracts
            .iter()
            .find(|c| c.command == "dcx ca create-agent")
            .unwrap();
        assert!(
            create.is_mutation,
            "ca create-agent should be marked as mutation"
        );

        let ask = contracts
            .iter()
            .find(|c| c.command == "dcx ca ask")
            .unwrap();
        assert!(!ask.is_mutation, "ca ask should not be marked as mutation");

        let ds = contracts
            .iter()
            .find(|c| c.command == "dcx datasets list")
            .unwrap();
        assert!(
            !ds.is_mutation,
            "datasets list should not be marked as mutation"
        );
    }

    #[test]
    fn auth_check_has_format_and_auth_globals() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let check = contracts
            .iter()
            .find(|c| c.command == "dcx auth check")
            .unwrap();
        let global_names: Vec<&str> = check.global_flags.iter().map(|f| f.name.as_str()).collect();
        assert!(
            global_names.contains(&"--format"),
            "auth check should advertise --format"
        );
        assert!(
            global_names.contains(&"--token"),
            "auth check should advertise --token"
        );
        assert!(
            global_names.contains(&"--credentials-file"),
            "auth check should advertise --credentials-file"
        );
        assert!(
            !global_names.contains(&"--project-id"),
            "auth check should not include --project-id"
        );
        assert!(
            check.exit_codes.contains_key("3"),
            "auth check should advertise exit 3"
        );
        assert!(
            !check.output.formats.is_empty(),
            "auth check should support structured output"
        );
    }

    #[test]
    fn profiles_test_has_format_and_auth_globals() {
        let app = sample_app();
        let contracts = collect_all(&app);
        let test = contracts
            .iter()
            .find(|c| c.command == "dcx profiles test")
            .unwrap();
        let global_names: Vec<&str> = test.global_flags.iter().map(|f| f.name.as_str()).collect();
        assert!(
            global_names.contains(&"--format"),
            "profiles test should advertise --format"
        );
        assert!(
            global_names.contains(&"--token"),
            "profiles test should advertise --token"
        );
        assert!(
            global_names.contains(&"--credentials-file"),
            "profiles test should advertise --credentials-file"
        );
        assert!(
            !global_names.contains(&"--project-id"),
            "profiles test should not include --project-id"
        );
        assert!(
            !test.output.formats.is_empty(),
            "profiles test should support structured output"
        );
    }

    #[test]
    fn supports_dry_run_detected_from_clap() {
        let app = sample_app();
        let contracts = collect_all(&app);

        let query = contracts
            .iter()
            .find(|c| c.command == "dcx jobs query")
            .unwrap();
        assert!(
            query.supports_dry_run,
            "jobs query has --dry-run and should report supports_dry_run=true"
        );

        let ds = contracts
            .iter()
            .find(|c| c.command == "dcx datasets list")
            .unwrap();
        assert!(
            !ds.supports_dry_run,
            "datasets list has no --dry-run and should report supports_dry_run=false"
        );
    }
}
