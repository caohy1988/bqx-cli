use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use bqx::bigquery::client::{QueryExecutor, QueryRequest, QueryResult, SchemaField, TableSchema};
use bqx::cli::{EvaluatorType, OutputFormat};
use bqx::commands::analytics::doctor::{
    build_columns_query, build_stats_query, columns_from_result, doctor_report_from_rows,
    find_missing_columns,
};
use bqx::commands::analytics::evaluate::{build_evaluate_query, eval_result_from_rows};
use bqx::commands::analytics::get_trace::{build_trace_query, trace_result_from_rows};
use bqx::commands::analytics::list_traces::{build_list_traces_query, traces_from_rows};
use bqx::commands::analytics::views::build_create_view_sql;
use bqx::commands::jobs_query::build_query_request;
use bqx::config::Config;

// ── MockExecutor ──

struct MockExecutor {
    result: QueryResult,
}

impl MockExecutor {
    fn new(
        schema: Vec<(&str, &str)>,
        rows: Vec<serde_json::Map<String, serde_json::Value>>,
    ) -> Self {
        let fields = schema
            .into_iter()
            .map(|(name, field_type)| SchemaField {
                name: name.to_string(),
                field_type: field_type.to_string(),
                mode: None,
            })
            .collect();
        let total_rows = rows.len() as u64;
        Self {
            result: QueryResult {
                schema: TableSchema { fields },
                rows,
                total_rows,
            },
        }
    }

    fn empty(schema: Vec<(&str, &str)>) -> Self {
        Self::new(schema, vec![])
    }
}

#[async_trait]
impl QueryExecutor for MockExecutor {
    async fn query(&self, _project: &str, _req: QueryRequest) -> Result<QueryResult> {
        Ok(QueryResult {
            schema: self.result.schema.clone(),
            rows: self.result.rows.clone(),
            total_rows: self.result.total_rows,
        })
    }
}

fn make_row(pairs: Vec<(&str, serde_json::Value)>) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    map
}

fn test_config(format: OutputFormat) -> Config {
    Config {
        project_id: "test-project".into(),
        dataset_id: Some("test_dataset".into()),
        location: "US".into(),
        table: "agent_events".into(),
        format,
        sanitize_template: None,
    }
}

// ═══════════════════════════════════════════════
// SQL Builder Tests
// ═══════════════════════════════════════════════

// ── jobs_query ──

#[test]
fn build_query_request_sets_fields() {
    let req = build_query_request("SELECT 1".into(), false, "US".into());
    assert_eq!(req.query, "SELECT 1");
    assert!(!req.use_legacy_sql);
    assert_eq!(req.location, "US");
    assert!(req.max_results.is_none());
    assert_eq!(req.timeout_ms, Some(30000));
}

#[test]
fn build_query_request_with_legacy_sql() {
    let req = build_query_request("SELECT 1".into(), true, "EU".into());
    assert!(req.use_legacy_sql);
    assert_eq!(req.location, "EU");
}

// ── doctor ──

#[test]
fn build_columns_query_substitutes_params() {
    let sql = build_columns_query("my-proj", "my_ds", "events");
    assert!(sql.contains("my-proj.my_ds.INFORMATION_SCHEMA.COLUMNS"));
    assert!(sql.contains("table_name = 'events'"));
}

#[test]
fn build_stats_query_substitutes_params() {
    let sql = build_stats_query("my-proj", "my_ds", "events");
    assert!(sql.contains("my-proj.my_ds.events"));
}

// ── get_trace ──

#[test]
fn build_trace_query_substitutes_all_params() {
    let sql = build_trace_query("proj", "ds", "events", "session-123");
    assert!(sql.contains("proj.ds.events"));
    assert!(sql.contains("session_id = 'session-123'"));
    assert!(sql.contains("ORDER BY timestamp ASC"));
}

// ── evaluate ──

#[test]
fn build_evaluate_latency_query() {
    let sql = build_evaluate_query(
        &EvaluatorType::Latency,
        "proj",
        "ds",
        "events",
        "INTERVAL 24 HOUR",
        5000.0,
        None,
    );
    assert!(sql.contains("proj.ds.events"));
    assert!(sql.contains("INTERVAL 24 HOUR"));
    assert!(sql.contains("5000"));
    assert!(sql.contains("max_latency_ms"));
    // No agent filter
    assert!(!sql.contains("AND agent ="));
}

