use serde_json::json;

use dcx::bigquery::client::{QueryResult, TableSchema};
use dcx::cli::EvaluatorType;
use dcx::commands::analytics::doctor::{doctor_report_from_rows, find_missing_columns};
use dcx::commands::analytics::evaluate::eval_result_from_rows;
use dcx::commands::analytics::get_trace::trace_result_from_rows;

fn make_row(pairs: Vec<(&str, serde_json::Value)>) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    map
}

// ═══════════════════════════════════════════════
// Doctor snapshots
// ═══════════════════════════════════════════════

#[test]
fn snapshot_doctor_healthy() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("total_rows", json!("296")),
            ("distinct_sessions", json!("12")),
            ("distinct_agents", json!("2")),
            ("earliest_event", json!("2026-03-01 00:00:00.000 UTC")),
            ("latest_event", json!("2026-03-10 08:30:00.000 UTC")),
            ("minutes_since_last_event", json!("15")),
            ("null_session_ids", json!("0")),
            ("null_agents", json!("0")),
            ("null_event_types", json!("0")),
            ("null_timestamps", json!("0")),
            ("distinct_event_types", json!("5")),
        ])],
        total_rows: 1,
    };
    let columns = vec![
        "session_id".into(),
        "agent".into(),
        "event_type".into(),
        "timestamp".into(),
        "status".into(),
    ];
    let report = doctor_report_from_rows("proj.ds.agent_events", columns, &result).unwrap();
    insta::assert_json_snapshot!("doctor_healthy", report);
}

#[test]
fn snapshot_doctor_error_empty_table() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("total_rows", json!("0")),
            ("distinct_sessions", json!("0")),
            ("distinct_agents", json!("0")),
            ("null_session_ids", json!("0")),
            ("null_agents", json!("0")),
            ("null_event_types", json!("0")),
            ("null_timestamps", json!("0")),
            ("distinct_event_types", json!("0")),
        ])],
        total_rows: 1,
    };
    let report = doctor_report_from_rows("proj.ds.agent_events", vec![], &result).unwrap();
    insta::assert_json_snapshot!("doctor_error_empty", report);
}

#[test]
fn snapshot_doctor_missing_columns() {
    let columns = vec!["session_id".into(), "timestamp".into()];
    let missing = find_missing_columns(&columns);

    // Build the report that run_with_executor would create for missing columns
    let report = dcx::commands::analytics::doctor::DoctorReport {
        status: "error".into(),
        table: "proj.ds.agent_events".into(),
        total_rows: 0,
        distinct_sessions: 0,
        distinct_agents: 0,
        earliest_event: None,
        latest_event: None,
        minutes_since_last_event: None,
        null_checks: dcx::commands::analytics::doctor::NullChecks {
            session_id: 0,
            agent: 0,
            event_type: 0,
            timestamp: 0,
        },
        distinct_event_types: 0,
        columns,
        missing_required_columns: missing.clone(),
        warnings: vec![format!("Missing required columns: {}", missing.join(", "))],
    };
    insta::assert_json_snapshot!("doctor_missing_columns", report);
}

// ═══════════════════════════════════════════════
// Evaluate snapshots
// ═══════════════════════════════════════════════

#[test]
fn snapshot_evaluate_latency_mixed() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("adcp-a20d176b82af")),
                ("agent", json!("sales_agent")),
                ("max_latency_ms", json!("32135")),
                ("avg_latency_ms", json!("15000")),
                ("no_latency_data", json!("false")),
                ("passed", json!("false")),
            ]),
            make_row(vec![
                ("session_id", json!("adcp-affa9b3c1234")),
                ("agent", json!("sales_agent")),
                ("max_latency_ms", json!("1200")),
                ("avg_latency_ms", json!("800")),
                ("no_latency_data", json!("false")),
                ("passed", json!("true")),
            ]),
            make_row(vec![
                ("session_id", json!("adcp-bbb0deadbeef")),
                ("agent", json!("support_agent")),
                ("max_latency_ms", json!("26848")),
                ("avg_latency_ms", json!("12000")),
                ("no_latency_data", json!("false")),
                ("passed", json!("false")),
            ]),
        ],
        total_rows: 3,
    };
    let eval = eval_result_from_rows(&EvaluatorType::Latency, 5000.0, "7d".into(), None, &result);
    insta::assert_json_snapshot!("evaluate_latency_mixed", eval);
}

