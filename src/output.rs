use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use serde::Serialize;

use crate::cli::OutputFormat;

pub fn render<T: Serialize>(value: &T, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(value)?;
            println!("{json}");
        }
        OutputFormat::Table => {
            let json = serde_json::to_value(value)?;
            render_value_as_table(&json)?;
        }
        OutputFormat::Text => {
            anyhow::bail!(
                "Text format requires command-specific rendering; \
                 this is a bug if you see it"
            );
        }
    }
    Ok(())
}

pub fn render_rows_as_table(columns: &[String], rows: &[Vec<String>]) -> Result<()> {
    println!("{}", fmt_rows_as_table(columns, rows));
    Ok(())
}

pub fn fmt_rows_as_table(columns: &[String], rows: &[Vec<String>]) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(columns);
    for row in rows {
        table.add_row(row);
    }
    table.to_string()
}

fn render_value_as_table(value: &serde_json::Value) -> Result<()> {
    println!("{}", fmt_value_as_table(value)?);
    Ok(())
}

pub fn fmt_value_as_table(value: &serde_json::Value) -> Result<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in ["sessions", "events", "rows", "results"] {
                if let Some(serde_json::Value::Array(arr)) = map.get(key) {
                    if let Some(serde_json::Value::Object(first_map)) = arr.first() {
                        let columns: Vec<String> = first_map.keys().cloned().collect();
                        let rows: Vec<Vec<String>> = arr
                            .iter()
                            .map(|item| {
                                columns
                                    .iter()
                                    .map(|col| format_cell(item.get(col)))
                                    .collect()
                            })
                            .collect();
                        return Ok(fmt_rows_as_table(&columns, &rows));
                    }
                }
            }
            Ok(fmt_kv_table(map))
        }
        serde_json::Value::Array(arr) => {
            if let Some(serde_json::Value::Object(first)) = arr.first() {
                let columns: Vec<String> = first.keys().cloned().collect();
                let rows: Vec<Vec<String>> = arr
                    .iter()
                    .map(|item| {
                        columns
                            .iter()
                            .map(|col| format_cell(item.get(col)))
                            .collect()
                    })
                    .collect();
                Ok(fmt_rows_as_table(&columns, &rows))
            } else {
                Ok(serde_json::to_string_pretty(&arr)?)
            }
        }
        _ => Ok(serde_json::to_string_pretty(value)?),
    }
}

pub fn fmt_kv_table(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["Field", "Value"]);
    for (key, value) in map {
        table.add_row(vec![key.clone(), format_cell(Some(value))]);
    }
    table.to_string()
}

fn format_cell(value: Option<&serde_json::Value>) -> String {
    match value {
        None | Some(serde_json::Value::Null) => "null".to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(v) => serde_json::to_string(v).unwrap_or_default(),
    }
}

// ── Command-specific text renderers ──
//
// Each renderer writes to a `&mut dyn std::fmt::Write` for testability,
// with a public wrapper that writes to stdout.

pub mod text {
    use std::fmt::Write;

    use crate::ca::models::CaQuestionResponse;
    use crate::commands::analytics::doctor::DoctorReport;
    use crate::commands::analytics::evaluate::{EvalResult, SessionEval};
    use crate::commands::analytics::get_trace::{TraceEvent, TraceResult};

    pub fn render_query_dry_run(url: &str, query: &str, legacy_sql: bool, location: &str) {
        let mut buf = String::new();
        fmt_query_dry_run(&mut buf, url, query, legacy_sql, location);
        print!("{buf}");
    }

    pub fn render_query(total_rows: u64, columns: &[String], rows: &[Vec<String>]) {
        let mut buf = String::new();
        fmt_query(&mut buf, total_rows, columns, rows);
        print!("{buf}");
    }

    pub fn render_doctor(report: &DoctorReport) {
        let mut buf = String::new();
        fmt_doctor(&mut buf, report);
        print!("{buf}");
    }

    pub fn render_evaluate(result: &EvalResult) {
        let mut buf = String::new();
        fmt_evaluate(&mut buf, result);
        print!("{buf}");
    }

    pub fn render_trace(trace: &TraceResult) {
        let mut buf = String::new();
        fmt_trace(&mut buf, trace);
        print!("{buf}");
    }

