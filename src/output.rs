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
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(columns);
    for row in rows {
        table.add_row(row);
    }
    println!("{table}");
    Ok(())
}

fn render_value_as_table(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Object(map) => {
            // Check if there is a list field we should render as a table
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
                        return render_rows_as_table(&columns, &rows);
                    }
                }
            }
            // Fall back to key-value table
            render_kv_table(map)
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
                render_rows_as_table(&columns, &rows)
            } else {
                // Plain JSON fallback
                println!("{}", serde_json::to_string_pretty(&arr)?);
                Ok(())
            }
        }
        _ => {
            println!("{}", serde_json::to_string_pretty(value)?);
            Ok(())
        }
    }
}

fn render_kv_table(map: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec!["Field", "Value"]);
    for (key, value) in map {
        table.add_row(vec![key.clone(), format_cell(Some(value))]);
    }
    println!("{table}");
    Ok(())
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

pub mod text {
    use crate::commands::analytics::doctor::DoctorReport;
    use crate::commands::analytics::evaluate::{EvalResult, SessionEval};
    use crate::commands::analytics::get_trace::{TraceEvent, TraceResult};

    pub fn render_query_dry_run(url: &str, query: &str, legacy_sql: bool, location: &str) {
        println!("Dry run: POST {url}");
        println!("Query: {query}");
        println!("Legacy SQL: {legacy_sql}");
        println!("Location: {location}");
    }

    pub fn render_query(total_rows: u64, columns: &[String], rows: &[Vec<String>]) {
        println!("Query complete: {total_rows} rows");
        if columns.is_empty() {
            return;
        }
        println!("Columns: {}", columns.join(", "));
        for (i, row) in rows.iter().enumerate() {
            println!("Row {}: {}", i + 1, row.join(" | "));
        }
    }

    pub fn render_doctor(report: &DoctorReport) {
        println!("Status: {}", report.status);
        println!("Table: {}", report.table);
        println!(
            "Rows: {}  Sessions: {}  Agents: {}",
            report.total_rows, report.distinct_sessions, report.distinct_agents
        );
        if let Some(ref latest) = report.latest_event {
            println!("Latest event: {latest}");
        }
        for warning in &report.warnings {
            println!("Warning: {warning}");
        }
        if !report.missing_required_columns.is_empty() {
            println!(
                "Missing columns: {}",
                report.missing_required_columns.join(", ")
            );
        }
    }

    pub fn render_evaluate(result: &EvalResult) {
        println!(
            "Evaluator: {}  Threshold: {}  Window: {}",
            result.evaluator, result.threshold, result.time_window
        );
        println!(
            "Sessions: {}  Passed: {}  Failed: {}  Pass rate: {:.2}",
            result.total_sessions, result.passed, result.failed, result.pass_rate
        );
        let worst: Vec<&SessionEval> = result.sessions.iter().filter(|s| !s.passed).collect();
        if !worst.is_empty() {
            println!("Worst sessions:");
            for s in worst {
                println!("- {}  {}  score={:.1}", s.session_id, s.agent, s.score);
            }
        }
    }

    pub fn render_trace(trace: &TraceResult) {
        println!("Session: {}", trace.session_id);
        println!("Agent: {}", trace.agent);
        println!(
            "Events: {}  Errors: {}",
            trace.event_count, trace.has_errors
        );
        for event in &trace.events {
            print_trace_event(event);
        }
    }

    fn print_trace_event(e: &TraceEvent) {
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
            Some(lat) => println!("{}  {:<24}{} {}", e.timestamp, e.event_type, status, lat),
            None => println!("{}  {:<24}{}", e.timestamp, e.event_type, status),
        }
    }
}
