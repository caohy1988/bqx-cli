use clap::{CommandFactory, FromArgMatches};
use serde_json::json;

use dcx::auth;
use dcx::bigquery::discovery::{self, DiscoverySource};
use dcx::bigquery::dynamic::{clap_tree, executor, model, service};
use dcx::cli::{
    AnalyticsCommand, AuthCommand, CaCommand, Cli, Command, JobsCommand, OutputFormat,
    ProfilesCommand, ShellType, ViewsCommand,
};
use dcx::commands;
use dcx::config::{self, Config};
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
];

/// A loaded service with its generated commands and base URL.
struct LoadedService {
    config: service::ServiceConfig,
    commands: Vec<model::GeneratedCommand>,
    base_url: String,
}

#[tokio::main]
async fn main() {
    // 1. Load all services (BigQuery, Spanner, AlloyDB, Cloud SQL, Looker).
    let services = load_all_services();

    // 2. Build a hybrid clap::Command: static derive tree + dynamic subcommands.
    let mut app = Cli::command();
    let mut namespace_names: Vec<String> = Vec::new();

    for svc in &services {
        let global_params = svc.config.global_param_names();
        let dynamic_clap = clap_tree::build_dynamic_commands(
            &svc.commands,
            &global_params,
            svc.config.service_label,
        );

        if svc.config.namespace.is_empty() {
            // Top-level dynamic commands (BigQuery).
            for sub in dynamic_clap {
                if !STATIC_COMMANDS.contains(&sub.get_name()) {
                    app = app.subcommand(sub);
                }
            }
        } else {
            // Namespaced service (Spanner, AlloyDB, Cloud SQL).
            let ns_cmd = clap::Command::new(svc.config.namespace)
                .about(format!(
                    "{} operations (generated from API)",
                    svc.config.service_label
                ))
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommands(dynamic_clap);
            let ns_cmd =
                commands::database_helpers::augment_namespace_command(svc.config.namespace, ns_cmd);
            namespace_names.push(svc.config.namespace.to_string());
            app = app.subcommand(ns_cmd);
        }
    }

    // 3. Parse args with the augmented command.
    let matches = app.get_matches();

    // 4. Check if the matched subcommand is dynamic (top-level or namespaced).
    if let Some((sub_name, sub_matches)) = matches.subcommand() {
        if !STATIC_COMMANDS.contains(&sub_name) {
            // Find the service for this command.
            if let Some(svc) = services
                .iter()
                .find(|s| !s.config.namespace.is_empty() && s.config.namespace == sub_name)
            {
                // Namespaced service: drill into group → action.
                let (group_name, group_matches) = match sub_matches.subcommand() {
                    Some(pair) => pair,
                    None => {
                        eprintln!("{}", json!({"error": "No subcommand provided"}));
                        std::process::exit(1);
                    }
                };
                let (action_name, action_matches) = match group_matches.subcommand() {
                    Some(pair) => pair,
                    None => {
                        eprintln!("{}", json!({"error": "No subcommand provided"}));
                        std::process::exit(1);
                    }
                };
                if let Some(result) = commands::database_helpers::try_run_namespace_helper(
                    svc.config.namespace,
                    group_name,
                    action_name,
                    action_matches,
                    &matches,
                )
                .await
                {
                    if let Err(e) = result {
                        eprintln!("{}", json!({"error": e.to_string()}));
                        std::process::exit(1);
                    }
                    return;
                }
                run_dynamic(svc, group_name, action_name, action_matches, &matches).await;
                return;
            } else {
                // Top-level dynamic (BigQuery).
                let bq_svc = services
                    .iter()
                    .find(|s| s.config.namespace.is_empty())
                    .expect("BigQuery service not loaded");
                let (action_name, action_matches) = match sub_matches.subcommand() {
                    Some(pair) => pair,
                    None => {
                        eprintln!("{}", json!({"error": "No subcommand provided"}));
                        std::process::exit(1);
                    }
                };
                run_dynamic(bq_svc, sub_name, action_name, action_matches, &matches).await;
                return;
            }
        }
    }

    // 5. Static path: reconstruct Cli from the already-parsed matches.
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", json!({"error": e.to_string()}));
            std::process::exit(1);
        }
    };

    run_static(cli, &services).await;
}

/// Load all service discovery docs and build generated commands.
fn load_all_services() -> Vec<LoadedService> {
    let mut services = Vec::new();
    for config in service::all_services() {
        match load_service(&config) {
            Ok(svc) => services.push(svc),
            Err(e) => {
                eprintln!(
                    "Warning: could not load {} commands: {e}",
                    config.service_label
                );
                // Still push an empty service so namespace registration works.
                services.push(LoadedService {
                    config,
                    commands: Vec::new(),
                    base_url: String::new(),
                });
            }
        }
    }
    services
}

fn load_service(config: &service::ServiceConfig) -> anyhow::Result<LoadedService> {
    let doc = if config.discovery_name == "bigquery" {
        // BigQuery uses the existing bundled loading path for cache/remote support.
        discovery::load(&DiscoverySource::Bundled)?
    } else {
        config.load_bundled()?
    };
    let base_url = doc.base_url.clone();
    let methods = model::extract_methods(&doc, config.use_flat_path);
    let allowed = model::filter_allowed(&methods, config.allowed_methods);
    let commands: Vec<model::GeneratedCommand> =
        allowed.iter().map(model::to_generated_command).collect();
    // We need to clone the config since all_services() returns owned values.
    // Recreate the config for storage.
    Ok(LoadedService {
        config: recreate_config(config),
        commands,
        base_url,
    })
}

