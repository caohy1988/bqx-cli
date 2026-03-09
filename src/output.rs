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
            // For table format, serialize to JSON then render
            let json = serde_json::to_value(value)?;
            render_value_as_table(&json)?;
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
