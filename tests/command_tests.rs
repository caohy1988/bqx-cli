use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use dcx::bigquery::client::{QueryExecutor, QueryRequest, QueryResult, SchemaField, TableSchema};
use dcx::cli::{EvaluatorType, OutputFormat};
use dcx::commands::analytics::categorical_eval::{
    build_list_sessions_query, build_persist_sql, build_session_events_query, classify_session,
    sessions_from_rows, CategoryDef, Classification, MetricDefinition, SessionCategoricalResult,
};
use dcx::commands::analytics::categorical_views::build_categorical_view_sqls;
use dcx::commands::analytics::distribution::{build_distribution_query, distribution_from_rows};
use dcx::commands::analytics::doctor::{
    build_columns_query, build_stats_query, columns_from_result, doctor_report_from_rows,
    find_missing_columns,
};
use dcx::commands::analytics::drift::{build_drift_query, drift_from_rows};
use dcx::commands::analytics::evaluate::{build_evaluate_query, eval_result_from_rows};
use dcx::commands::analytics::get_trace::{build_trace_query, trace_result_from_rows};
use dcx::commands::analytics::hitl_metrics::{
    build_hitl_sessions_query, build_hitl_summary_query, hitl_sessions_from_rows,
    hitl_summary_from_rows,
};
use dcx::commands::analytics::insights::{
    build_insights_query, build_top_errors_query, build_top_tools_query, summary_from_rows,
    top_errors_from_rows, top_tools_from_rows,
};
use dcx::commands::analytics::list_traces::{build_list_traces_query, traces_from_rows};
use dcx::commands::analytics::views::{build_create_view_sql, is_known_event_type};
use dcx::commands::jobs_query::build_query_request;
use dcx::config::Config;

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
        100,
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
        100,
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
        dcx::commands::jobs_query::run_with_executor(&executor, "SELECT 1".into(), false, &config)
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
        dcx::commands::jobs_query::run_with_executor(&executor, "SELECT 1".into(), false, &config)
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
    let result = dcx::commands::analytics::doctor::run_with_executor(&executor, &config).await;
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
    let result = dcx::commands::analytics::doctor::run_with_executor(&executor, &config).await;
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
        dcx::commands::analytics::get_trace::run_with_executor(&executor, "s1".into(), &config)
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
        dcx::commands::analytics::get_trace::run_with_executor(&executor, "s1".into(), &config)
            .await;
    let err = result.unwrap_err();
    assert!(err.to_string().contains("No events found"));
}

#[tokio::test]
async fn get_trace_rejects_invalid_session_id() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::get_trace::run_with_executor(
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
    let result = dcx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        None,
        false,
        100,
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
    let result = dcx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        None,
        true, // exit_code = true
        100,
        &config,
    )
    .await;
    assert!(result.is_err());
    // Should be a BqxError::EvalFailed
    let err = result.unwrap_err();
    assert!(err.downcast_ref::<dcx::models::BqxError>().is_some());
}