#[test]
fn build_evaluate_error_rate_with_agent() {
    let sql = build_evaluate_query(
        &EvaluatorType::ErrorRate,
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        0.1,
        Some("sales_agent"),
    );
    assert!(sql.contains("error_rate"));
    assert!(sql.contains("AND agent = 'sales_agent'"));
    assert!(sql.contains("0.1"));
}

// ═══════════════════════════════════════════════
// Result Mapper Tests
// ═══════════════════════════════════════════════

// ── doctor ──

#[test]
fn columns_from_result_extracts_names() {
    let result = QueryResult {
        schema: TableSchema {
            fields: vec![SchemaField {
                name: "column_name".into(),
                field_type: "STRING".into(),
                mode: None,
            }],
        },
        rows: vec![
            make_row(vec![("column_name", json!("session_id"))]),
            make_row(vec![("column_name", json!("agent"))]),
            make_row(vec![("column_name", json!("event_type"))]),
        ],
        total_rows: 3,
    };
    let cols = columns_from_result(&result);
    assert_eq!(cols, vec!["session_id", "agent", "event_type"]);
}

#[test]
fn find_missing_columns_detects_gaps() {
    let cols = vec!["session_id".into(), "timestamp".into()];
    let missing = find_missing_columns(&cols);
    assert!(missing.contains(&"agent".to_string()));
    assert!(missing.contains(&"event_type".to_string()));
    assert!(!missing.contains(&"session_id".to_string()));
}

#[test]
fn find_missing_columns_returns_empty_when_all_present() {
    let cols = vec![
        "session_id".into(),
        "agent".into(),
        "event_type".into(),
        "timestamp".into(),
    ];
    assert!(find_missing_columns(&cols).is_empty());
}

#[test]
fn doctor_report_healthy() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("total_rows", json!("100")),
            ("distinct_sessions", json!("10")),
            ("distinct_agents", json!("2")),
            ("earliest_event", json!("2026-03-01T00:00:00Z")),
            ("latest_event", json!("2026-03-10T00:00:00Z")),
            ("minutes_since_last_event", json!("5")),
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
    let report = doctor_report_from_rows("proj.ds.events", columns, &result).unwrap();
    assert_eq!(report.status, "healthy");
    assert_eq!(report.total_rows, 100);
    assert_eq!(report.distinct_sessions, 10);
    assert!(report.warnings.is_empty());
}

#[test]
fn doctor_report_empty_table_is_error() {
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
    let report = doctor_report_from_rows("proj.ds.events", vec![], &result).unwrap();
    assert_eq!(report.status, "error");
    assert!(report.warnings.iter().any(|w| w.contains("empty")));
}

// ── get_trace ──

#[test]
fn trace_result_from_rows_basic() {
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
    assert_eq!(trace.session_id, "s1");
    assert_eq!(trace.agent, "agent_a");
    assert_eq!(trace.event_count, 2);
    assert!(!trace.has_errors);
    assert_eq!(trace.events[0].event_type, "LLM_REQUEST");
    assert_eq!(trace.events[1].event_type, "LLM_RESPONSE");
}

#[test]
fn trace_result_from_rows_empty_returns_error() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };
    let err = trace_result_from_rows("s1".into(), &result);
    assert!(err.is_err());
    let msg = err.err().unwrap().to_string();
    assert!(msg.contains("No events found"));
}

#[test]
fn trace_result_detects_errors() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("session_id", json!("s1")),
            ("agent", json!("agent_a")),
            ("event_type", json!("TOOL_ERROR")),
            ("timestamp", json!("2026-03-05 10:00:00.000 UTC")),
            ("status", json!("ERROR")),
            ("error_message", json!("connection refused")),
            ("latency_ms", serde_json::Value::Null),
            ("content", serde_json::Value::Null),
        ])],
        total_rows: 1,
    };
    let trace = trace_result_from_rows("s1".into(), &result).unwrap();
    assert!(trace.has_errors);
}

// ── evaluate ──