#[test]
fn snapshot_evaluate_error_rate_all_pass() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s1")),
                ("agent", json!("agent_a")),
                ("total_events", json!("20")),
                ("error_events", json!("0")),
                ("error_rate", json!("0.0")),
                ("passed", json!("true")),
            ]),
            make_row(vec![
                ("session_id", json!("s2")),
                ("agent", json!("agent_a")),
                ("total_events", json!("15")),
                ("error_events", json!("1")),
                ("error_rate", json!("0.0667")),
                ("passed", json!("true")),
            ]),
        ],
        total_rows: 2,
    };
    let eval = eval_result_from_rows(
        &EvaluatorType::ErrorRate,
        0.1,
        "24h".into(),
        Some("agent_a".into()),
        &result,
    );
    insta::assert_json_snapshot!("evaluate_error_rate_all_pass", eval);
}

// ═══════════════════════════════════════════════
// Trace snapshots
// ═══════════════════════════════════════════════

#[test]
fn snapshot_trace_with_events() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("adcp-a20d176b82af")),
                ("agent", json!("yahoo_sales_agent")),
                ("event_type", json!("LLM_REQUEST")),
                ("timestamp", json!("2026-03-05 09:26:59.270 UTC")),
                ("status", json!("OK")),
                ("error_message", serde_json::Value::Null),
                ("latency_ms", serde_json::Value::Null),
                ("content", serde_json::Value::Null),
            ]),
            make_row(vec![
                ("session_id", json!("adcp-a20d176b82af")),
                ("agent", json!("yahoo_sales_agent")),
                ("event_type", json!("LLM_RESPONSE")),
                ("timestamp", json!("2026-03-05 09:27:03.208 UTC")),
                ("status", json!("OK")),
                ("error_message", serde_json::Value::Null),
                ("latency_ms", json!("{\"total_ms\": 3938}")),
                ("content", serde_json::Value::Null),
            ]),
            make_row(vec![
                ("session_id", json!("adcp-a20d176b82af")),
                ("agent", json!("yahoo_sales_agent")),
                ("event_type", json!("INVOCATION_COMPLETED")),
                ("timestamp", json!("2026-03-05 09:27:17.494 UTC")),
                ("status", json!("OK")),
                ("error_message", serde_json::Value::Null),
                ("latency_ms", json!("{\"total_ms\": 32135}")),
                ("content", serde_json::Value::Null),
            ]),
        ],
        total_rows: 3,
    };
    let trace = trace_result_from_rows("adcp-a20d176b82af".into(), &result).unwrap();
    insta::assert_json_snapshot!("trace_with_events", trace);
}

#[test]
fn snapshot_trace_with_error_event() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("err-session-001")),
                ("agent", json!("support_agent")),
                ("event_type", json!("TOOL_CALL")),
                ("timestamp", json!("2026-03-05 10:00:00.000 UTC")),
                ("status", json!("OK")),
                ("error_message", serde_json::Value::Null),
                ("latency_ms", serde_json::Value::Null),
                ("content", serde_json::Value::Null),
            ]),
            make_row(vec![
                ("session_id", json!("err-session-001")),
                ("agent", json!("support_agent")),
                ("event_type", json!("TOOL_ERROR")),
                ("timestamp", json!("2026-03-05 10:00:05.000 UTC")),
                ("status", json!("ERROR")),
                ("error_message", json!("Connection refused to external API")),
                ("latency_ms", serde_json::Value::Null),
                ("content", serde_json::Value::Null),
            ]),
        ],
        total_rows: 2,
    };
    let trace = trace_result_from_rows("err-session-001".into(), &result).unwrap();
    insta::assert_json_snapshot!("trace_with_error", trace);
}

// ═══════════════════════════════════════════════
// SQL builder snapshots
// ═══════════════════════════════════════════════

#[test]
fn snapshot_sql_trace_query() {
    let sql = dcx::commands::analytics::get_trace::build_trace_query(
        "my-project",
        "analytics_ds",
        "agent_events",
        "session-abc-123",
    );
    insta::assert_snapshot!("sql_trace_query", sql);
}

#[test]
fn snapshot_sql_evaluate_latency() {
    let sql = dcx::commands::analytics::evaluate::build_evaluate_query(
        &EvaluatorType::Latency,
        "my-project",
        "analytics_ds",
        "agent_events",
        "INTERVAL 24 HOUR",
        5000.0,
        None,
    );
    insta::assert_snapshot!("sql_evaluate_latency", sql);
}

#[test]
fn snapshot_sql_evaluate_error_rate_with_agent() {
    let sql = dcx::commands::analytics::evaluate::build_evaluate_query(
        &EvaluatorType::ErrorRate,
        "my-project",
        "analytics_ds",
        "agent_events",
        "INTERVAL 7 DAY",
        0.1,
        Some("sales_agent"),
    );
    insta::assert_snapshot!("sql_evaluate_error_rate_with_agent", sql);
}