#[tokio::test]
async fn evaluate_rejects_invalid_agent_id() {
    let executor = MockExecutor::empty(vec![("session_id", "STRING")]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "24h".into(),
        Some("bad agent!".into()),
        false,
        100,
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
    let result = dcx::commands::analytics::evaluate::run_with_executor(
        &executor,
        EvaluatorType::Latency,
        5000.0,
        "bad_duration".into(),
        None,
        false,
        100,
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
    let sql = build_list_traces_query("proj", "ds", "events", "INTERVAL 24 HOUR", None, None, 20);
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
        None,
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
    let result = dcx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "24h".into(),
        None,
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
    let result = dcx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "7d".into(),
        None,
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
    let result = dcx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "24h".into(),
        None,
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
    let result = dcx::commands::analytics::list_traces::run_with_executor(
        &executor,
        "bad_duration".into(),
        None,
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
        dcx::commands::analytics::views::run_with_executor(&executor, "adk_".into(), &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn views_create_all_text_output() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Text);
    let result =
        dcx::commands::analytics::views::run_with_executor(&executor, "adk_".into(), &config).await;
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
        dcx::commands::analytics::views::run_with_executor(&executor, "adk_".into(), &config).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("18 of 18 views failed"), "Got: {err}");
}

#[test]
fn validate_view_prefix_accepts_valid() {
    assert!(dcx::config::validate_view_prefix("").is_ok());
    assert!(dcx::config::validate_view_prefix("adk_").is_ok());
    assert!(dcx::config::validate_view_prefix("MyPrefix123").is_ok());
}

#[test]
fn validate_view_prefix_rejects_invalid() {
    let bad = dcx::config::validate_view_prefix("bad\\prefix");
    assert!(bad.is_err());
    assert!(bad.unwrap_err().to_string().contains("Invalid view prefix"));

    assert!(dcx::config::validate_view_prefix("has space").is_err());
    assert!(dcx::config::validate_view_prefix("has-dash").is_err());
    assert!(dcx::config::validate_view_prefix("has.dot").is_err());
}

// ── Insights tests ──

#[test]
fn build_insights_query_basic() {
    let sql = build_insights_query("proj", "ds", "events", "INTERVAL 24 HOUR", None);
    assert!(sql.contains("proj.ds.events"));
    assert!(sql.contains("INTERVAL 24 HOUR"));
    assert!(sql.contains("total_sessions"));
    assert!(sql.contains("error_rate"));
}

#[test]
fn build_top_errors_query_with_agent() {
    let sql = build_top_errors_query("proj", "ds", "events", "INTERVAL 7 DAY", Some("bot"));
    assert!(sql.contains("AND agent = 'bot'"));
    assert!(sql.contains("LIMIT 5"));
}

#[test]
fn build_top_tools_query_basic() {
    let sql = build_top_tools_query("proj", "ds", "events", "INTERVAL 1 HOUR", None);
    assert!(sql.contains("tool_name"));
    assert!(sql.contains("LIMIT 10"));
}

#[test]
fn summary_from_rows_parses_result() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![{
            let mut m = serde_json::Map::new();
            m.insert("total_sessions".into(), json!("10"));
            m.insert("total_events".into(), json!("200"));
            m.insert("total_errors".into(), json!("5"));
            m.insert("error_rate".into(), json!("0.025"));
            m.insert("sessions_with_errors".into(), json!("3"));
            m.insert("session_error_rate".into(), json!("0.3"));
            m.insert("avg_events_per_session".into(), json!("20.0"));
            m.insert("total_llm_requests".into(), json!("80"));
            m.insert("total_tool_calls".into(), json!("40"));
            m.insert("peak_latency_ms".into(), json!("5000.0"));
            m.insert("avg_latency_ms".into(), json!("1200.0"));
            m.insert("earliest_session".into(), json!("2026-03-13 00:00:00 UTC"));
            m.insert("latest_session".into(), json!("2026-03-13 23:59:00 UTC"));
            m
        }],
        total_rows: 1,
    };
    let summary = summary_from_rows(&result);
    assert_eq!(summary.total_sessions, 10);
    assert_eq!(summary.total_events, 200);
    assert_eq!(summary.total_errors, 5);
    assert!((summary.error_rate - 0.025).abs() < 0.001);
    assert_eq!(summary.total_llm_requests, 80);
    assert!(summary.peak_latency_ms.is_some());
}

#[test]
fn summary_from_rows_empty() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };
    let summary = summary_from_rows(&result);
    assert_eq!(summary.total_sessions, 0);
}

#[test]
fn top_errors_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![{
            let mut m = serde_json::Map::new();
            m.insert("event_type".into(), json!("TOOL_ERROR"));
            m.insert("error_message".into(), json!("timeout"));
            m.insert("occurrences".into(), json!("3"));
            m
        }],
        total_rows: 1,
    };
    let errors = top_errors_from_rows(&result);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].event_type, "TOOL_ERROR");
    assert_eq!(errors[0].occurrences, 3);
}

#[test]
fn top_tools_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![{
            let mut m = serde_json::Map::new();
            m.insert("tool_name".into(), json!("search"));
            m.insert("call_count".into(), json!("25"));
            m.insert("avg_latency_ms".into(), json!("500.0"));
            m.insert("max_latency_ms".into(), json!("2000.0"));
            m
        }],
        total_rows: 1,
    };
    let tools = top_tools_from_rows(&result);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].tool_name, "search");
    assert_eq!(tools[0].call_count, 25);
}

