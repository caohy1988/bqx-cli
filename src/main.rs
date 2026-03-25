use clap::{CommandFactory, FromArgMatches};
use serde_json::json;

use dcx::auth;
use dcx::bigquery::discovery::{self, DiscoverySource};
use dcx::bigquery::dynamic::{clap_tree, executor, model};
use dcx::cli::{
    AnalyticsCommand, AuthCommand, CaCommand, Cli, Command, JobsCommand, LookerCommand,
    LookerDashboardsCommand, LookerExploresCommand, OutputFormat, ProfilesCommand, ShellType,
    ViewsCommand,
};
use dcx::commands;
use dcx::config::Config;
use dcx::models::BqxError;

/// Names of static (derive-based) top-level subcommands.
const STATIC_COMMANDS: &[&str] = &[
    "jobs",
    "analytics",
    "ca",
    "auth",
    "generate-skills",
    "completions",
    "profiles",
    "looker",
];

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
                json!({"error": "--project-id or DCX_PROJECT is required"})
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
            json!({"error": "--dataset-id or DCX_DATASET is required for this command"})
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

    // completions doesn't need project/dataset config
    if let Command::Completions { ref shell } = cli.command {
        let shell = match shell {
            ShellType::Bash => clap_complete::Shell::Bash,
            ShellType::Zsh => clap_complete::Shell::Zsh,
            ShellType::Fish => clap_complete::Shell::Fish,
        };
        // Build the augmented command tree (static + dynamic) so that
        // completions cover the full CLI surface including API commands.
        let mut app = Cli::command();
        if let Ok((cmds, _)) = load_generated_commands() {
            let dynamic_clap = clap_tree::build_dynamic_commands(&cmds);
            for sub in dynamic_clap {
                if !STATIC_COMMANDS.contains(&sub.get_name()) {
                    app = app.subcommand(sub);
                }
            }
        }
        clap_complete::generate(shell, &mut app, "dcx", &mut std::io::stdout());
        return;
    }

    // profiles commands don't need project/dataset config
    if let Command::Profiles { ref command } = cli.command {
        let result = match command {
            ProfilesCommand::List => commands::profiles::list::run(&cli.format),
            ProfilesCommand::Show { profile } => {
                commands::profiles::show::run(profile, &cli.format)
            }
            ProfilesCommand::Validate { profile } => {
                commands::profiles::validate::run(profile, &cli.format)
            }
        };
        if let Err(e) = result {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
        return;
    }

    // looker commands use profile-based auth, no project/dataset needed
    if let Command::Looker { ref command } = cli.command {
        let auth_opts = auth::AuthOptions {
            token: cli.token.clone(),
            credentials_file: cli.credentials_file.clone(),
        };
        let result = match command {
            LookerCommand::Explores { command } => match command {
                LookerExploresCommand::List { profile } => {
                    commands::looker::explores::run_list(profile, &auth_opts, &cli.format).await
                }
                LookerExploresCommand::Get { profile, explore } => {
                    commands::looker::explores::run_get(profile, explore, &auth_opts, &cli.format)
                        .await
                }
            },
            LookerCommand::Dashboards { command } => match command {
                LookerDashboardsCommand::List { profile } => {
                    commands::looker::dashboards::run_list(profile, &auth_opts, &cli.format).await
                }
                LookerDashboardsCommand::Get {
                    profile,
                    dashboard_id,
                } => {
                    commands::looker::dashboards::run_get(
                        profile,
                        dashboard_id,
                        &auth_opts,
                        &cli.format,
                    )
                    .await
                }
            },
        };
        if let Err(e) = result {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
        return;
    }

    // ca ask --profile bypasses Config::from_cli() because the profile
    // supplies its own project/location — no --project-id required.
    if let Command::Ca {
        command:
            CaCommand::Ask {
                ref question,
                profile: Some(ref profile_ref),
                ref agent,
                ref tables,
            },
    } = cli.command
    {
        if agent.is_some() || tables.is_some() {
            eprintln!(
                "{}",
                json!({"error": "--profile cannot be combined with --agent or --tables"})
            );
            std::process::exit(1);
        }
        let auth_opts = auth::AuthOptions {
            token: cli.token.clone(),
            credentials_file: cli.credentials_file.clone(),
        };
        let result = commands::ca::ask::run_profile(
            question.clone(),
            profile_ref,
            &auth_opts,
            &cli.format,
            cli.sanitize.as_deref(),
        )
        .await;
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
        Command::Ca { command } => match command {
            CaCommand::Ask {
                question,
                profile: _, // already handled above
                agent,
                tables,
            } => commands::ca::ask::run(question, agent, tables, &auth_opts, &config).await,
            CaCommand::CreateAgent {
                name,
                tables,
                views,
                verified_queries,
                instructions,
            } => {
                commands::ca::create_agent::run(
                    name,
                    tables,
                    views,
                    verified_queries,
                    instructions,
                    &auth_opts,
                    &config,
                )
                .await
            }
            CaCommand::ListAgents => commands::ca::list_agents::run(&auth_opts, &config).await,
            CaCommand::AddVerifiedQuery {
                agent,
                question,
                query,
            } => {
                commands::ca::add_verified_query::run(agent, question, query, &auth_opts, &config)
                    .await
            }
        },
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
            AnalyticsCommand::ListTraces {
                last,
                agent_id,
                limit,
            } => {
                commands::analytics::list_traces::run(last, agent_id, limit, &auth_opts, &config)
                    .await
            }
            AnalyticsCommand::Insights { last, agent_id } => {
                commands::analytics::insights::run(last, agent_id, &auth_opts, &config).await
            }
            AnalyticsCommand::Drift {
                golden_dataset,
                last,
                agent_id,
                min_coverage,
                exit_code,
            } => {
                commands::analytics::drift::run(
                    golden_dataset,
                    last,
                    agent_id,
                    min_coverage,
                    exit_code,
                    &auth_opts,
                    &config,
                )
                .await
            }
            AnalyticsCommand::Distribution { last, agent_id } => {
                commands::analytics::distribution::run(last, agent_id, &auth_opts, &config).await
            }
            AnalyticsCommand::HitlMetrics {
                last,
                agent_id,
                limit,
            } => {
                commands::analytics::hitl_metrics::run(last, agent_id, limit, &auth_opts, &config)
                    .await
            }
            AnalyticsCommand::Views { command } => match command {
                ViewsCommand::CreateAll { prefix } => {
                    commands::analytics::views::run(prefix, &auth_opts, &config).await
                }
            },
        },
        Command::Auth { .. }
        | Command::GenerateSkills { .. }
        | Command::Completions { .. }
        | Command::Profiles { .. }
        | Command::Looker { .. } => {
            unreachable!()
        }
    };

    if let Err(e) = result {
        if let Some(BqxError::EvalFailed { exit_code }) = e.downcast_ref::<BqxError>() {
            std::process::exit(*exit_code);
        }
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
}