    pub fn render_ca_ask(resp: &CaQuestionResponse) {
        let mut buf = String::new();
        fmt_ca_ask(&mut buf, resp);
        print!("{buf}");
    }

    // ── Formatting functions (write to any fmt::Write) ──

    pub fn fmt_query_dry_run(
        w: &mut dyn Write,
        url: &str,
        query: &str,
        legacy_sql: bool,
        location: &str,
    ) {
        let _ = writeln!(w, "Dry run: POST {url}");
        let _ = writeln!(w, "Query: {query}");
        let _ = writeln!(w, "Legacy SQL: {legacy_sql}");
        let _ = writeln!(w, "Location: {location}");
    }

    pub fn fmt_query(w: &mut dyn Write, total_rows: u64, columns: &[String], rows: &[Vec<String>]) {
        let _ = writeln!(w, "Query complete: {total_rows} rows");
        if columns.is_empty() {
            return;
        }
        let _ = writeln!(w, "Columns: {}", columns.join(", "));
        for (i, row) in rows.iter().enumerate() {
            let _ = writeln!(w, "Row {}: {}", i + 1, row.join(" | "));
        }
    }

    pub fn fmt_doctor(w: &mut dyn Write, report: &DoctorReport) {
        let _ = writeln!(w, "Status: {}", report.status);
        let _ = writeln!(w, "Table: {}", report.table);
        let _ = writeln!(
            w,
            "Rows: {}  Sessions: {}  Agents: {}",
            report.total_rows, report.distinct_sessions, report.distinct_agents
        );
        if let Some(ref latest) = report.latest_event {
            let _ = writeln!(w, "Latest event: {latest}");
        }
        for warning in &report.warnings {
            let _ = writeln!(w, "Warning: {warning}");
        }
        if !report.missing_required_columns.is_empty() {
            let _ = writeln!(
                w,
                "Missing columns: {}",
                report.missing_required_columns.join(", ")
            );
        }
    }

    pub fn fmt_evaluate(w: &mut dyn Write, result: &EvalResult) {
        let _ = writeln!(
            w,
            "Evaluator: {}  Threshold: {}  Window: {}",
            result.evaluator, result.threshold, result.time_window
        );
        let _ = writeln!(
            w,
            "Sessions: {}  Passed: {}  Failed: {}  Pass rate: {:.2}",
            result.total_sessions, result.passed, result.failed, result.pass_rate
        );
        let worst: Vec<&SessionEval> = result.sessions.iter().filter(|s| !s.passed).collect();
        if !worst.is_empty() {
            let _ = writeln!(w, "Worst sessions:");
            for s in worst {
                let _ = writeln!(w, "- {}  {}  score={:.1}", s.session_id, s.agent, s.score);
            }
        }
    }

    pub fn fmt_trace(w: &mut dyn Write, trace: &TraceResult) {
        let _ = writeln!(w, "Session: {}", trace.session_id);
        let _ = writeln!(w, "Agent: {}", trace.agent);
        let _ = writeln!(
            w,
            "Events: {}  Errors: {}",
            trace.event_count, trace.has_errors
        );
        for event in &trace.events {
            fmt_trace_event(w, event);
        }
    }

    fn fmt_trace_event(w: &mut dyn Write, e: &TraceEvent) {
        let status = e.status.as_deref().unwrap_or("OK");
        let latency_part = e.latency_ms.as_ref().and_then(|v| {
            if let Some(obj) = v.as_object() {
                obj.get("total_ms").map(|ms| format!("latency={ms}"))
            } else if v.is_number() {
                Some(format!("latency={v}"))
            } else {
                None
            }
        });
        match latency_part {
            Some(lat) => {
                let _ = writeln!(w, "{}  {:<24}{} {}", e.timestamp, e.event_type, status, lat);
            }
            None => {
                let _ = writeln!(w, "{}  {:<24}{}", e.timestamp, e.event_type, status);
            }
        }
    }

