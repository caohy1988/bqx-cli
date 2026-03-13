use clap::{CommandFactory, FromArgMatches};
use serde_json::json;

use bqx::auth;
use bqx::bigquery::discovery::{self, DiscoverySource};
use bqx::bigquery::dynamic::{clap_tree, executor, model};
use bqx::cli::{AnalyticsCommand, AuthCommand, Cli, Command, JobsCommand, OutputFormat};
use bqx::commands;
use bqx::config::Config;
use bqx::models::BqxError;

/// Names of static (derive-based) top-level subcommands.
const STATIC_COMMANDS: &[&str] = &["jobs", "analytics", "auth", "generate-skills"];

#[tokio::main]
async fn main() {
    // 1. Build a hybrid clap::Command: static derive tree + dynamic subcommands.
    //    Discovery loading is cheap (bundled include_str!) but isolated so that
    //    a bad bundled asset cannot brick static commands like auth/analytics.
    let (generated_commands, base_url) = match load_generated_commands() {
        Ok((cmds, url)) => (cmds, url),
        Err(e) => {
            eprintln!("Warning: could not load dynamic commands: {e}");
            (Vec::new(), String::new())
        }
    };

    let mut app = Cli::command();
    let dynamic_clap = clap_tree::build_dynamic_commands(&generated_commands);
    for sub in dynamic_clap {
        // Only add if it doesn't collide with a static command name.
        if !STATIC_COMMANDS.contains(&sub.get_name()) {
            app = app.subcommand(sub);
        }
    }

    // 2. Parse args with the augmented command.
    let matches = app.get_matches();

    // 3. Check if the matched subcommand is dynamic.
    if let Some((group_name, group_matches)) = matches.subcommand() {
        if !STATIC_COMMANDS.contains(&group_name) {
            // This is a dynamic command.
            run_dynamic(
                group_name,
                group_matches,
                &generated_commands,
                &base_url,
                &matches,
            )
            .await;
            return;
        }
    }

    // 4. Static path: reconstruct Cli from the already-parsed matches.
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
    };

    run_static(cli).await;
}

/// Load Discovery and build the generated command metadata.
/// Separated so that a failure here does not brick static commands.
fn load_generated_commands() -> anyhow::Result<(Vec<model::GeneratedCommand>, String)> {
    let doc = discovery::load(&DiscoverySource::Bundled)?;
    let base_url = doc.base_url.clone();
    let methods = model::extract_methods(&doc);
    let allowed = model::filter_allowed(&methods);
    let commands: Vec<model::GeneratedCommand> =
        allowed.iter().map(model::to_generated_command).collect();
    Ok((commands, base_url))
}

async fn run_dynamic(
    group_name: &str,
    group_matches: &clap::ArgMatches,
    generated_commands: &[model::GeneratedCommand],
    base_url: &str,
    root_matches: &clap::ArgMatches,
) {
    let (action_name, action_matches) = match group_matches.subcommand() {
        Some(pair) => pair,
        None => {
            eprintln!("{}", json!({"error": "No subcommand provided"}));
            std::process::exit(1);
        }
    };

    let cmd = match clap_tree::find_command(generated_commands, group_name, action_name) {
        Some(c) => c,
        None => {
            eprintln!(
                "{}",
                json!({"error": format!("Unknown command: {group_name} {action_name}")})
            );
            std::process::exit(1);
        }
    };

    // Extract global flags from root matches.
    // --project-id is required for all dynamic commands.
    let project_id = match root_matches.get_one::<String>("project_id") {
        Some(p) => p.clone(),
        None => {
            eprintln!(
                "{}",
                json!({"error": "--project-id or BQX_PROJECT is required"})
            );
            std::process::exit(1);
        }
    };

    let format = root_matches
        .get_one::<OutputFormat>("format")
        .cloned()
        .unwrap_or(OutputFormat::Json);

    let dry_run = action_matches.get_flag("dry-run");

    let auth_opts = auth::AuthOptions {
        token: root_matches.get_one::<String>("token").cloned(),
        credentials_file: root_matches.get_one::<String>("credentials_file").cloned(),
    };

    let mut args = clap_tree::extract_args(action_matches, cmd);

    // Inject global flag values for params that are skipped in clap generation.
    // Check if datasetId is a required path parameter for this command.
    let needs_dataset_id =
        cmd.method.parameters.iter().any(|p| {
            p.name == "datasetId" && p.location == model::ParamLocation::Path && p.required
        });

    if let Some(dataset_id) = root_matches.get_one::<String>("dataset_id") {
        args.entry("datasetId".to_string())
            .or_insert_with(|| dataset_id.clone());
    } else if needs_dataset_id && !args.contains_key("datasetId") {
        eprintln!(
            "{}",
            json!({"error": "--dataset-id or BQX_DATASET is required for this command"})
        );
        std::process::exit(1);
    }

    let sanitize_template = root_matches
        .get_one::<String>("sanitize")
        .map(|s| s.as_str());

    let result = executor::execute(
        cmd,
        &args,
        &project_id,
        base_url,
        &format,
        dry_run,
        &auth_opts,
        sanitize_template,
    )
    .await;

    if let Err(e) = result {
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
}

async fn run_static(cli: Cli) {
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

    // generate-skills doesn't need project/dataset config
    if let Command::GenerateSkills {
        ref output_dir,
        ref filter,
    } = cli.command
    {
        let result = commands::generate_skills::run(output_dir, filter, &cli.format);
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
        Command::Auth { .. } | Command::GenerateSkills { .. } => unreachable!(),
    };

    if let Err(e) = result {
        if let Some(BqxError::EvalFailed { exit_code }) = e.downcast_ref::<BqxError>() {
            std::process::exit(*exit_code);
        }
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
}
