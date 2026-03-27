use std::collections::HashMap;

use clap::{Arg, ArgAction, ArgMatches};

use super::model::{ArgValueType, GeneratedCommand, ParamLocation};

/// Build a list of `clap::Command` groups from generated commands.
/// Groups commands by resource (e.g. "datasets", "tables") and adds action
/// subcommands (e.g. "list", "get") under each group.
///
/// `global_param_names`: API parameter names provided by global CLI flags
///   (these are skipped in generated per-command args).
/// `service_label`: human label for help text (e.g. "BigQuery", "Cloud Spanner").
pub fn build_dynamic_commands(
    commands: &[GeneratedCommand],
    global_param_names: &[&str],
    service_label: &str,
) -> Vec<clap::Command> {
    let mut groups: HashMap<&str, Vec<&GeneratedCommand>> = HashMap::new();
    for cmd in commands {
        groups.entry(cmd.group.as_str()).or_default().push(cmd);
    }

    let mut result: Vec<clap::Command> = Vec::new();
    let mut group_names: Vec<&&str> = groups.keys().collect();
    group_names.sort();

    for group_name in group_names {
        let group_cmds = &groups[group_name];
        let mut subcommands: Vec<clap::Command> = Vec::new();

        let mut sorted_cmds: Vec<&&GeneratedCommand> = group_cmds.iter().collect();
        sorted_cmds.sort_by_key(|c| &c.action);

        for gen_cmd in sorted_cmds {
            let mut action_cmd =
                clap::Command::new(gen_cmd.action.clone()).about(gen_cmd.about.clone());

            for arg in &gen_cmd.args {
                // Skip params already handled by global CLI flags.
                if global_param_names.contains(&arg.api_name.as_str()) {
                    continue;
                }

                let mut clap_arg = Arg::new(&arg.flag_name)
                    .long(&arg.flag_name)
                    .help(&arg.help);

                if arg.required {
                    clap_arg = clap_arg.required(true);
                }

                match arg.value_type {
                    ArgValueType::Boolean => {
                        clap_arg = clap_arg
                            .action(ArgAction::SetTrue)
                            .required(false)
                            .num_args(0);
                    }
                    ArgValueType::Integer => {
                        clap_arg = clap_arg.value_parser(clap::value_parser!(i64));
                    }
                    ArgValueType::String => {}
                }

                action_cmd = action_cmd.arg(clap_arg);
            }

            // Add --dry-run flag to every dynamic action command.
            action_cmd = action_cmd.arg(
                Arg::new("dry-run")
                    .long("dry-run")
                    .help("Show the request that would be sent without executing it")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .num_args(0),
            );

            subcommands.push(action_cmd);
        }

        let group_about = format!("{service_label} {group_name} operations (generated from API)");
        let group_cmd = clap::Command::new(group_name.to_string())
            .about(group_about)
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommands(subcommands);

        result.push(group_cmd);
    }

    result
}

/// Extract matched argument values from clap ArgMatches into a map.
/// Keys are the original API parameter names (camelCase).
pub fn extract_args(
    matches: &ArgMatches,
    cmd: &GeneratedCommand,
    global_param_names: &[&str],
) -> HashMap<String, String> {
    let mut args = HashMap::new();
    for arg in &cmd.args {
        if global_param_names.contains(&arg.api_name.as_str()) {
            continue;
        }
        match arg.value_type {
            ArgValueType::Boolean => {
                if matches.get_flag(&arg.flag_name) {
                    args.insert(arg.api_name.clone(), "true".to_string());
                }
            }
            ArgValueType::Integer => {
                if let Some(val) = matches.get_one::<i64>(&arg.flag_name) {
                    args.insert(arg.api_name.clone(), val.to_string());
                }
            }
            ArgValueType::String => {
                if let Some(val) = matches.get_one::<String>(&arg.flag_name) {
                    args.insert(arg.api_name.clone(), val.clone());
                }
            }
        }
    }
    args
}

/// Find the GeneratedCommand matching a (group, action) pair.
pub fn find_command<'a>(
    commands: &'a [GeneratedCommand],
    group: &str,
    action: &str,
) -> Option<&'a GeneratedCommand> {
    commands
        .iter()
        .find(|c| c.group == group && c.action == action)
}