#[tokio::test]
async fn insights_json_output() {
    let executor = MockExecutor::new(
        vec![("total_sessions", "INTEGER")],
        vec![{
            let mut m = serde_json::Map::new();
            m.insert("total_sessions".into(), json!("5"));
            m.insert("total_events".into(), json!("100"));
            m.insert("total_errors".into(), json!("2"));
            m.insert("error_rate".into(), json!("0.02"));
            m.insert("sessions_with_errors".into(), json!("1"));
            m.insert("session_error_rate".into(), json!("0.2"));
            m.insert("avg_events_per_session".into(), json!("20.0"));
            m.insert("total_llm_requests".into(), json!("50"));
            m.insert("total_tool_calls".into(), json!("20"));
            m.insert("peak_latency_ms".into(), json!(serde_json::Value::Null));
            m.insert("avg_latency_ms".into(), json!(serde_json::Value::Null));
            m.insert("earliest_session".into(), json!(serde_json::Value::Null));
            m.insert("latest_session".into(), json!(serde_json::Value::Null));
            m
        }],
    );
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::insights::run_with_executor(
        &executor,
        "24h".into(),
        None,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

// ── Drift tests ──

#[test]
fn build_drift_query_basic() {
    let sql = build_drift_query("proj", "ds", "events", "golden_qs", "INTERVAL 7 DAY", None);
    assert!(sql.contains("proj.ds.golden_qs"));
    assert!(sql.contains("proj.ds.events"));
    assert!(sql.contains("INTERVAL 7 DAY"));
    assert!(sql.contains("golden_question"));
}

#[test]
fn build_drift_query_with_agent() {
    let sql = build_drift_query("proj", "ds", "events", "gq", "INTERVAL 1 DAY", Some("bot"));
    assert!(sql.contains("AND agent = 'bot'"));
}

#[test]
fn drift_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            {
                let mut m = serde_json::Map::new();
                m.insert("golden_question".into(), json!("What is error rate?"));
                m.insert("expected_answer".into(), json!("Low"));
                m.insert("session_id".into(), json!("s1"));
                m.insert("actual_answer".into(), json!("Very low"));
                m.insert("covered".into(), json!("true"));
                m
            },
            {
                let mut m = serde_json::Map::new();
                m.insert("golden_question".into(), json!("How many users?"));
                m.insert("expected_answer".into(), json!("100"));
                m.insert("session_id".into(), json!(serde_json::Value::Null));
                m.insert("actual_answer".into(), json!(serde_json::Value::Null));
                m.insert("covered".into(), json!("false"));
                m
            },
        ],
        total_rows: 2,
    };
    let questions = drift_from_rows(&result);
    assert_eq!(questions.len(), 2);
    assert!(questions[0].covered);
    assert_eq!(questions[0].session_id, Some("s1".into()));
    assert!(!questions[1].covered);
    assert!(questions[1].session_id.is_none());
}

