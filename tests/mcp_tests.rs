//! Integration tests for the MCP bridge server.
//!
//! Tests the JSON-RPC protocol over stdio by piping requests to `dcx mcp serve`
//! and validating the responses.

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

fn cargo_bin() -> String {
    let output = Command::new(env!("CARGO"))
        .args(["build", "--quiet"])
        .output()
        .expect("Failed to build");
    assert!(output.status.success(), "Build failed");

    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| String::from("target"));
    format!("{target_dir}/debug/dcx")
}

/// Send a sequence of JSON-RPC requests to the MCP server and return responses.
fn mcp_session(requests: &[Value]) -> Vec<Value> {
    let bin = cargo_bin();
    let mut child = Command::new(&bin)
        .args(["mcp", "serve"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start MCP server");

    let stdin = child.stdin.as_mut().unwrap();
    for req in requests {
        writeln!(stdin, "{}", serde_json::to_string(req).unwrap()).unwrap();
    }
    drop(child.stdin.take()); // Close stdin to signal EOF.

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("Invalid JSON: {e}\n{l}")))
        .collect()
}

// ---------------------------------------------------------------------------
// Protocol tests
// ---------------------------------------------------------------------------

#[test]
fn mcp_initialize_returns_server_info() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    })]);

    assert_eq!(responses.len(), 1);
    let result = &responses[0]["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert!(result["capabilities"]["tools"].is_object());
    assert_eq!(result["serverInfo"]["name"], "dcx");
    assert!(!result["serverInfo"]["version"].as_str().unwrap().is_empty());
}

#[test]
fn mcp_tools_list_returns_expected_tools() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    })]);

    let tools = responses[0]["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Should have at least one tool");

    // Verify each tool has required MCP fields.
    for tool in tools {
        assert!(tool["name"].as_str().is_some(), "Tool missing name: {tool}");
        assert!(
            tool["description"].as_str().is_some(),
            "Tool missing description: {tool}"
        );
        assert!(
            tool["inputSchema"].is_object(),
            "Tool missing inputSchema: {}",
            tool["name"]
        );
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "inputSchema should be type: object"
        );
    }

    // Key differentiated tools are present.
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in &[
        "dcx_analytics_evaluate",
        "dcx_analytics_doctor",
        "dcx_ca_ask",
        "dcx_meta_commands",
        "dcx_profiles_list",
        "dcx_auth_check",
        "dcx_datasets_list",
    ] {
        assert!(
            names.contains(expected),
            "Missing expected tool: {expected}"
        );
    }

    // Non-differentiated domains should not be present.
    for name in &names {
        assert!(
            !name.starts_with("dcx_spanner_"),
            "Spanner tools should not be in MCP: {name}"
        );
        assert!(
            !name.starts_with("dcx_alloydb_"),
            "AlloyDB tools should not be in MCP: {name}"
        );
        assert!(
            !name.starts_with("dcx_cloudsql_"),
            "Cloud SQL tools should not be in MCP: {name}"
        );
        assert!(
            !name.starts_with("dcx_looker_"),
            "Looker tools should not be in MCP: {name}"
        );
    }
}

#[test]
fn mcp_tool_call_dry_run() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "dcx_datasets_list",
            "arguments": {
                "project_id": "mcp-test-proj",
                "dry_run": true
            }
        }
    })]);

    let result = &responses[0]["result"];
    assert_eq!(result["isError"], false);

    let content = result["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(content).unwrap();
    assert_eq!(parsed["dry_run"], true);
    assert_eq!(parsed["method"], "GET");
    assert!(parsed["url"].as_str().unwrap().contains("mcp-test-proj"));
}

#[test]
fn mcp_tool_call_meta_commands() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "dcx_meta_commands",
            "arguments": {}
        }
    })]);

    let result = &responses[0]["result"];
    assert_eq!(result["isError"], false);

    let content = result["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(content).unwrap();
    assert_eq!(parsed["contract_version"], "1");
    assert!(parsed["total"].as_u64().unwrap() > 10);
}

#[test]
fn mcp_unknown_method_returns_error() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/list",
        "params": {}
    })]);

    assert!(responses[0]["error"].is_object());
    assert_eq!(responses[0]["error"]["code"], -32601);
}

#[test]
fn mcp_notification_gets_no_response() {
    // A notification (no id) should not produce a response.
    let responses = mcp_session(&[
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping",
            "params": {}
        }),
    ]);

    // Should get 2 responses (initialize + ping), not 3.
    assert_eq!(
        responses.len(),
        2,
        "Notification should not produce a response"
    );
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[1]["id"], 2);
}

#[test]
fn mcp_ping_responds() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "ping",
        "params": {}
    })]);

    assert_eq!(responses[0]["id"], 42);
    assert!(responses[0]["result"].is_object());
    assert!(responses[0].get("error").is_none());
}

#[test]
fn mcp_tool_input_schema_has_required_flags() {
    let responses = mcp_session(&[serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    })]);

    let tools = responses[0]["result"]["tools"].as_array().unwrap();
    let evaluate = tools
        .iter()
        .find(|t| t["name"] == "dcx_analytics_evaluate")
        .expect("Should have dcx_analytics_evaluate tool");

    let schema = &evaluate["inputSchema"];
    assert!(
        schema["properties"].is_object(),
        "inputSchema should have properties"
    );

    // analytics evaluate has required --evaluator flag.
    let required = schema["required"].as_array().unwrap();
    let required_names: Vec<&str> = required.iter().map(|r| r.as_str().unwrap()).collect();
    assert!(
        required_names.contains(&"evaluator"),
        "evaluator should be required: {:?}",
        required_names
    );
}
