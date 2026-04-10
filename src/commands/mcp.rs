//! MCP (Model Context Protocol) bridge server.
//!
//! Exposes the `dcx` command surface as MCP tools over stdio transport.
//! Tools are generated from the command contract so they stay aligned with
//! the CLI automatically.
//!
//! Protocol: JSON-RPC 2.0 over stdin/stdout (one JSON object per line).

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::process::Command;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::commands::meta::{self, CommandContract, FlagContract};

// ---------------------------------------------------------------------------
// MCP protocol types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// ---------------------------------------------------------------------------
// MCP tool definitions
// ---------------------------------------------------------------------------

/// Domains to expose via MCP. These are the differentiated `dcx` surfaces;
/// generic BigQuery/Spanner/AlloyDB/Cloud SQL CRUD is left to Google-managed
/// MCP servers.
const MCP_DOMAINS: &[&str] = &["analytics", "auth", "bigquery", "ca", "meta", "profiles"];

/// Convert a command path like "dcx analytics evaluate" to an MCP tool name
/// like "dcx_analytics_evaluate".
///
/// Spaces become underscores. Hyphens in subcommand names (e.g. "get-trace")
/// are preserved — they are part of the command identity, not word separators.
fn tool_name(command: &str) -> String {
    command.replace(' ', "_")
}

/// Reverse a tool name back to CLI args.
///
/// "dcx_analytics_get-trace" → ["analytics", "get-trace"]
///
/// We store a mapping from tool name → command path to avoid lossy reverse
/// parsing. This function just strips the "dcx_" prefix and splits on "_",
/// but callers should prefer the lookup table built during tool list generation.
fn tool_name_to_args(tool: &str) -> Vec<String> {
    tool.strip_prefix("dcx_")
        .unwrap_or(tool)
        .split('_')
        .map(|s| s.to_string())
        .collect()
}

/// Build a JSON Schema `inputSchema` from a command contract's flags.
fn build_input_schema(contract: &CommandContract) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for flag in contract.flags.iter().chain(contract.global_flags.iter()) {
        let prop = flag_to_json_schema(flag);
        let key = flag.name.trim_start_matches("--").replace('-', "_");
        if flag.required {
            required.push(Value::String(key.clone()));
        }
        properties.insert(key, prop);
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

/// Convert a single flag contract to a JSON Schema property.
fn flag_to_json_schema(flag: &FlagContract) -> Value {
    let mut prop = serde_json::Map::new();

    match flag.flag_type.as_str() {
        "boolean" => {
            prop.insert("type".into(), json!("boolean"));
        }
        "enum" => {
            prop.insert("type".into(), json!("string"));
            if let Some(ref values) = flag.values {
                prop.insert("enum".into(), json!(values));
            }
        }
        _ => {
            prop.insert("type".into(), json!("string"));
        }
    }

    if !flag.description.is_empty() {
        prop.insert("description".into(), json!(flag.description));
    }
    if let Some(ref default) = flag.default {
        prop.insert("default".into(), json!(default));
    }

    Value::Object(prop)
}

/// Build the list of MCP tools from command contracts, and a reverse lookup
/// from tool name to CLI args.
fn build_tool_list(contracts: &[CommandContract]) -> (Vec<Value>, HashMap<String, Vec<String>>) {
    let mut tools = Vec::new();
    let mut lookup = HashMap::new();

    for c in contracts {
        if !MCP_DOMAINS.contains(&c.domain.as_str()) {
            continue;
        }
        let name = tool_name(&c.command);
        // Command path "dcx analytics evaluate" → args ["analytics", "evaluate"]
        let args: Vec<String> = c
            .command
            .strip_prefix("dcx ")
            .unwrap_or(&c.command)
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        lookup.insert(name.clone(), args);
        tools.push(json!({
            "name": name,
            "description": c.synopsis,
            "inputSchema": build_input_schema(c),
        }));
    }

    (tools, lookup)
}

// ---------------------------------------------------------------------------
// Tool execution
// ---------------------------------------------------------------------------

/// Execute a tool call by running the `dcx` binary as a subprocess.
///
/// This ensures the MCP bridge has exactly the same contract, validation,
/// output schema, and error semantics as the CLI.
fn execute_tool(
    dcx_bin: &str,
    tool_name_str: &str,
    arguments: &HashMap<String, Value>,
    cmd_lookup: &HashMap<String, Vec<String>>,
) -> Result<(bool, String)> {
    // Look up the CLI args for this tool name.
    let cmd_args = match cmd_lookup.get(tool_name_str) {
        Some(args) => args.clone(),
        None => tool_name_to_args(tool_name_str),
    };

    let mut args = cmd_args;

    // Add arguments as flags.
    for (key, value) in arguments {
        let flag = format!("--{}", key.replace('_', "-"));
        match value {
            Value::Bool(true) => {
                args.push(flag);
            }
            Value::Bool(false) => {
                // Omit false boolean flags.
            }
            Value::Null => {
                // Omit null values.
            }
            _ => {
                args.push(flag);
                args.push(value.as_str().unwrap_or(&value.to_string()).to_string());
            }
        }
    }

    // Always request JSON output.
    args.push("--format".to_string());
    args.push("json".to_string());

    let output = Command::new(dcx_bin)
        .args(&args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute dcx: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok((false, stdout))
    } else {
        // Combine stderr error envelope with any stdout content.
        let error_text = if !stderr.is_empty() { stderr } else { stdout };
        Ok((true, error_text))
    }
}

// ---------------------------------------------------------------------------
// MCP server loop
// ---------------------------------------------------------------------------

/// Run the MCP server on stdio.
pub fn run(app: &clap::Command) -> Result<()> {
    let contracts = meta::collect_all(app);
    let (tools, cmd_lookup) = build_tool_list(&contracts);

    // Resolve path to the dcx binary (same binary that's running).
    let dcx_bin = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "dcx".to_string());

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // MCP uses line-delimited JSON-RPC over stdio.
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {e}"),
                        data: None,
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            let resp = JsonRpcResponse {
                jsonrpc: "2.0",
                id: request.id.unwrap_or(Value::Null),
                result: None,
                error: Some(JsonRpcError {
                    code: -32600,
                    message: "Invalid JSON-RPC version".to_string(),
                    data: None,
                }),
            };
            writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
            stdout.flush()?;
            continue;
        }

        let id = request.id.clone().unwrap_or(Value::Null);

        // Notifications (no id) don't get responses.
        if request.id.is_none() {
            // Handle notifications silently (e.g., notifications/initialized).
            continue;
        }

        let response = match request.method.as_str() {
            "initialize" => handle_initialize(id),
            "tools/list" => handle_tools_list(id, &tools),
            "tools/call" => handle_tools_call(id, &request.params, &dcx_bin, &cmd_lookup),
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(json!({})),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        };

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_initialize(id: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "dcx",
                "version": env!("CARGO_PKG_VERSION"),
            }
        })),
        error: None,
    }
}

