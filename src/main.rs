mod auth;
mod bigquery;
mod cli;
mod commands;
mod config;
mod models;
mod output;

use clap::Parser;
use serde_json::json;

use cli::{AnalyticsCommand, Cli, Command, JobsCommand};
use config::Config;
use models::BqxError;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = match Config::from_cli(&cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        Command::Jobs {
            command:
                JobsCommand::Query {
                    query,
                    use_legacy_sql,
                    dry_run,
                },
        } => commands::jobs_query::run(query, use_legacy_sql, dry_run, &config).await,
        Command::Analytics { command } => match command {
            AnalyticsCommand::Doctor => commands::analytics::doctor::run(&config).await,
            AnalyticsCommand::Evaluate {
                evaluator,
                threshold,
                last,
                agent_id,
                exit_code,
            } => {
                commands::analytics::evaluate::run(
                    evaluator, threshold, last, agent_id, exit_code, &config,
                )
                .await
            }
            AnalyticsCommand::GetTrace { session_id } => {
                commands::analytics::get_trace::run(session_id, &config).await
            }
        },
    };

    if let Err(e) = result {
        if let Some(BqxError::EvalFailed { exit_code }) = e.downcast_ref::<BqxError>() {
            std::process::exit(*exit_code);
        }
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
}