#[test]
fn eval_result_latency_all_pass() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s1")),
                ("agent", json!("agent_a")),
                ("max_latency_ms", json!("1200")),
                ("avg_latency_ms", json!("800")),
                ("no_latency_data", json!("false")),
                ("passed", json!("true")),
            ]),
            make_row(vec![
                ("session_id", json!("s2")),
                ("agent", json!("agent_a")),
                ("max_latency_ms", json!("900")),
                ("avg_latency_ms", json!("500")),
                ("no_latency_data", json!("false")),
                ("passed", json!("true")),
            ]),
        ],
        total_rows: 2,
    };
    let eval = eval_result_from_rows(&EvaluatorType::Latency, 5000.0, "24h".into(), None, &result);
    assert_eq!(eval.evaluator, "latency");
    assert_eq!(eval.threshold, 5000.0);
    assert_eq!(eval.total_sessions, 2);
    assert_eq!(eval.passed, 2);
    assert_eq!(eval.failed, 0);
    assert!((eval.pass_rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn eval_result_error_rate_with_failures() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s1")),
                ("agent", json!("agent_a")),
                ("total_events", json!("10")),
                ("error_events", json!("5")),
                ("error_rate", json!("0.5")),
                ("passed", json!("false")),
            ]),
            make_row(vec![
                ("session_id", json!("s2")),
                ("agent", json!("agent_a")),
                ("total_events", json!("10")),
                ("error_events", json!("0")),
                ("error_rate", json!("0.0")),
                ("passed", json!("true")),
            ]),
        ],
        total_rows: 2,
    };
    let eval = eval_result_from_rows(
        &EvaluatorType::ErrorRate,
        0.1,
        "7d".into(),
        Some("agent_a".into()),
        &result,
    );
    assert_eq!(eval.evaluator, "error_rate");
    assert_eq!(eval.total_sessions, 2);
    assert_eq!(eval.passed, 1);
    assert_eq!(eval.failed, 1);
    assert!((eval.pass_rate - 0.5).abs() < f64::EPSILON);
}

#[test]
fn eval_result_empty_is_100_percent() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };
    let eval = eval_result_from_rows(&EvaluatorType::Latency, 5000.0, "1h".into(), None, &result);
    assert_eq!(eval.total_sessions, 0);
    assert!((eval.pass_rate - 1.0).abs() < f64::EPSILON);
}

// ═══════════════════════════════════════════════
// End-to-end command tests via MockExecutor
// ═══════════════════════════════════════════════

// ── jobs_query ──