fn handle_tools_list(id: Value, tools: &[Value]) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}

fn handle_tools_call(
    id: Value,
    params: &Value,
    dcx_bin: &str,
    cmd_lookup: &HashMap<String, Vec<String>>,
) -> JsonRpcResponse {
    let name = params["name"].as_str().unwrap_or("");
    let arguments: HashMap<String, Value> = params["arguments"]
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    match execute_tool(dcx_bin, name, &arguments, cmd_lookup) {
        Ok((is_error, text)) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": text,
                }],
                "isError": is_error,
            })),
            error: None,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32603,
                message: format!("Internal error: {e}"),
                data: None,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_name_conversion() {
        assert_eq!(
            tool_name("dcx analytics evaluate"),
            "dcx_analytics_evaluate"
        );
        assert_eq!(tool_name("dcx meta commands"), "dcx_meta_commands");
        assert_eq!(tool_name("dcx ca ask"), "dcx_ca_ask");
    }

    #[test]
    fn flag_to_schema_string() {
        let flag = FlagContract {
            name: "--project-id".to_string(),
            flag_type: "string".to_string(),
            required: true,
            default: None,
            description: "GCP project ID".to_string(),
            values: None,
            env: Some("DCX_PROJECT".to_string()),
        };
        let schema = flag_to_json_schema(&flag);
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["description"], "GCP project ID");
    }

    #[test]
    fn flag_to_schema_boolean() {
        let flag = FlagContract {
            name: "--dry-run".to_string(),
            flag_type: "boolean".to_string(),
            required: false,
            default: None,
            description: "Preview without executing".to_string(),
            values: None,
            env: None,
        };
        let schema = flag_to_json_schema(&flag);
        assert_eq!(schema["type"], "boolean");
    }

    #[test]
    fn flag_to_schema_enum() {
        let flag = FlagContract {
            name: "--evaluator".to_string(),
            flag_type: "enum".to_string(),
            required: true,
            default: None,
            description: "Evaluator to run".to_string(),
            values: Some(vec!["latency".into(), "error-rate".into()]),
            env: None,
        };
        let schema = flag_to_json_schema(&flag);
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["enum"], json!(["latency", "error-rate"]));
    }

    #[test]
    fn build_tool_list_filters_by_domain() {
        let analytics_contract = CommandContract::new_for_test(
            "dcx analytics evaluate",
            "analytics",
            "Evaluate agent sessions",
        );
        let bigquery_contract =
            CommandContract::new_for_test("dcx datasets list", "bigquery", "List datasets");
        let spanner_contract = CommandContract::new_for_test(
            "dcx spanner instances list",
            "spanner",
            "List Spanner instances",
        );

        let contracts = vec![analytics_contract, bigquery_contract, spanner_contract];
        let (tools, lookup) = build_tool_list(&contracts);

        let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

        // analytics and bigquery are in MCP_DOMAINS; spanner is not.
        assert!(tool_names.contains(&"dcx_analytics_evaluate"));
        assert!(tool_names.contains(&"dcx_datasets_list"));
        assert!(!tool_names.contains(&"dcx_spanner_instances_list"));

        // Lookup table maps tool names to CLI args.
        assert_eq!(
            lookup["dcx_analytics_evaluate"],
            vec!["analytics", "evaluate"]
        );
        assert_eq!(lookup["dcx_datasets_list"], vec!["datasets", "list"]);
        assert!(!lookup.contains_key("dcx_spanner_instances_list"));
    }

    #[test]
    fn initialize_response_has_required_fields() {
        let resp = handle_initialize(json!(1));
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["serverInfo"]["name"], "dcx");
    }

    #[test]
    fn tools_list_returns_tools_array() {
        let tools = vec![json!({"name": "dcx_meta_commands", "description": "List commands"})];
        let resp = handle_tools_list(json!(1), &tools);
        let result = resp.result.unwrap();
        assert_eq!(result["tools"].as_array().unwrap().len(), 1);
    }
}