#[tokio::test]
async fn drift_json_output() {
    let executor = MockExecutor::new(
        vec![("golden_question", "STRING")],
        vec![{
            let mut m = serde_json::Map::new();
            m.insert("golden_question".into(), json!("Q1"));
            m.insert("expected_answer".into(), json!("A1"));
            m.insert("session_id".into(), json!("s1"));
            m.insert("actual_answer".into(), json!("A1b"));
            m.insert("covered".into(), json!("true"));
            m
        }],
    );
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::drift::run_with_executor(
        &executor,
        "golden_qs".into(),
        "7d".into(),
        None,
        0.8,
        false,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn drift_with_exit_code_on_failure() {
    let executor = MockExecutor::new(
        vec![("golden_question", "STRING")],
        vec![{
            let mut m = serde_json::Map::new();
            m.insert("golden_question".into(), json!("Q1"));
            m.insert("expected_answer".into(), json!("A1"));
            m.insert("session_id".into(), json!(serde_json::Value::Null));
            m.insert("actual_answer".into(), json!(serde_json::Value::Null));
            m.insert("covered".into(), json!("false"));
            m
        }],
    );
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::drift::run_with_executor(
        &executor,
        "golden_qs".into(),
        "7d".into(),
        None,
        0.8,
        true,
        &config,
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn drift_rejects_invalid_golden_dataset() {
    let executor = MockExecutor::new(vec![], vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::drift::run_with_executor(
        &executor,
        "bad dataset!".into(),
        "7d".into(),
        None,
        0.8,
        false,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid golden-dataset"));
}

#[tokio::test]
async fn drift_rejects_invalid_min_coverage() {
    let executor = MockExecutor::new(vec![], vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::drift::run_with_executor(
        &executor,
        "golden_qs".into(),
        "7d".into(),
        None,
        2.0,
        false,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid min-coverage"),
        "Should reject min-coverage > 1.0"
    );
}

#[test]
fn drift_sql_deduplicates_per_golden_question() {
    let sql = build_drift_query("proj", "ds", "events", "gq", "INTERVAL 7 DAY", None);
    assert!(
        sql.contains("ROW_NUMBER()"),
        "SQL must deduplicate with ROW_NUMBER"
    );
    assert!(
        sql.contains("WHERE rn = 1"),
        "SQL must keep only first match per golden question"
    );
}

#[test]
fn drift_from_rows_coverage_not_inflated_by_duplicates() {
    // Simulate what would happen if the SQL returned two rows for one golden
    // question (i.e. the old bug). With the fix, the SQL deduplicates, but
    // the Rust layer should also produce correct coverage from any input.
    // Two distinct golden questions: one covered, one not.
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            {
                let mut m = serde_json::Map::new();
                m.insert("golden_question".into(), json!("Q1"));
                m.insert("expected_answer".into(), json!("A1"));
                m.insert("session_id".into(), json!("s1"));
                m.insert("actual_answer".into(), json!("A1b"));
                m.insert("covered".into(), json!("true"));
                m
            },
            {
                let mut m = serde_json::Map::new();
                m.insert("golden_question".into(), json!("Q2"));
                m.insert("expected_answer".into(), json!("A2"));
                m.insert("session_id".into(), json!(serde_json::Value::Null));
                m.insert("actual_answer".into(), json!(serde_json::Value::Null));
                m.insert("covered".into(), json!("false"));
                m
            },
        ],
        total_rows: 2,
    };
    let questions = drift_from_rows(&result);
    assert_eq!(questions.len(), 2);
    let covered = questions.iter().filter(|q| q.covered).count();
    let coverage = covered as f64 / questions.len() as f64;
    assert!(
        (coverage - 0.5).abs() < 0.01,
        "Coverage should be 1/2 = 0.50, got {coverage}"
    );
}

// ── Distribution tests ──

#[test]
fn build_distribution_query_basic() {
    let sql = build_distribution_query("proj", "ds", "events", "INTERVAL 24 HOUR", None, 100);
    assert!(sql.contains("proj.ds.events"));
    assert!(sql.contains("event_type"));
    assert!(sql.contains("proportion"));
}

#[test]
fn distribution_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            {
                let mut m = serde_json::Map::new();
                m.insert("event_type".into(), json!("LLM_REQUEST"));
                m.insert("event_count".into(), json!("40"));
                m.insert("session_count".into(), json!("10"));
                m.insert("proportion".into(), json!("0.4"));
                m
            },
            {
                let mut m = serde_json::Map::new();
                m.insert("event_type".into(), json!("TOOL_CALL"));
                m.insert("event_count".into(), json!("30"));
                m.insert("session_count".into(), json!("8"));
                m.insert("proportion".into(), json!("0.3"));
                m
            },
        ],
        total_rows: 2,
    };
    let dist = distribution_from_rows(&result);
    assert_eq!(dist.len(), 2);
    assert_eq!(dist[0].event_type, "LLM_REQUEST");
    assert_eq!(dist[0].event_count, 40);
    assert!((dist[0].proportion - 0.4).abs() < 0.001);
}

#[tokio::test]
async fn distribution_json_output() {
    let executor = MockExecutor::new(
        vec![("event_type", "STRING")],
        vec![{
            let mut m = serde_json::Map::new();
            m.insert("event_type".into(), json!("LLM_REQUEST"));
            m.insert("event_count".into(), json!("50"));
            m.insert("session_count".into(), json!("10"));
            m.insert("proportion".into(), json!("1.0"));
            m
        }],
    );
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::distribution::run_with_executor(
        &executor,
        "24h".into(),
        None,
        100,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

// ── HITL Metrics tests ──

#[test]
fn build_hitl_summary_query_basic() {
    let sql = build_hitl_summary_query("proj", "ds", "events", "INTERVAL 7 DAY", None);
    assert!(sql.contains("HUMAN_INPUT_REQUIRED"));
    assert!(sql.contains("HUMAN_INPUT_RECEIVED"));
    assert!(sql.contains("hitl_session_rate"));
}

#[test]
fn build_hitl_sessions_query_with_limit() {
    let sql = build_hitl_sessions_query("proj", "ds", "events", "INTERVAL 1 HOUR", Some("bot"), 10);
    assert!(sql.contains("AND agent = 'bot'"));
    assert!(sql.contains("LIMIT 10"));
}

#[test]
fn hitl_summary_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![{
            let mut m = serde_json::Map::new();
            m.insert("total_sessions".into(), json!("20"));
            m.insert("hitl_required_count".into(), json!("5"));
            m.insert("hitl_received_count".into(), json!("4"));
            m.insert("sessions_with_hitl".into(), json!("3"));
            m.insert("hitl_session_rate".into(), json!("0.15"));
            m
        }],
        total_rows: 1,
    };
    let summary = hitl_summary_from_rows(&result);
    assert_eq!(summary.total_sessions, 20);
    assert_eq!(summary.hitl_required_count, 5);
    assert_eq!(summary.sessions_with_hitl, 3);
    assert!((summary.hitl_session_rate - 0.15).abs() < 0.001);
}

#[test]
fn hitl_summary_from_rows_empty() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };
    let summary = hitl_summary_from_rows(&result);
    assert_eq!(summary.total_sessions, 0);
}

