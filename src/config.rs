use anyhow::{bail, Result};
use regex::Regex;

use crate::cli::Cli;
use crate::cli::OutputFormat;

pub struct Config {
    pub project_id: String,
    pub dataset_id: String,
    pub location: String,
    pub table: String,
    pub format: OutputFormat,
}

pub struct ParsedDuration {
    pub interval_sql: String,
}

impl Config {
    pub fn from_cli(cli: &Cli) -> Result<Config> {
        let project_id = cli
            .project_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--project-id or BQX_PROJECT is required"))?;
        let dataset_id = cli
            .dataset_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--dataset-id or BQX_DATASET is required"))?;

        validate_identifier(&project_id, "project_id")?;
        validate_identifier(&dataset_id, "dataset_id")?;
        validate_identifier(&cli.table, "table")?;

        Ok(Config {
            project_id,
            dataset_id,
            location: cli.location.clone(),
            table: cli.table.clone(),
            format: cli.format.clone(),
        })
    }
}

pub fn parse_duration(s: &str) -> Result<ParsedDuration> {
    let re = Regex::new(r"^(\d+)(h|d|m)$").unwrap();
    let caps = re
        .captures(s)
        .ok_or_else(|| anyhow::anyhow!("Invalid duration format: {s}. Expected: 1h, 24h, 7d"))?;

    let n = &caps[1];
    let unit = match &caps[2] {
        "h" => "HOUR",
        "d" => "DAY",
        "m" => "MINUTE",
        _ => bail!("Invalid duration unit"),
    };

    Ok(ParsedDuration {
        interval_sql: format!("INTERVAL {n} {unit}"),
    })
}

pub fn validate_identifier(s: &str, name: &str) -> Result<()> {
    let re = Regex::new(r"^[a-zA-Z0-9_][a-zA-Z0-9_\-]*$").unwrap();
    if !re.is_match(s) {
        bail!("Invalid {name}: '{s}'. Must be alphanumeric with underscores/hyphens.");
    }
    Ok(())
}

pub fn validate_session_id(s: &str) -> Result<()> {
    let re = Regex::new(r"^[a-zA-Z0-9_.\-]+$").unwrap();
    if !re.is_match(s) {
        bail!("Invalid session_id: '{s}'. Must be alphanumeric with underscores, dots, and hyphens.");
    }
    Ok(())
}

pub fn validate_agent_id(s: &str) -> Result<()> {
    let re = Regex::new(r"^[a-zA-Z0-9_.\-]+$").unwrap();
    if !re.is_match(s) {
        bail!("Invalid agent_id: '{s}'. Must be alphanumeric with underscores, dots, and hyphens.");
    }
    Ok(())
}