#[tokio::test]
async fn jobs_query_json_output() {
    let executor = MockExecutor::new(
        vec![("name", "STRING"), ("value", "INTEGER")],
        vec![make_row(vec![
            ("name", json!("foo")),
            ("value", json!("42")),
        ])],
    );
    let config = test_config(OutputFormat::Json);
    let result =
        bqx::commands::jobs_query::run_with_executor(&executor, "SELECT 1".into(), false, &config)
            .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn jobs_query_text_output() {
    let executor = MockExecutor::new(
        vec![("col_a", "STRING"), ("col_b", "STRING")],
        vec![make_row(vec![
            ("col_a", json!("hello")),
            ("col_b", json!("world")),
        ])],
    );
    let config = test_config(OutputFormat::Text);
    let result =
        bqx::commands::jobs_query::run_with_executor(&executor, "SELECT 1".into(), false, &config)
            .await;
    assert!(result.is_ok());
}

// ── doctor ──

/// MockExecutor that returns different results for the two doctor queries.
struct DoctorMockExecutor {
    columns_result: QueryResult,
    stats_result: QueryResult,
}

#[async_trait]
impl QueryExecutor for DoctorMockExecutor {
    async fn query(&self, _project: &str, req: QueryRequest) -> Result<QueryResult> {
        if req.query.contains("INFORMATION_SCHEMA") {
            Ok(QueryResult {
                schema: self.columns_result.schema.clone(),
                rows: self.columns_result.rows.clone(),
                total_rows: self.columns_result.total_rows,
            })
        } else {
            Ok(QueryResult {
                schema: self.stats_result.schema.clone(),
                rows: self.stats_result.rows.clone(),
                total_rows: self.stats_result.total_rows,
            })
        }
    }
}

#[tokio::test]
async fn doctor_run_with_executor_healthy() {
    let columns_result = QueryResult {
        schema: TableSchema {
            fields: vec![SchemaField {
                name: "column_name".into(),
                field_type: "STRING".into(),
                mode: None,
            }],
        },
        rows: vec![
            make_row(vec![("column_name", json!("session_id"))]),
            make_row(vec![("column_name", json!("agent"))]),
            make_row(vec![("column_name", json!("event_type"))]),
            make_row(vec![("column_name", json!("timestamp"))]),
        ],
        total_rows: 4,
    };
    let stats_result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![make_row(vec![
            ("total_rows", json!("50")),
            ("distinct_sessions", json!("5")),
            ("distinct_agents", json!("1")),
            ("earliest_event", json!("2026-03-01T00:00:00Z")),
            ("latest_event", json!("2026-03-10T09:00:00Z")),
            ("minutes_since_last_event", json!("10")),
            ("null_session_ids", json!("0")),
            ("null_agents", json!("0")),
            ("null_event_types", json!("0")),
            ("null_timestamps", json!("0")),
            ("distinct_event_types", json!("4")),
        ])],
        total_rows: 1,
    };
    let executor = DoctorMockExecutor {
        columns_result,
        stats_result,
    };
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::doctor::run_with_executor(&executor, &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn doctor_run_missing_columns_returns_early() {
    let columns_result = QueryResult {
        schema: TableSchema {
            fields: vec![SchemaField {
                name: "column_name".into(),
                field_type: "STRING".into(),
                mode: None,
            }],
        },
        // Only session_id — missing agent, event_type, timestamp
        rows: vec![make_row(vec![("column_name", json!("session_id"))])],
        total_rows: 1,
    };
    let stats_result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };
    let executor = DoctorMockExecutor {
        columns_result,
        stats_result,
    };
    let config = test_config(OutputFormat::Json);
    // Should succeed (renders the error report) rather than bail
    let result = bqx::commands::analytics::doctor::run_with_executor(&executor, &config).await;
    assert!(result.is_ok());
}

// ── get_trace ──

#[tokio::test]
async fn get_trace_json_output() {
    let executor = MockExecutor::new(
        vec![
            ("session_id", "STRING"),
            ("agent", "STRING"),
            ("event_type", "STRING"),
            ("timestamp", "TIMESTAMP"),
            ("status", "STRING"),
            ("error_message", "STRING"),
            ("latency_ms", "STRING"),
            ("content", "STRING"),
        ],
        vec![make_row(vec![
            ("session_id", json!("s1")),
            ("agent", json!("test_agent")),
            ("event_type", json!("LLM_REQUEST")),
            ("timestamp", json!("2026-03-05 09:26:59.270 UTC")),
            ("status", json!("OK")),
            ("error_message", serde_json::Value::Null),
            ("latency_ms", serde_json::Value::Null),
            ("content", serde_json::Value::Null),
        ])],
    );
    let config = test_config(OutputFormat::Json);
    let result =
        bqx::commands::analytics::get_trace::run_with_executor(&executor, "s1".into(), &config)
            .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_trace_empty_session_errors() {
    let executor = MockExecutor::empty(vec![
        ("session_id", "STRING"),
        ("agent", "STRING"),
        ("event_type", "STRING"),
        ("timestamp", "TIMESTAMP"),
        ("status", "STRING"),
        ("error_message", "STRING"),
        ("latency_ms", "STRING"),
        ("content", "STRING"),
    ]);
    let config = test_config(OutputFormat::Json);
    let result =
        bqx::commands::analytics::get_trace::run_with_executor(&executor, "s1".into(), &config)
            .await;
    let err = result.unwrap_err();
    assert!(err.to_string().contains("No events found"));
}

#[tokio::test]
async fn get_trace_rejects_invalid_session_id() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::get_trace::run_with_executor(
        &executor,
        "bad session id!".into(),
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid session_id"));
}

// ── evaluate ──

#[tokio::test]
async fn evaluate_latency_json_output() {
    let executor = MockExecutor::new(
        vec![
            ("session_id", "STRING"),
            ("agent", "STRING"),
            ("max_latency_ms", "FLOAT"),
            ("avg_latency_ms", "FLOAT"),
            ("no_latency_data", "BOOLEAN"),
            ("passed", "BOOLEAN"),
        ],
        vec![make_row(vec![
            ("session_id", json!("s1")),
            ("agent", json!("agent_a")),
            ("max_latency_ms", json!("1200")),
            ("avg_latency_ms", json!("800")),
            ("no_latency_data", json!("false")),
            ("passed", json!("true")),
        ])],
    );
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        None,
        false,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn evaluate_with_exit_code_on_failure() {
    let executor = MockExecutor::new(
        vec![
            ("session_id", "STRING"),
            ("agent", "STRING"),
            ("max_latency_ms", "FLOAT"),
            ("avg_latency_ms", "FLOAT"),
            ("no_latency_data", "BOOLEAN"),
            ("passed", "BOOLEAN"),
        ],
        vec![make_row(vec![
            ("session_id", json!("s1")),
            ("agent", json!("agent_a")),
            ("max_latency_ms", json!("10000")),
            ("avg_latency_ms", json!("8000")),
            ("no_latency_data", json!("false")),
            ("passed", json!("false")),
        ])],
    );
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        None,
        true, // exit_code = true
        &config,
    )
    .await;
    assert!(result.is_err());
    // Should be a BqxError::EvalFailed
    let err = result.unwrap_err();
    assert!(err.downcast_ref::<bqx::models::BqxError>().is_some());
}

#[tokio::test]
async fn evaluate_rejects_invalid_agent_id() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        Some("bad agent!".into()),
        false,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid agent_id"));
}

#[tokio::test]
async fn evaluate_rejects_invalid_duration() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "bad_duration".into(),
        None,
        false,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid duration"));
}