#[test]
fn snapshot_sql_doctor_columns() {
    let sql = dcx::commands::analytics::doctor::build_columns_query("proj", "ds", "agent_events");
    insta::assert_snapshot!("sql_doctor_columns", sql);
}

#[test]
fn snapshot_sql_doctor_stats() {
    let sql = dcx::commands::analytics::doctor::build_stats_query("proj", "ds", "agent_events");
    insta::assert_snapshot!("sql_doctor_stats", sql);
}

// ═══════════════════════════════════════════════
// Text renderer snapshots
// ═══════════════════════════════════════════════

#[test]
fn snapshot_text_doctor_healthy() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("total_rows", json!("296")),
            ("distinct_sessions", json!("12")),
            ("distinct_agents", json!("2")),
            ("earliest_event", json!("2026-03-01 00:00:00.000 UTC")),
            ("latest_event", json!("2026-03-10 08:30:00.000 UTC")),
            ("minutes_since_last_event", json!("15")),
            ("null_session_ids", json!("0")),
            ("null_agents", json!("0")),
            ("null_event_types", json!("0")),
            ("null_timestamps", json!("0")),
            ("distinct_event_types", json!("5")),
        ])],
        total_rows: 1,
    };
    let columns = vec![
        "session_id".into(),
        "agent".into(),
        "event_type".into(),
        "timestamp".into(),
    ];
    let report = doctor_report_from_rows("proj.ds.agent_events", columns, &result).unwrap();

    let mut buf = String::new();
    dcx::output::text::fmt_doctor(&mut buf, &report);
    insta::assert_snapshot!("text_doctor_healthy", buf);
}

#[test]
fn snapshot_text_evaluate_mixed() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s-fast")),
                ("agent", json!("agent_a")),
                ("max_latency_ms", json!("1200")),
                ("avg_latency_ms", json!("800")),
                ("no_latency_data", json!("false")),
                ("passed", json!("true")),
            ]),
            make_row(vec![
                ("session_id", json!("s-slow")),
                ("agent", json!("agent_a")),
                ("max_latency_ms", json!("32135")),
                ("avg_latency_ms", json!("15000")),
                ("no_latency_data", json!("false")),
                ("passed", json!("false")),
            ]),
        ],
        total_rows: 2,
    };
    let eval = eval_result_from_rows(&EvaluatorType::Latency, 5000.0, "24h".into(), None, &result);

    let mut buf = String::new();
    dcx::output::text::fmt_evaluate(&mut buf, &eval);
    insta::assert_snapshot!("text_evaluate_mixed", buf);
}

#[test]
fn snapshot_text_trace() {
    let trace = dcx::commands::analytics::get_trace::TraceResult {
        session_id: "adcp-a20d176b82af".into(),
        agent: "yahoo_sales_agent".into(),
        event_count: 2,
        started_at: Some("2026-03-05 09:26:59.270 UTC".into()),
        ended_at: Some("2026-03-05 09:27:03.208 UTC".into()),
        has_errors: false,
        events: vec![
            dcx::commands::analytics::get_trace::TraceEvent {
                event_type: "LLM_REQUEST".into(),
                timestamp: "2026-03-05 09:26:59.270 UTC".into(),
                status: Some("OK".into()),
                error_message: None,
                latency_ms: None,
                content: None,
            },
            dcx::commands::analytics::get_trace::TraceEvent {
                event_type: "LLM_RESPONSE".into(),
                timestamp: "2026-03-05 09:27:03.208 UTC".into(),
                status: Some("OK".into()),
                error_message: None,
                latency_ms: Some(json!({"total_ms": 3938})),
                content: None,
            },
        ],
    };

    let mut buf = String::new();
    dcx::output::text::fmt_trace(&mut buf, &trace);
    insta::assert_snapshot!("text_trace", buf);
}

#[test]
fn snapshot_text_query() {
    let columns = vec!["session_id".into(), "agent".into(), "event_type".into()];
    let rows = vec![
        vec!["s1".into(), "agent_a".into(), "LLM_REQUEST".into()],
        vec!["s2".into(), "agent_b".into(), "TOOL_CALL".into()],
    ];
    let mut buf = String::new();
    dcx::output::text::fmt_query(&mut buf, 2, &columns, &rows);
    insta::assert_snapshot!("text_query", buf);
}

// ═══════════════════════════════════════════════
// Table output snapshots
// ═══════════════════════════════════════════════