/// Validate that all required path parameters are present.
pub fn validate_required_params(
    args: &HashMap<String, String>,
    cmd: &GeneratedCommand,
    global_param_names: &[&str],
) -> Result<(), String> {
    for gen_arg in &cmd.args {
        if global_param_names.contains(&gen_arg.api_name.as_str()) {
            continue;
        }
        if gen_arg.required {
            let has_path_param = cmd
                .method
                .parameters
                .iter()
                .any(|p| p.name == gen_arg.api_name && p.location == ParamLocation::Path);
            if has_path_param && !args.contains_key(&gen_arg.api_name) {
                return Err(format!(
                    "Missing required argument: --{}",
                    gen_arg.flag_name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bigquery::discovery::{self, DiscoverySource};
    use crate::bigquery::dynamic::model::{extract_methods, filter_allowed, to_generated_command};
    use crate::bigquery::dynamic::service;

    fn load_bq_commands() -> Vec<GeneratedCommand> {
        let cfg = service::bigquery();
        let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        allowed.iter().map(to_generated_command).collect()
    }

    fn bq_global_params() -> Vec<&'static str> {
        service::bigquery().global_param_names()
    }

    #[test]
    fn builds_expected_groups() {
        let cmds = load_bq_commands();
        let dynamic = build_dynamic_commands(&cmds, &bq_global_params(), "BigQuery");
        let names: Vec<&str> = dynamic.iter().map(|c| c.get_name()).collect();
        assert_eq!(names, vec!["datasets", "models", "routines", "tables"]);
    }

    #[test]
    fn datasets_has_list_and_get() {
        let cmds = load_bq_commands();
        let dynamic = build_dynamic_commands(&cmds, &bq_global_params(), "BigQuery");
        let datasets = dynamic.iter().find(|c| c.get_name() == "datasets").unwrap();
        let sub_names: Vec<&str> = datasets.get_subcommands().map(|s| s.get_name()).collect();
        assert!(sub_names.contains(&"list"));
        assert!(sub_names.contains(&"get"));
    }

    #[test]
    fn datasets_list_skips_project_id_arg() {
        let cmds = load_bq_commands();
        let dynamic = build_dynamic_commands(&cmds, &bq_global_params(), "BigQuery");
        let datasets = dynamic.iter().find(|c| c.get_name() == "datasets").unwrap();
        let list = datasets
            .get_subcommands()
            .find(|s| s.get_name() == "list")
            .unwrap();
        let arg_names: Vec<&str> = list.get_arguments().map(|a| a.get_id().as_str()).collect();
        assert!(
            !arg_names.contains(&"project-id"),
            "project-id should be handled by global flag, not generated arg"
        );
    }

    #[test]
    fn find_command_works() {
        let cmds = load_bq_commands();
        let found = find_command(&cmds, "datasets", "list");
        assert!(found.is_some());
        assert_eq!(found.unwrap().method.id, "bigquery.datasets.list");

        let not_found = find_command(&cmds, "nonexistent", "list");
        assert!(not_found.is_none());
    }

    #[test]
    fn validate_required_catches_missing_table_id() {
        let cmds = load_bq_commands();
        let tbl_get = find_command(&cmds, "tables", "get").unwrap();
        let mut args = HashMap::new();
        args.insert("datasetId".to_string(), "my_dataset".to_string());
        let result = validate_required_params(&args, tbl_get, &bq_global_params());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--table-id"));
    }

    #[test]
    fn validate_required_passes_when_present() {
        let cmds = load_bq_commands();
        let tbl_get = find_command(&cmds, "tables", "get").unwrap();
        let mut args = HashMap::new();
        args.insert("datasetId".to_string(), "my_dataset".to_string());
        args.insert("tableId".to_string(), "my_table".to_string());
        let result = validate_required_params(&args, tbl_get, &bq_global_params());
        assert!(result.is_ok());
    }

    #[test]
    fn spanner_commands_build_correctly() {
        let cfg = service::spanner();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        let cmds: Vec<GeneratedCommand> = allowed.iter().map(to_generated_command).collect();
        let global = cfg.global_param_names();
        let dynamic = build_dynamic_commands(&cmds, &global, cfg.service_label);
        let names: Vec<&str> = dynamic.iter().map(|c| c.get_name()).collect();
        assert!(names.contains(&"instances"), "names: {names:?}");
        assert!(names.contains(&"databases"), "names: {names:?}");
    }

    #[test]
    fn cloudsql_commands_build_correctly() {
        let cfg = service::cloudsql();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        let cmds: Vec<GeneratedCommand> = allowed.iter().map(to_generated_command).collect();
        let global = cfg.global_param_names();
        let dynamic = build_dynamic_commands(&cmds, &global, cfg.service_label);
        let names: Vec<&str> = dynamic.iter().map(|c| c.get_name()).collect();
        assert!(names.contains(&"instances"), "names: {names:?}");
        assert!(names.contains(&"databases"), "names: {names:?}");
    }
}