// ═══════════════════════════════════════════════
// list-traces SQL Builder Tests
// ═══════════════════════════════════════════════

#[test]
fn build_list_traces_query_basic() {
    let sql = build_list_traces_query("proj", "ds", "events", "INTERVAL 24 HOUR", None, 20);
    assert!(sql.contains("proj.ds.events"));
    assert!(sql.contains("INTERVAL 24 HOUR"));
    assert!(sql.contains("LIMIT 20"));
    assert!(!sql.contains("AND agent ="));
}

#[test]
fn build_list_traces_query_with_agent_filter() {
    let sql = build_list_traces_query(
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        Some("support_bot"),
        10,
    );
    assert!(sql.contains("AND agent = 'support_bot'"));
    assert!(sql.contains("LIMIT 10"));
}

// ── list-traces Result Mapper Tests ──

#[test]
fn traces_from_rows_parses_results() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            make_row(vec![
                ("session_id", json!("s1")),
                ("agent", json!("support_bot")),
                ("event_count", json!("12")),
                ("started_at", json!("2026-03-13 10:00:00 UTC")),
                ("ended_at", json!("2026-03-13 10:01:00 UTC")),
                ("has_errors", json!(false)),
            ]),
            make_row(vec![
                ("session_id", json!("s2")),
                ("agent", json!("sales_agent")),
                ("event_count", json!("5")),
                ("started_at", json!("2026-03-13 09:00:00 UTC")),
                ("ended_at", json!("2026-03-13 09:00:30 UTC")),
                ("has_errors", json!(true)),
            ]),
        ],
        total_rows: 2,
    };
    let traces = traces_from_rows(&result);
    assert_eq!(traces.len(), 2);
    assert_eq!(traces[0].session_id, "s1");
    assert_eq!(traces[0].agent, "support_bot");
    assert_eq!(traces[0].event_count, 12);
    assert!(!traces[0].has_errors);
    assert_eq!(traces[1].session_id, "s2");
    assert!(traces[1].has_errors);
}

#[test]
fn traces_from_rows_empty() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };
    let traces = traces_from_rows(&result);
    assert!(traces.is_empty());
}

// ── list-traces Integration Tests ──