/// Recreate a ServiceConfig from an existing one (needed because we consume the iterator).
fn recreate_config(c: &service::ServiceConfig) -> service::ServiceConfig {
    service::ServiceConfig {
        namespace: c.namespace,
        service_label: c.service_label,
        discovery_name: c.discovery_name,
        bundled_json: c.bundled_json,
        allowed_methods: c.allowed_methods,
        global_params: c.global_params,
        use_flat_path: c.use_flat_path,
    }
}

async fn run_dynamic(
    svc: &LoadedService,
    group_name: &str,
    action_name: &str,
    action_matches: &clap::ArgMatches,
    root_matches: &clap::ArgMatches,
) {
    let cmd = match clap_tree::find_command(&svc.commands, group_name, action_name) {
        Some(c) => c,
        None => {
            eprintln!(
                "{}",
                json!({"error": format!("Unknown command: {group_name} {action_name}")})
            );
            std::process::exit(1);
        }
    };

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

    let global_params = svc.config.global_param_names();
    let mut args = clap_tree::extract_args(action_matches, cmd, &global_params);

    // Inject global flag values for params mapped in the service config.
    for (api_name, cli_flag) in svc.config.global_params {
        if *cli_flag == "project_id" {
            // Already handled via the dedicated project_id path in executor.
            continue;
        }
        if let Some(value) = root_matches.get_one::<String>(cli_flag) {
            let mut effective = value.clone();
            // AlloyDB and Looker use region-granularity locations; the global
            // --location default "US" is a BigQuery convention — normalize to
            // "-" (all locations) so the contract is preserved.
            if *cli_flag == "location"
                && (svc.config.namespace == "alloydb" || svc.config.namespace == "looker")
                && effective == "US"
            {
                effective = "-".to_string();
            }
            args.entry(api_name.to_string())
                .or_insert_with(|| effective);
        }
    }

    // BigQuery-specific: inject datasetId from global flag if needed.
    if svc.config.namespace.is_empty() {
        let needs_dataset_id = cmd.method.parameters.iter().any(|p| {
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
    }

    // Validate identifiers in all path parameters before any network call.
    // The wildcard "-" is allowed (e.g., AlloyDB location = all regions).
    if let Err(e) = config::validate_identifier(&project_id, "project-id") {
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
    for gen_arg in &cmd.args {
        let is_path = cmd
            .method
            .parameters
            .iter()
            .any(|p| p.name == gen_arg.api_name && p.location == model::ParamLocation::Path);
        if !is_path {
            continue;
        }
        let global_params = svc.config.global_param_names();
        if global_params.contains(&gen_arg.api_name.as_str()) {
            continue; // project_id already validated above; location may be "-"
        }
        if let Some(value) = args.get(&gen_arg.api_name) {
            if value != "-" {
                if let Err(e) = config::validate_identifier(value, &gen_arg.flag_name) {
                    eprintln!("{}", json!({"error": e.to_string()}));
                    std::process::exit(1);
                }
            }
        }
    }

    let sanitize_template = root_matches
        .get_one::<String>("sanitize")
        .map(|s| s.as_str());

    let result = executor::execute(
        cmd,
        &args,
        &project_id,
        &svc.base_url,
        &format,
        dry_run,
        &auth_opts,
        sanitize_template,
        &svc.config,
    )
    .await;

    if let Err(e) = result {
        eprintln!("{}", json!({"error": e.to_string()}));
        std::process::exit(1);
    }
}

async fn run_static(cli: Cli, services: &[LoadedService]) {
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
        for svc in services {
            let global_params = svc.config.global_param_names();
            let dynamic_clap = clap_tree::build_dynamic_commands(
                &svc.commands,
                &global_params,
                svc.config.service_label,
            );
            if svc.config.namespace.is_empty() {
                for sub in dynamic_clap {
                    if !STATIC_COMMANDS.contains(&sub.get_name()) {
                        app = app.subcommand(sub);
                    }
                }
            } else {
                let ns_cmd = clap::Command::new(svc.config.namespace)
                    .about(format!(
                        "{} operations (generated from API)",
                        svc.config.service_label
                    ))
                    .subcommand_required(true)
                    .arg_required_else_help(true)
                    .subcommands(dynamic_clap);
                app = app.subcommand(ns_cmd);
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
                ViewsCommand::Create { event_type, prefix } => {
                    commands::analytics::views::run_create(
                        event_type, prefix, &auth_opts, &config,
                    )
                    .await
                }
            },
            AnalyticsCommand::CategoricalEval {
                metrics_file,
                agent_id,
                last,
                limit,
                endpoint,
                include_justification,
                persist,
                results_table,
                prompt_version,
            } => {
                commands::analytics::categorical_eval::run(
                    metrics_file,
                    agent_id,
                    last,
                    limit,
                    endpoint,
                    include_justification,
                    persist,
                    results_table,
                    prompt_version,
                    &auth_opts,
                    &config,
                )
                .await
            }
            AnalyticsCommand::CategoricalViews {
                results_table,
                prefix,
            } => {
                commands::analytics::categorical_views::run(
                    results_table, prefix, &auth_opts, &config,
                )
                .await
            }
        },
        Command::Auth { .. }
        | Command::GenerateSkills { .. }
        | Command::Completions { .. }
        | Command::Profiles { .. } => {
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