#[test]
fn hitl_sessions_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![{
            let mut m = serde_json::Map::new();
            m.insert("session_id".into(), json!("s-abc"));
            m.insert("agent".into(), json!("bot"));
            m.insert("required_count".into(), json!("3"));
            m.insert("received_count".into(), json!("2"));
            m.insert("first_hitl_at".into(), json!("2026-03-13 10:00:00 UTC"));
            m.insert("last_hitl_at".into(), json!("2026-03-13 10:05:00 UTC"));
            m
        }],
        total_rows: 1,
    };
    let sessions = hitl_sessions_from_rows(&result);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "s-abc");
    assert_eq!(sessions[0].required_count, 3);
}

#[tokio::test]
async fn hitl_metrics_json_output() {
    let executor = MockExecutor::new(
        vec![("total_sessions", "INTEGER")],
        vec![{
            let mut m = serde_json::Map::new();
            m.insert("total_sessions".into(), json!("10"));
            m.insert("hitl_required_count".into(), json!("0"));
            m.insert("hitl_received_count".into(), json!("0"));
            m.insert("sessions_with_hitl".into(), json!("0"));
            m.insert("hitl_session_rate".into(), json!("0.0"));
            m
        }],
    );
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::hitl_metrics::run_with_executor(
        &executor,
        "7d".into(),
        None,
        20,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn insights_rejects_invalid_duration() {
    let executor = MockExecutor::new(vec![], vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::insights::run_with_executor(
        &executor,
        "bad".into(),
        None,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid duration"));
}

#[tokio::test]
async fn distribution_rejects_invalid_agent_id() {
    let executor = MockExecutor::new(vec![], vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::distribution::run_with_executor(
        &executor,
        "24h".into(),
        Some("bad agent!".into()),
        100,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid agent_id"));
}

// ═══════════════════════════════════════════════
// Views Create (single) Tests
// ═══════════════════════════════════════════════

#[test]
fn is_known_event_type_accepts_standard() {
    assert!(is_known_event_type("LLM_REQUEST"));
    assert!(is_known_event_type("llm_request")); // case-insensitive
    assert!(is_known_event_type("TOOL_ERROR"));
    assert!(is_known_event_type("SESSION_START"));
}

#[test]
fn is_known_event_type_returns_false_for_custom() {
    assert!(!is_known_event_type("CUSTOM_EVENT"));
    assert!(!is_known_event_type("MY_SPECIAL_TYPE"));
}

#[tokio::test]
async fn views_create_single_json_output() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::views::run_create_with_executor(
        &executor,
        "LLM_REQUEST".into(),
        "".into(),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn views_create_single_with_prefix() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::views::run_create_with_executor(
        &executor,
        "TOOL_COMPLETED".into(),
        "adk_".into(),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn views_create_single_accepts_custom_type() {
    // SDK passes event_type through directly; dcx should not reject custom types.
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::views::run_create_with_executor(
        &executor,
        "CUSTOM_EVENT".into(),
        "".into(),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════
// Categorical Eval Tests
// ═══════════════════════════════════════════════

#[test]
fn build_list_sessions_query_basic() {
    let sql = build_list_sessions_query("proj", "ds", "events", None, None, 100).unwrap();
    assert!(sql.contains("SELECT DISTINCT"));
    assert!(sql.contains("session_id"));
    assert!(sql.contains("LIMIT 100"));
}

#[test]
fn build_list_sessions_query_with_filters() {
    let sql =
        build_list_sessions_query("proj", "ds", "events", Some("7d"), Some("bot"), 50).unwrap();
    assert!(sql.contains("INTERVAL 7 DAY"));
    assert!(sql.contains("AND agent = 'bot'"));
    assert!(sql.contains("LIMIT 50"));
}

#[test]
fn build_session_events_query_basic() {
    let sql = build_session_events_query("proj", "ds", "events", "sess-123");
    assert!(sql.contains("WHERE session_id = 'sess-123'"));
    assert!(sql.contains("event_type"));
    assert!(sql.contains("user_query"));
}

#[test]
fn sessions_from_rows_parses() {
    let result = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            {
                let mut m = serde_json::Map::new();
                m.insert("session_id".into(), json!("s-1"));
                m.insert("agent".into(), json!("bot"));
                m
            },
            {
                let mut m = serde_json::Map::new();
                m.insert("session_id".into(), json!("s-2"));
                m.insert("agent".into(), json!("assistant"));
                m
            },
        ],
        total_rows: 2,
    };
    let sessions = sessions_from_rows(&result);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].0, "s-1");
    assert_eq!(sessions[0].1, "bot");
    assert_eq!(sessions[1].0, "s-2");
    assert_eq!(sessions[1].1, "assistant");
}

#[test]
fn classify_session_with_errors() {
    let events = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![
            {
                let mut m = serde_json::Map::new();
                m.insert("event_type".into(), json!("LLM_REQUEST"));
                m.insert("error_message".into(), json!(""));
                m.insert("user_query".into(), json!("hello"));
                m.insert("agent_response".into(), json!("hi"));
                m.insert("tool_name".into(), json!(""));
                m
            },
            {
                let mut m = serde_json::Map::new();
                m.insert("event_type".into(), json!("TOOL_ERROR"));
                m.insert("error_message".into(), json!("connection failed"));
                m.insert("user_query".into(), json!(""));
                m.insert("agent_response".into(), json!(""));
                m.insert("tool_name".into(), json!("search"));
                m
            },
        ],
        total_rows: 2,
    };

    let metrics = vec![MetricDefinition {
        name: "error_handling".into(),
        definition: "Evaluates error handling behavior".into(),
        categories: vec![
            CategoryDef {
                name: "graceful".into(),
                definition: "Handles errors gracefully".into(),
            },
            CategoryDef {
                name: "poor".into(),
                definition: "Poor error handling".into(),
            },
        ],
        required: true,
    }];

    let classifications = classify_session(&events, &metrics, true);
    assert_eq!(classifications.len(), 1);
    assert_eq!(classifications[0].metric, "error_handling");
    // Should classify as the last category because errors were found
    assert_eq!(classifications[0].category, "poor");
    assert!(classifications[0].justification.is_some());
}

#[test]
fn classify_session_without_justification() {
    let events = QueryResult {
        schema: TableSchema { fields: vec![] },
        rows: vec![],
        total_rows: 0,
    };

    let metrics = vec![MetricDefinition {
        name: "completeness".into(),
        definition: "Response completeness".into(),
        categories: vec![CategoryDef {
            name: "empty".into(),
            definition: "No events".into(),
        }],
        required: true,
    }];

    let classifications = classify_session(&events, &metrics, false);
    assert_eq!(classifications.len(), 1);
    assert!(classifications[0].justification.is_none());
}

#[tokio::test]
async fn categorical_eval_json_output() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let metrics = vec![MetricDefinition {
        name: "quality".into(),
        definition: "Response quality".into(),
        categories: vec![
            CategoryDef {
                name: "good".into(),
                definition: "Good quality".into(),
            },
            CategoryDef {
                name: "bad".into(),
                definition: "Bad quality".into(),
            },
        ],
        required: true,
    }];
    let result = dcx::commands::analytics::categorical_eval::run_with_executor(
        &executor,
        &metrics,
        Some("7d".into()),
        None,
        100,
        true,
        false,
        None,
        None,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════
// Categorical Views Tests
// ═══════════════════════════════════════════════

#[test]
fn build_categorical_view_sqls_creates_four_views() {
    let views = build_categorical_view_sqls("proj", "ds", "categorical_results", "");
    assert_eq!(views.len(), 4);
    let names: Vec<&str> = views.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"categorical_summary"));
    assert!(names.contains(&"categorical_timeline"));
    assert!(names.contains(&"categorical_by_agent"));
    assert!(names.contains(&"categorical_latest_per_session"));
}

#[test]
fn build_categorical_view_sqls_with_prefix() {
    let views = build_categorical_view_sqls("proj", "ds", "categorical_results", "v_");
    for (name, sql) in &views {
        assert!(name.starts_with("v_categorical_"));
        assert!(sql.contains("categorical_results"));
    }
}

#[test]
fn build_categorical_view_sqls_uses_custom_results_table() {
    let views = build_categorical_view_sqls("proj", "ds", "my_results", "");
    for (_, sql) in &views {
        assert!(sql.contains("my_results"));
    }
}

#[tokio::test]
async fn categorical_views_json_output() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::categorical_views::run_with_executor(
        &executor,
        "categorical_results".into(),
        "".into(),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn categorical_views_with_prefix_json_output() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::analytics::categorical_views::run_with_executor(
        &executor,
        "categorical_results".into(),
        "dash_".into(),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════
// SQL Escaping Tests (categorical-eval)
// ═══════════════════════════════════════════════

#[test]
fn build_persist_sql_escapes_single_quotes() {
    let results = vec![SessionCategoricalResult {
        session_id: "s-1".into(),
        agent: "O'Reilly's bot".into(),
        classifications: vec![Classification {
            metric: "met'ric".into(),
            category: "cat'egory".into(),
            justification: Some("it's a test".into()),
        }],
    }];
    let sql = build_persist_sql("proj", "ds", "results", &results, Some("v1.0"));
    // All single quotes in values must be escaped
    assert!(sql.contains("O\\'Reilly\\'s bot"));
    assert!(sql.contains("met\\'ric"));
    assert!(sql.contains("cat\\'egory"));
    assert!(sql.contains("it\\'s a test"));
    assert!(sql.contains("'v1.0'"));
    // No unescaped single quotes inside value positions
    assert!(!sql.contains("O'Reilly"));
}

#[test]
fn build_persist_sql_handles_null_justification_and_prompt_version() {
    let results = vec![SessionCategoricalResult {
        session_id: "s-1".into(),
        agent: "bot".into(),
        classifications: vec![Classification {
            metric: "quality".into(),
            category: "good".into(),
            justification: None,
        }],
    }];
    let sql = build_persist_sql("proj", "ds", "results", &results, None);
    // justification and prompt_version should be NULL
    let values_part = sql.split("VALUES").nth(1).unwrap();
    assert!(values_part.contains("NULL"));
    // Two NULLs: justification + prompt_version
    assert_eq!(values_part.matches("NULL").count(), 2);
}

#[test]
fn build_session_events_query_escapes_session_id() {
    let sql = build_session_events_query("proj", "ds", "events", "s'; DROP TABLE --");
    assert!(sql.contains("s\\'; DROP TABLE --"));
    assert!(!sql.contains("s'; DROP TABLE --"));
}

#[test]
fn build_list_sessions_query_escapes_agent_id() {
    let sql = build_list_sessions_query(
        "proj",
        "ds",
        "events",
        None,
        Some("bot'; DROP TABLE --"),
        10,
    )
    .unwrap();
    assert!(sql.contains("bot\\'; DROP TABLE --"));
    assert!(!sql.contains("bot'; DROP TABLE --"));
}

#[tokio::test]
async fn categorical_eval_rejects_endpoint_flag() {
    let config = test_config(OutputFormat::Json);
    // Create a temp metrics file
    let dir = std::env::temp_dir().join("dcx_test_metrics");
    let _ = std::fs::create_dir_all(&dir);
    let metrics_path = dir.join("test_metrics.json");
    std::fs::write(
        &metrics_path,
        r#"[{"name":"q","definition":"quality","categories":[{"name":"good","definition":"g"}]}]"#,
    )
    .unwrap();

    let auth_opts = dcx::auth::AuthOptions {
        token: Some("fake".into()),
        credentials_file: None,
    };
    let result = dcx::commands::analytics::categorical_eval::run(
        metrics_path.to_str().unwrap().into(),
        None,
        Some("7d".into()),
        10,
        Some("https://model.example.com".into()), // --endpoint provided
        true,
        false,
        None,
        None,
        &auth_opts,
        &config,
    )
    .await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("--endpoint is not yet supported"));
    // cleanup
    let _ = std::fs::remove_file(&metrics_path);
}

// ═══════════════════════════════════════════════
// Duplicate metric name rejection
// ═══════════════════════════════════════════════

#[test]
fn load_metrics_file_rejects_duplicate_names() {
    let dir = std::env::temp_dir().join("dcx_test_dup_metrics");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("dup_metrics.json");
    std::fs::write(
        &path,
        r#"[
            {"name":"quality","definition":"q","categories":[{"name":"good","definition":"g"}]},
            {"name":"quality","definition":"different","categories":[{"name":"bad","definition":"b"}]}
        ]"#,
    )
    .unwrap();

    let result =
        dcx::commands::analytics::categorical_eval::load_metrics_file(path.to_str().unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Duplicate metric name 'quality'"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_metrics_file_accepts_unique_names() {
    let dir = std::env::temp_dir().join("dcx_test_unique_metrics");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("unique_metrics.json");
    std::fs::write(
        &path,
        r#"[
            {"name":"quality","definition":"q","categories":[{"name":"good","definition":"g"}]},
            {"name":"safety","definition":"s","categories":[{"name":"safe","definition":"s"}]}
        ]"#,
    )
    .unwrap();

    let result =
        dcx::commands::analytics::categorical_eval::load_metrics_file(path.to_str().unwrap());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
    let _ = std::fs::remove_file(&path);
}

// ═══════════════════════════════════════════════
// --no-include-justification flag
// ═══════════════════════════════════════════════

#[tokio::test]
async fn categorical_eval_no_include_justification() {
    let executor = MockExecutor::empty(vec![]);
    let config = test_config(OutputFormat::Json);
    let metrics = vec![MetricDefinition {
        name: "quality".into(),
        definition: "Response quality".into(),
        categories: vec![CategoryDef {
            name: "good".into(),
            definition: "Good quality".into(),
        }],
        required: true,
    }];
    // include_justification = false should work and omit justifications
    let result = dcx::commands::analytics::categorical_eval::run_with_executor(
        &executor,
        &metrics,
        Some("7d".into()),
        None,
        100,
        false, // justification disabled
        false,
        None,
        None,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════
// Empty categories rejection
// ═══════════════════════════════════════════════

#[test]
fn load_metrics_file_rejects_empty_categories() {
    let dir = std::env::temp_dir().join("dcx_test_empty_cats");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("empty_cats.json");
    std::fs::write(
        &path,
        r#"[{"name":"quality","definition":"q","categories":[]}]"#,
    )
    .unwrap();

    let result =
        dcx::commands::analytics::categorical_eval::load_metrics_file(path.to_str().unwrap());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("has no categories"));
    let _ = std::fs::remove_file(&path);
}

// ═══════════════════════════════════════════════
// Custom event type passthrough (no uppercasing)
// ═══════════════════════════════════════════════

#[test]
fn build_create_view_sql_preserves_event_type_case() {
    // SDK passes event_type as-is; dcx should not uppercase it.
    let (view_name, sql) = dcx::commands::analytics::views::build_create_view_sql(
        "p",
        "d",
        "t",
        "",
        "My_Custom_Event",
    );
    // view_name still lowercases for the view identifier
    assert_eq!(view_name, "my_custom_event");
    // SQL WHERE clause must use the original case
    assert!(sql.contains("WHERE event_type = 'My_Custom_Event'"));
}

#[test]
fn is_known_event_type_case_insensitive() {
    // Known types should be recognized regardless of case
    assert!(is_known_event_type("llm_request"));
    assert!(is_known_event_type("LLM_REQUEST"));
    assert!(is_known_event_type("Llm_Request"));
    // Unknown types
    assert!(!is_known_event_type("CUSTOM_THING"));
}

// ═══════════════════════════════════════════════
// Milestone C: New evaluator SQL builders
// ═══════════════════════════════════════════════

#[test]
fn build_evaluate_query_turn_count() {
    let sql = build_evaluate_query(
        &EvaluatorType::TurnCount,
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        10.0,
        None,
        100,
    );
    assert!(sql.contains("turn_count"));
    assert!(sql.contains("HUMAN_INPUT_RECEIVED"));
    assert!(sql.contains("10"));
}

#[test]
fn build_evaluate_query_token_efficiency() {
    let sql = build_evaluate_query(
        &EvaluatorType::TokenEfficiency,
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        5000.0,
        None,
        100,
    );
    assert!(sql.contains("total_tokens"));
    assert!(sql.contains("5000"));
}

#[test]
fn build_evaluate_query_ttft() {
    let sql = build_evaluate_query(
        &EvaluatorType::Ttft,
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        500.0,
        None,
        100,
    );
    assert!(sql.contains("ttft_ms"));
    assert!(sql.contains("LLM_RESPONSE"));
    assert!(sql.contains("500"));
}

#[test]
fn build_evaluate_query_cost() {
    let sql = build_evaluate_query(
        &EvaluatorType::Cost,
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        1.5,
        None,
        100,
    );
    assert!(sql.contains("cost_usd"));
    assert!(sql.contains("1.5"));
}

// ═══════════════════════════════════════════════
// Milestone C: evaluate --limit applies LIMIT to SQL
// ═══════════════════════════════════════════════

#[test]
fn build_evaluate_query_applies_limit() {
    let sql = build_evaluate_query(
        &EvaluatorType::Latency,
        "proj",
        "ds",
        "events",
        "INTERVAL 24 HOUR",
        5000.0,
        None,
        42,
    );
    assert!(sql.contains("LIMIT 42"));
}

#[test]
fn build_distribution_query_applies_limit() {
    let sql = build_distribution_query("proj", "ds", "events", "INTERVAL 24 HOUR", None, 25);
    assert!(sql.contains("LIMIT 25"));
}

// ═══════════════════════════════════════════════
// Milestone C: evaluate rejects llm-judge
// ═══════════════════════════════════════════════

#[tokio::test]
async fn evaluate_rejects_llm_judge() {
    let config = test_config(OutputFormat::Json);
    let auth_opts = dcx::auth::AuthOptions {
        token: Some("fake".into()),
        credentials_file: None,
    };
    let result = dcx::commands::analytics::evaluate::run(
        EvaluatorType::LlmJudge,
        0.5,
        "7d".into(),
        None,
        false,
        "correctness".into(),
        100,
        false,
        None,
        None,
        &auth_opts,
        &config,
    )
    .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("llm-judge is not yet supported"));
}

// ═══════════════════════════════════════════════
// Milestone C: list-traces with session_id filter
// ═══════════════════════════════════════════════

#[test]
fn build_list_traces_query_with_session_filter() {
    let sql = build_list_traces_query(
        "proj",
        "ds",
        "events",
        "INTERVAL 7 DAY",
        None,
        Some("sess-42"),
        100,
    );
    assert!(sql.contains("AND session_id = 'sess-42'"));
    assert!(sql.contains("LIMIT 100"));
}