#[tokio::test]
async fn list_traces_json_output() {
    let executor = MockExecutor::new(
        vec![
            ("session_id", "STRING"),
            ("agent", "STRING"),
            ("event_count", "INTEGER"),
            ("started_at", "TIMESTAMP"),
            ("ended_at", "TIMESTAMP"),
            ("has_errors", "BOOLEAN"),
        ],
        vec![make_row(vec![
            ("session_id", json!("s1")),
            ("agent", json!("test_agent")),
            ("event_count", json!("5")),
            ("started_at", json!("2026-03-13 10:00:00 UTC")),
            ("ended_at", json!("2026-03-13 10:01:00 UTC")),
            ("has_errors", json!(false)),
        ])],
    );
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "24h".into(),
        None,
        20,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn list_traces_text_output() {
    let executor = MockExecutor::new(
        vec![
            ("session_id", "STRING"),
            ("agent", "STRING"),
            ("event_count", "INTEGER"),
            ("started_at", "TIMESTAMP"),
            ("ended_at", "TIMESTAMP"),
            ("has_errors", "BOOLEAN"),
        ],
        vec![make_row(vec![
            ("session_id", json!("s1")),
            ("agent", json!("test_agent")),
            ("event_count", json!("3")),
            ("started_at", json!("2026-03-13 10:00:00 UTC")),
            ("ended_at", json!("2026-03-13 10:00:30 UTC")),
            ("has_errors", json!(false)),
        ])],
    );
    let config = test_config(OutputFormat::Text);
    let result = bqx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "7d".into(),
        Some("test_agent".into()),
        10,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn list_traces_rejects_invalid_agent_id() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "24h".into(),
        Some("bad agent!".into()),
        20,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid agent_id"));
}

#[tokio::test]
async fn list_traces_rejects_invalid_duration() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = bqx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "bad_duration".into(),
        None,
        20,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid duration"));
}

// ═══════════════════════════════════════════════
// views SQL Builder Tests
// ═══════════════════════════════════════════════

#[test]
fn build_create_view_sql_basic() {
    let (view_name, sql) =
        build_create_view_sql("proj", "ds", "agent_events", "adk_", "LLM_REQUEST");
    assert_eq!(view_name, "adk_llm_request");
    assert!(sql.contains("CREATE OR REPLACE VIEW `proj.ds.adk_llm_request`"));
    assert!(sql.contains("FROM `proj.ds.agent_events`"));
    assert!(sql.contains("WHERE event_type = 'LLM_REQUEST'"));
}

#[test]
fn build_create_view_sql_no_prefix() {
    let (view_name, sql) = build_create_view_sql("proj", "ds", "agent_events", "", "TOOL_ERROR");
    assert_eq!(view_name, "tool_error");
    assert!(sql.contains("CREATE OR REPLACE VIEW `proj.ds.tool_error`"));
}

// ── views Integration Tests ──

#[tokio::test]
async fn views_create_all_json_output() {
    // Mock always succeeds — each DDL returns empty
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let result =
        bqx::commands::analytics::views::run_with_executor(&executor, "adk_".into(), &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn views_create_all_text_output() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Text);
    let result =
        bqx::commands::analytics::views::run_with_executor(&executor, "adk_".into(), &config).await;
    assert!(result.is_ok());
}

/// MockExecutor that always fails.
struct FailingExecutor;

#[async_trait]
impl QueryExecutor for FailingExecutor {
    async fn query(&self, _project: &str, _req: QueryRequest) -> Result<QueryResult> {
        anyhow::bail!("permission denied")
    }
}

#[tokio::test]
async fn views_create_all_with_failures_returns_error() {
    let executor = FailingExecutor;
    let config = test_config(OutputFormat::Json);
    let result =
        bqx::commands::analytics::views::run_with_executor(&executor, "adk_".into(), &config).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("18 of 18 views failed"), "Got: {err}");
}

#[test]
fn validate_view_prefix_accepts_valid() {
    assert!(bqx::config::validate_view_prefix("").is_ok());
    assert!(bqx::config::validate_view_prefix("adk_").is_ok());
    assert!(bqx::config::validate_view_prefix("MyPrefix123").is_ok());
}

#[test]
fn validate_view_prefix_rejects_invalid() {
    let bad = bqx::config::validate_view_prefix("bad\\prefix");
    assert!(bad.is_err());
    assert!(bad.unwrap_err().to_string().contains("Invalid view prefix"));

    assert!(bqx::config::validate_view_prefix("has space").is_err());
    assert!(bqx::config::validate_view_prefix("has-dash").is_err());
    assert!(bqx::config::validate_view_prefix("has.dot").is_err());
}
