use anyhow::Result;
use serde::Serialize;

use crate::bigquery::client::{BigQueryClient, QueryRequest};
use crate::config::Config;
use crate::output;

const DOCTOR_SQL: &str = r#"
SELECT
  COUNT(*) AS total_rows,
  COUNT(DISTINCT session_id) AS distinct_sessions,
  COUNT(DISTINCT agent) AS distinct_agents,
  MIN(timestamp) AS earliest_event,
  MAX(timestamp) AS latest_event,
  TIMESTAMP_DIFF(CURRENT_TIMESTAMP(), MAX(timestamp), MINUTE) AS minutes_since_last_event,
  COUNTIF(session_id IS NULL) AS null_session_ids,
  COUNTIF(agent IS NULL) AS null_agents,
  COUNTIF(event_type IS NULL) AS null_event_types,
  COUNTIF(timestamp IS NULL) AS null_timestamps,
  COUNT(DISTINCT event_type) AS distinct_event_types
FROM `{project}.{dataset}.{table}`
"#;

const COLUMNS_SQL: &str = r#"
SELECT column_name
FROM `{project}.{dataset}.INFORMATION_SCHEMA.COLUMNS`
WHERE table_name = '{table}'
ORDER BY ordinal_position
"#;

#[derive(Serialize)]
pub struct DoctorReport {
    pub status: String,
    pub table: String,
    pub total_rows: u64,
    pub distinct_sessions: u64,
    pub distinct_agents: u64,
    pub earliest_event: Option<String>,
    pub latest_event: Option<String>,
    pub minutes_since_last_event: Option<i64>,
    pub null_checks: NullChecks,
    pub distinct_event_types: u64,
    pub columns: Vec<String>,
    pub missing_required_columns: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct NullChecks {
    pub session_id: u64,
    pub agent: u64,
    pub event_type: u64,
    pub timestamp: u64,
}

const REQUIRED_COLUMNS: &[&str] = &["session_id", "agent", "event_type", "timestamp"];

pub async fn run(config: &Config) -> Result<()> {
    let client = BigQueryClient::new().await?;
    let full_table = format!("{}.{}.{}", config.project_id, config.dataset_id, config.table);

    // Query columns from INFORMATION_SCHEMA
    let columns_sql = COLUMNS_SQL
        .replace("{project}", &config.project_id)
        .replace("{dataset}", &config.dataset_id)
        .replace("{table}", &config.table);

    let columns_result = client
        .query(
            &config.project_id,
            QueryRequest {
                query: columns_sql,
                use_legacy_sql: false,
                location: config.location.clone(),
                max_results: None,
                timeout_ms: Some(30000),
            },
        )
        .await?;

    let columns: Vec<String> = columns_result
        .rows
        .iter()
        .filter_map(|row| {
            row.get("column_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    let missing_required: Vec<String> = REQUIRED_COLUMNS
        .iter()
        .filter(|col| !columns.iter().any(|c| c == **col))
        .map(|s| s.to_string())
        .collect();

    if !missing_required.is_empty() {
        let report = DoctorReport {
            status: "error".into(),
            table: full_table,
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
            columns,
            missing_required_columns: missing_required.clone(),
            warnings: vec![format!(
                "Missing required columns: {}",
                missing_required.join(", ")
            )],
        };
        output::render(&report, &config.format)?;
        return Ok(());
    }

    // Query stats
    let stats_sql = DOCTOR_SQL
        .replace("{project}", &config.project_id)
        .replace("{dataset}", &config.dataset_id)
        .replace("{table}", &config.table);

    let stats_result = client
        .query(
            &config.project_id,
            QueryRequest {
                query: stats_sql,
                use_legacy_sql: false,
                location: config.location.clone(),
                max_results: None,
                timeout_ms: Some(30000),
            },
        )
        .await?;

    let row = stats_result
        .rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("Doctor query returned no rows"))?;

    let get_u64 = |key: &str| -> u64 {
        row.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };

    let get_str = |key: &str| -> Option<String> {
        row.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let get_i64 = |key: &str| -> Option<i64> {
        row.get(key)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
    };

    let total_rows = get_u64("total_rows");
    let minutes_since = get_i64("minutes_since_last_event");

    let mut warnings = Vec::new();
    if total_rows == 0 {
        warnings.push("Table is empty — no events found.".into());
    }
    if get_u64("null_session_ids") > 0 {
        warnings.push(format!("{} rows have NULL session_id.", get_u64("null_session_ids")));
    }
    if get_u64("null_agents") > 0 {
        warnings.push(format!("{} rows have NULL agent.", get_u64("null_agents")));
    }
    if get_u64("null_event_types") > 0 {
        warnings.push(format!("{} rows have NULL event_type.", get_u64("null_event_types")));
    }
    if let Some(mins) = minutes_since {
        if mins > 60 {
            warnings.push(format!(
                "No recent data — last event was {mins} minutes ago."
            ));
        }
    }

    let status = if total_rows == 0
        || get_u64("null_session_ids") > 0
        || get_u64("null_event_types") > 0
        || get_u64("null_timestamps") > 0
    {
        "error"
    } else if minutes_since.map_or(false, |m| m > 60) || !warnings.is_empty() {
        "warning"
    } else {
        "healthy"
    };

    let report = DoctorReport {
        status: status.into(),
        table: full_table,
        total_rows,
        distinct_sessions: get_u64("distinct_sessions"),
        distinct_agents: get_u64("distinct_agents"),
        earliest_event: get_str("earliest_event"),
        latest_event: get_str("latest_event"),
        minutes_since_last_event: minutes_since,
        null_checks: NullChecks {
            session_id: get_u64("null_session_ids"),
            agent: get_u64("null_agents"),
            event_type: get_u64("null_event_types"),
            timestamp: get_u64("null_timestamps"),
        },
        distinct_event_types: get_u64("distinct_event_types"),
        columns,
        missing_required_columns: vec![],
        warnings,
    };

    output::render(&report, &config.format)?;
    Ok(())
}
