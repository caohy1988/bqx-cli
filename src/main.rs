use clap::Parser;
use serde_json::json;

use bqx::auth;
use bqx::cli::{AnalyticsCommand, AuthCommand, Cli, Command, JobsCommand};
use bqx::commands;
use bqx::config::Config;
use bqx::models::BqxError;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Auth commands don't need project/dataset config
    if let Command::Auth { ref command } = cli.command {
        let auth_opts = auth::AuthOptions {
            token: cli.token.clone(),
            credentials_file: cli.credentials_file.clone(),
        };
        let result = match command {
            AuthCommand::Login => auth::login::run_login().await,
            AuthCommand::Logout => auth::login::run_logout(),
            AuthCommand::Status => auth::login::run_status(&auth_opts).await,
        };
        if let Err(e) = result {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
        return;
    }

    let config = match Config::from_cli(&cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
    };

    // Resolve auth for data commands
    let auth_opts = auth::AuthOptions {
        token: cli.token.clone(),
        credentials_file: cli.credentials_file.clone(),
    };

    let result = match cli.command {
        Command::Jobs {
            command:
                JobsCommand::Query {
                    query,
                    use_legacy_sql,
                    dry_run,
                },
        } => commands::jobs_query::run(query, use_legacy_sql, dry_run, &auth_opts, &config).await,
        Command::Analytics { command } => match command {
            AnalyticsCommand::Doctor => commands::analytics::doctor::run(&auth_opts, &config).await,
            AnalyticsCommand::Evaluate {
                evaluator,
                threshold,
                last,
                agent_id,
                exit_code,
            } => {
                commands::analytics::evaluate::run(
                    evaluator, threshold, last, agent_id, exit_code, &auth_opts, &config,
                )
                .await
            }
            AnalyticsCommand::GetTrace { session_id } => {
                commands::analytics::get_trace::run(session_id, &auth_opts, &config).await
            }
        },
        Command::Auth { .. } => unreachable!(),
    };

    if let Err(e) = result {
        if let Some(BqxError::EvalFailed { exit_code }) = e.downcast_ref::<BqxError>() {
            std::process::exit(*exit_code);
        }
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
}