#[test]
fn snapshot_table_rows() {
    let columns = vec!["session_id".into(), "agent".into(), "event_type".into()];
    let rows = vec![
        vec![
            "adcp-a20d176b82af".into(),
            "sales_agent".into(),
            "LLM_REQUEST".into(),
        ],
        vec![
            "adcp-affa9b3c1234".into(),
            "support_agent".into(),
            "TOOL_CALL".into(),
        ],
    ];
    let table = dcx::output::fmt_rows_as_table(&columns, &rows);
    insta::assert_snapshot!("table_rows", table);
}

#[test]
fn snapshot_table_kv() {
    let mut map = serde_json::Map::new();
    map.insert("status".into(), json!("healthy"));
    map.insert("table".into(), json!("proj.ds.agent_events"));
    map.insert("total_rows".into(), json!(296));
    map.insert("distinct_sessions".into(), json!(12));
    let table = dcx::output::fmt_kv_table(&map);
    insta::assert_snapshot!("table_kv", table);
}

#[test]
fn snapshot_table_doctor_report() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("total_rows", json!("296")),
            ("distinct_sessions", json!("12")),
            ("distinct_agents", json!("2")),
            ("earliest_event", json!("2026-03-01 00:00:00.000 UTC")),
            ("latest_event", json!("2026-03-10 08:30:00.000 UTC")),
            ("minutes_since_last_event", json!("15")),
            ("null_session_ids", json!("0")),
            ("null_agents", json!("0")),
            ("null_event_types", json!("0")),
            ("null_timestamps", json!("0")),
            ("distinct_event_types", json!("5")),
        ])],
        total_rows: 1,
    };
    let columns = vec![
        "session_id".into(),
        "agent".into(),
        "event_type".into(),
        "timestamp".into(),
    ];
    let report = doctor_report_from_rows("proj.ds.agent_events", columns, &result).unwrap();
    let value = serde_json::to_value(&report).unwrap();
    let table = dcx::output::fmt_value_as_table(&value).unwrap();
    insta::assert_snapshot!("table_doctor_report", table);
}

#[test]
fn snapshot_table_evaluate_result() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s-fast")),
                ("agent", json!("agent_a")),
                ("max_latency_ms", json!("1200")),
                ("avg_latency_ms", json!("800")),
                ("no_latency_data", json!("false")),
                ("passed", json!("true")),
            ]),
            make_row(vec![
                ("session_id", json!("s-slow")),
                ("agent", json!("agent_a")),
                ("max_latency_ms", json!("32135")),
                ("avg_latency_ms", json!("15000")),
                ("no_latency_data", json!("false")),
                ("passed", json!("false")),
            ]),
        ],
        total_rows: 2,
    };
    let eval = eval_result_from_rows(
        &dcx::cli::EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        None,
        &result,
    );
    let value = serde_json::to_value(&eval).unwrap();
    let table = dcx::output::fmt_value_as_table(&value).unwrap();
    insta::assert_snapshot!("table_evaluate_result", table);
}

#[test]
fn snapshot_table_trace_events() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s1")),
                ("agent", json!("agent_a")),
                ("event_type", json!("LLM_REQUEST")),
                ("timestamp", json!("2026-03-05 09:26:59.270 UTC")),
                ("status", json!("OK")),
                ("error_message", serde_json::Value::Null),
                ("latency_ms", serde_json::Value::Null),
                ("content", serde_json::Value::Null),
            ]),
            make_row(vec![
                ("session_id", json!("s1")),
                ("agent", json!("agent_a")),
                ("event_type", json!("LLM_RESPONSE")),
                ("timestamp", json!("2026-03-05 09:27:03.208 UTC")),
                ("status", json!("OK")),
                ("error_message", serde_json::Value::Null),
                ("latency_ms", json!("{\"total_ms\": 3938}")),
                ("content", serde_json::Value::Null),
            ]),
        ],
        total_rows: 2,
    };
    let trace = trace_result_from_rows("s1".into(), &result).unwrap();
    let value = serde_json::to_value(&trace).unwrap();
    let table = dcx::output::fmt_value_as_table(&value).unwrap();
    insta::assert_snapshot!("table_trace_events", table);
}

#[test]
fn snapshot_table_query_rows() {
    let value = json!({
        "total_rows": 2,
        "rows": [
            {"session_id": "s1", "agent": "agent_a", "event_type": "LLM_REQUEST"},
            {"session_id": "s2", "agent": "agent_b", "event_type": "TOOL_CALL"}
        ]
    });
    let table = dcx::output::fmt_value_as_table(&value).unwrap();
    insta::assert_snapshot!("table_query_rows", table);
}