    pub fn fmt_ca_ask(w: &mut dyn Write, resp: &CaQuestionResponse) {
        let _ = writeln!(w, "Question: {}", resp.question);
        if let Some(ref agent) = resp.agent {
            let _ = writeln!(w, "Agent: {agent}");
        }
        if let Some(ref sql) = resp.sql {
            let _ = writeln!(w, "SQL: {sql}");
        }
        if let Some(ref explanation) = resp.explanation {
            let _ = writeln!(w, "Explanation: {explanation}");
        }
        if resp.results.is_empty() {
            let _ = writeln!(w, "Results: (none)");
        } else {
            let _ = writeln!(w, "Results: {} rows", resp.results.len());
            // Print column headers from first row
            if let Some(first) = resp.results.first() {
                let cols: Vec<&String> = first.keys().collect();
                for (i, row) in resp.results.iter().enumerate() {
                    let vals: Vec<String> = cols
                        .iter()
                        .map(|col| {
                            row.get(*col)
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    serde_json::Value::Null => "null".to_string(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_else(|| "null".to_string())
                        })
                        .collect();
                    let _ = writeln!(w, "Row {}: {}", i + 1, vals.join(" | "));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::text::*;
    use crate::ca::models::CaQuestionResponse;
    use crate::commands::analytics::doctor::{DoctorReport, NullChecks};
    use crate::commands::analytics::evaluate::{EvalResult, SessionEval};
    use crate::commands::analytics::get_trace::{TraceEvent, TraceResult};

    #[test]
    fn text_doctor_healthy() {
        let report = DoctorReport {
            status: "healthy".into(),
            table: "proj.dataset.events".into(),
            total_rows: 296,
            distinct_sessions: 12,
            distinct_agents: 1,
            earliest_event: Some("2026-03-01 00:00:00.000 UTC".into()),
            latest_event: Some("2026-03-05 09:27:54.474 UTC".into()),
            minutes_since_last_event: Some(30),
            null_checks: NullChecks {
                session_id: 0,
                agent: 0,
                event_type: 0,
                timestamp: 0,
            },
            distinct_event_types: 5,
            columns: vec!["session_id".into(), "agent".into()],
            missing_required_columns: vec![],
            warnings: vec![],
        };
        let mut buf = String::new();
        fmt_doctor(&mut buf, &report);
        assert!(buf.contains("Status: healthy"));
        assert!(buf.contains("Table: proj.dataset.events"));
        assert!(buf.contains("Rows: 296  Sessions: 12  Agents: 1"));
        assert!(buf.contains("Latest event: 2026-03-05 09:27:54.474 UTC"));
        assert!(!buf.contains("Warning:"));
        assert!(!buf.contains("Missing columns:"));
    }

    #[test]
    fn text_doctor_warning_with_stale_data() {
        let report = DoctorReport {
            status: "warning".into(),
            table: "proj.dataset.events".into(),
            total_rows: 100,
            distinct_sessions: 5,
            distinct_agents: 2,
            earliest_event: None,
            latest_event: Some("2026-03-01 00:00:00.000 UTC".into()),
            minutes_since_last_event: Some(5659),
            null_checks: NullChecks {
                session_id: 0,
                agent: 0,
                event_type: 0,
                timestamp: 0,
            },
            distinct_event_types: 3,
            columns: vec![],
            missing_required_columns: vec![],
            warnings: vec!["No recent data — last event was 5659 minutes ago.".into()],
        };
        let mut buf = String::new();
        fmt_doctor(&mut buf, &report);
        assert!(buf.contains("Status: warning"));
        assert!(buf.contains("Warning: No recent data"));
    }

    #[test]
    fn text_doctor_error_missing_columns() {
        let report = DoctorReport {
            status: "error".into(),
            table: "proj.dataset.events".into(),
            total_rows: 0,
            distinct_sessions: 0,
            distinct_agents: 0,
            earliest_event: None,
            latest_event: None,
            minutes_since_last_event: None,
            null_checks: NullChecks {
                session_id: 0,
                agent: 0,
                event_type: 0,
                timestamp: 0,
            },
            distinct_event_types: 0,
            columns: vec![],
            missing_required_columns: vec!["session_id".into(), "agent".into()],
            warnings: vec!["Missing required columns: session_id, agent".into()],
        };
        let mut buf = String::new();
        fmt_doctor(&mut buf, &report);
        assert!(buf.contains("Status: error"));
        assert!(buf.contains("Missing columns: session_id, agent"));
    }

    #[test]
    fn text_evaluate_all_passed() {
        let result = EvalResult {
            evaluator: "latency".into(),
            threshold: 5000.0,
            time_window: "7d".into(),
            agent_id: None,
            total_sessions: 3,
            passed: 3,
            failed: 0,
            pass_rate: 1.0,
            sessions: vec![
                SessionEval {
                    session_id: "s1".into(),
                    agent: "agent_a".into(),
                    passed: true,
                    score: 1200.0,
                    no_latency_data: false,
                },
                SessionEval {
                    session_id: "s2".into(),
                    agent: "agent_a".into(),
                    passed: true,
                    score: 800.0,
                    no_latency_data: false,
                },
            ],
        };
        let mut buf = String::new();
        fmt_evaluate(&mut buf, &result);
        assert!(buf.contains("Evaluator: latency  Threshold: 5000  Window: 7d"));
        assert!(buf.contains("Sessions: 3  Passed: 3  Failed: 0  Pass rate: 1.00"));
        assert!(!buf.contains("Worst sessions:"));
    }

    #[test]
    fn text_evaluate_with_failures() {
        let result = EvalResult {
            evaluator: "latency".into(),
            threshold: 5000.0,
            time_window: "30d".into(),
            agent_id: None,
            total_sessions: 12,
            passed: 0,
            failed: 12,
            pass_rate: 0.0,
            sessions: vec![
                SessionEval {
                    session_id: "adcp-a20d".into(),
                    agent: "sales_agent".into(),
                    passed: false,
                    score: 32135.0,
                    no_latency_data: false,
                },
                SessionEval {
                    session_id: "adcp-affa".into(),
                    agent: "sales_agent".into(),
                    passed: false,
                    score: 26848.0,
                    no_latency_data: false,
                },
            ],
        };
        let mut buf = String::new();
        fmt_evaluate(&mut buf, &result);
        assert!(buf.contains("Failed: 12  Pass rate: 0.00"));
        assert!(buf.contains("Worst sessions:"));
        assert!(buf.contains("- adcp-a20d  sales_agent  score=32135.0"));
        assert!(buf.contains("- adcp-affa  sales_agent  score=26848.0"));
    }

    #[test]
    fn text_evaluate_error_rate() {
        let result = EvalResult {
            evaluator: "error_rate".into(),
            threshold: 0.1,
            time_window: "24h".into(),
            agent_id: Some("my_agent".into()),
            total_sessions: 5,
            passed: 4,
            failed: 1,
            pass_rate: 0.8,
            sessions: vec![SessionEval {
                session_id: "s-bad".into(),
                agent: "my_agent".into(),
                passed: false,
                score: 0.5,
                no_latency_data: false,
            }],
        };
        let mut buf = String::new();
        fmt_evaluate(&mut buf, &result);
        assert!(buf.contains("Evaluator: error_rate  Threshold: 0.1  Window: 24h"));
        assert!(buf.contains("- s-bad  my_agent  score=0.5"));
    }

    #[test]
    fn text_trace_basic() {
        let trace = TraceResult {
            session_id: "adcp-a20d176b82af".into(),
            agent: "yahoo_sales_agent".into(),
            event_count: 3,
            started_at: Some("2026-03-05 09:26:59.270 UTC".into()),
            ended_at: Some("2026-03-05 09:27:17.494 UTC".into()),
            has_errors: false,
            events: vec![
                TraceEvent {
                    event_type: "LLM_REQUEST".into(),
                    timestamp: "2026-03-05 09:26:59.270 UTC".into(),
                    status: Some("OK".into()),
                    error_message: None,
                    latency_ms: None,
                    content: None,
                },
                TraceEvent {
                    event_type: "LLM_RESPONSE".into(),
                    timestamp: "2026-03-05 09:27:03.208 UTC".into(),
                    status: Some("OK".into()),
                    error_message: None,
                    latency_ms: Some(serde_json::json!({"total_ms": 3938})),
                    content: None,
                },
                TraceEvent {
                    event_type: "INVOCATION_COMPLETED".into(),
                    timestamp: "2026-03-05 09:27:17.494 UTC".into(),
                    status: Some("OK".into()),
                    error_message: None,
                    latency_ms: Some(serde_json::json!({"total_ms": 32135})),
                    content: None,
                },
            ],
        };
        let mut buf = String::new();
        fmt_trace(&mut buf, &trace);
        assert!(buf.contains("Session: adcp-a20d176b82af"));
        assert!(buf.contains("Agent: yahoo_sales_agent"));
        assert!(buf.contains("Events: 3  Errors: false"));
        assert!(buf.contains("LLM_REQUEST"));
        assert!(buf.contains("LLM_RESPONSE"));
        assert!(buf.contains("latency=3938"));
        assert!(buf.contains("latency=32135"));
    }

    #[test]
    fn text_trace_with_errors() {
        let trace = TraceResult {
            session_id: "s-err".into(),
            agent: "agent_x".into(),
            event_count: 1,
            started_at: Some("2026-03-05 10:00:00.000 UTC".into()),
            ended_at: Some("2026-03-05 10:00:00.000 UTC".into()),
            has_errors: true,
            events: vec![TraceEvent {
                event_type: "TOOL_ERROR".into(),
                timestamp: "2026-03-05 10:00:00.000 UTC".into(),
                status: Some("ERROR".into()),
                error_message: Some("connection refused".into()),
                latency_ms: None,
                content: None,
            }],
        };
        let mut buf = String::new();
        fmt_trace(&mut buf, &trace);
        assert!(buf.contains("Events: 1  Errors: true"));
        assert!(buf.contains("TOOL_ERROR"));
        assert!(buf.contains("ERROR"));
    }

    #[test]
    fn text_trace_no_status_defaults_to_ok() {
        let trace = TraceResult {
            session_id: "s1".into(),
            agent: "a1".into(),
            event_count: 1,
            started_at: None,
            ended_at: None,
            has_errors: false,
            events: vec![TraceEvent {
                event_type: "LLM_REQUEST".into(),
                timestamp: "2026-03-05 10:00:00.000 UTC".into(),
                status: None,
                error_message: None,
                latency_ms: None,
                content: None,
            }],
        };
        let mut buf = String::new();
        fmt_trace(&mut buf, &trace);
        assert!(buf.contains("OK"), "Missing status should default to OK");
    }

    #[test]
    fn text_query_with_rows() {
        let columns = vec!["session_id".into(), "agent".into(), "event_type".into()];
        let rows = vec![
            vec!["s1".into(), "agent_a".into(), "LLM_REQUEST".into()],
            vec!["s2".into(), "agent_b".into(), "TOOL_CALL".into()],
        ];
        let mut buf = String::new();
        fmt_query(&mut buf, 2, &columns, &rows);
        assert!(buf.contains("Query complete: 2 rows"));
        assert!(buf.contains("Columns: session_id, agent, event_type"));
        assert!(buf.contains("Row 1: s1 | agent_a | LLM_REQUEST"));
        assert!(buf.contains("Row 2: s2 | agent_b | TOOL_CALL"));
    }

    #[test]
    fn text_query_empty_result() {
        let mut buf = String::new();
        fmt_query(&mut buf, 0, &[], &[]);
        assert_eq!(buf.trim(), "Query complete: 0 rows");
    }

    #[test]
    fn text_ca_ask_with_results() {
        let resp = CaQuestionResponse {
            question: "error rate for support_bot?".into(),
            agent: Some("agent-analytics".into()),
            sql: Some("SELECT error_rate FROM t".into()),
            results: vec![{
                let mut map = serde_json::Map::new();
                map.insert("error_rate".into(), serde_json::json!(0.05));
                map
            }],
            explanation: Some("Shows the error rate".into()),
        };
        let mut buf = String::new();
        fmt_ca_ask(&mut buf, &resp);
        assert!(buf.contains("Question: error rate for support_bot?"));
        assert!(buf.contains("Agent: agent-analytics"));
        assert!(buf.contains("SQL: SELECT error_rate FROM t"));
        assert!(buf.contains("Explanation: Shows the error rate"));
        assert!(buf.contains("Results: 1 rows"));
        assert!(buf.contains("Row 1:"));
    }

    #[test]
    fn text_ca_ask_no_results() {
        let resp = CaQuestionResponse {
            question: "test?".into(),
            agent: None,
            sql: Some("SELECT 1".into()),
            results: vec![],
            explanation: None,
        };
        let mut buf = String::new();
        fmt_ca_ask(&mut buf, &resp);
        assert!(buf.contains("Question: test?"));
        assert!(!buf.contains("Agent:"));
        assert!(buf.contains("Results: (none)"));
        assert!(!buf.contains("Explanation:"));
    }
}
