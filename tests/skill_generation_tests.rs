use clap::CommandFactory;
use dcx::bigquery::discovery::{self, DiscoverySource};
use dcx::bigquery::dynamic::clap_tree;
use dcx::bigquery::dynamic::model::{extract_methods, filter_allowed, to_generated_command};
use dcx::bigquery::dynamic::service;
use dcx::commands::meta;
use dcx::skills::generator;

fn load_generated_commands() -> Vec<dcx::bigquery::dynamic::model::GeneratedCommand> {
    let cfg = service::bigquery();
    let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
    let methods = extract_methods(&doc, cfg.use_flat_path);
    let allowed = filter_allowed(&methods, cfg.allowed_methods);
    allowed.iter().map(to_generated_command).collect()
}

fn load_contracts() -> Vec<meta::CommandContract> {
    let cfg = service::bigquery();
    let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
    let methods = extract_methods(&doc, cfg.use_flat_path);
    let allowed = filter_allowed(&methods, cfg.allowed_methods);
    let commands: Vec<_> = allowed.iter().map(to_generated_command).collect();

    let global_params = cfg.global_param_names();
    let dynamic_clap =
        clap_tree::build_dynamic_commands(&commands, &global_params, cfg.service_label);

    let mut app = dcx::cli::Cli::command();
    for sub in dynamic_clap {
        app = app.subcommand(sub);
    }
    meta::collect_all(&app)
}

// ---------------------------------------------------------------------------
// Snapshot tests for generated skills
// ---------------------------------------------------------------------------

#[test]
fn snapshot_datasets_skill_md() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let datasets_skill = skills
        .iter()
        .find(|s| s.dir_name == "dcx-datasets")
        .unwrap();
    insta::assert_snapshot!("generated_datasets_skill_md", &datasets_skill.skill_md);
}

#[test]
fn snapshot_tables_skill_md() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let tables_skill = skills.iter().find(|s| s.dir_name == "dcx-tables").unwrap();
    insta::assert_snapshot!("generated_tables_skill_md", &tables_skill.skill_md);
}

#[test]
fn snapshot_datasets_openai_yaml() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let datasets_skill = skills
        .iter()
        .find(|s| s.dir_name == "dcx-datasets")
        .unwrap();
    insta::assert_snapshot!(
        "generated_datasets_openai_yaml",
        &datasets_skill.openai_yaml
    );
}

#[test]
fn snapshot_tables_openai_yaml() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let tables_skill = skills.iter().find(|s| s.dir_name == "dcx-tables").unwrap();
    insta::assert_snapshot!("generated_tables_openai_yaml", &tables_skill.openai_yaml);
}

// ---------------------------------------------------------------------------
// End-to-end: generate and write to disk
// ---------------------------------------------------------------------------

#[test]
fn generate_skills_writes_all_expected_files() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);

    let tmp = tempfile::tempdir().unwrap();
    let written = generator::write_skills(tmp.path(), &skills).unwrap();

    // Verify all expected skill dirs were written.
    assert!(written.contains(&"dcx-datasets".to_string()));
    assert!(written.contains(&"dcx-tables".to_string()));
    assert!(written.contains(&"dcx-routines".to_string()));
    assert!(written.contains(&"dcx-models".to_string()));

    // Verify file structure.
    for name in &written {
        let skill_md = tmp.path().join(name).join("SKILL.md");
        let yaml = tmp.path().join(name).join("agents/openai.yaml");
        assert!(skill_md.exists(), "Missing SKILL.md for {name}");
        assert!(yaml.exists(), "Missing agents/openai.yaml for {name}");

        // Verify SKILL.md starts with frontmatter.
        let content = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            content.starts_with("---\n"),
            "{name}/SKILL.md missing frontmatter"
        );

        // Verify openai.yaml has required fields.
        let yaml_content = std::fs::read_to_string(&yaml).unwrap();
        assert!(
            yaml_content.contains("display_name:"),
            "{name}/openai.yaml missing display_name"
        );
    }
}

#[test]
fn generate_skills_filter_limits_output() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let filtered = generator::filter_skills(skills, &["dcx-tables".to_string()]);

    let tmp = tempfile::tempdir().unwrap();
    let written = generator::write_skills(tmp.path(), &filtered).unwrap();

    assert_eq!(written, vec!["dcx-tables"]);
    assert!(tmp.path().join("dcx-tables/SKILL.md").exists());
    assert!(!tmp.path().join("dcx-datasets").exists());
}

#[test]
fn generated_skill_md_references_dcx_shared() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);

    for skill in &skills {
        assert!(
            skill.skill_md.contains("dcx-shared"),
            "{}: should reference dcx-shared for auth guidance",
            skill.dir_name
        );
    }
}

#[test]
fn generated_skill_md_contains_command_examples() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);

    let datasets_skill = skills
        .iter()
        .find(|s| s.dir_name == "dcx-datasets")
        .unwrap();
    assert!(datasets_skill.skill_md.contains("dcx datasets list"));
    assert!(datasets_skill.skill_md.contains("dcx datasets get"));

    let tables_skill = skills.iter().find(|s| s.dir_name == "dcx-tables").unwrap();
    assert!(tables_skill.skill_md.contains("dcx tables list"));
    assert!(tables_skill.skill_md.contains("dcx tables get"));

    let routines_skill = skills
        .iter()
        .find(|s| s.dir_name == "dcx-routines")
        .unwrap();
    assert!(routines_skill.skill_md.contains("dcx routines list"));
    assert!(routines_skill.skill_md.contains("dcx routines get"));

    let models_skill = skills.iter().find(|s| s.dir_name == "dcx-models").unwrap();
    assert!(models_skill.skill_md.contains("dcx models list"));
    assert!(models_skill.skill_md.contains("dcx models get"));
}

#[test]
fn snapshot_routines_skill_md() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let routines_skill = skills
        .iter()
        .find(|s| s.dir_name == "dcx-routines")
        .unwrap();
    insta::assert_snapshot!("generated_routines_skill_md", &routines_skill.skill_md);
}

#[test]
fn snapshot_routines_openai_yaml() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let routines_skill = skills
        .iter()
        .find(|s| s.dir_name == "dcx-routines")
        .unwrap();
    insta::assert_snapshot!(
        "generated_routines_openai_yaml",
        &routines_skill.openai_yaml
    );
}

#[test]
fn snapshot_models_skill_md() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let models_skill = skills.iter().find(|s| s.dir_name == "dcx-models").unwrap();
    insta::assert_snapshot!("generated_models_skill_md", &models_skill.skill_md);
}

#[test]
fn snapshot_models_openai_yaml() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    let models_skill = skills.iter().find(|s| s.dir_name == "dcx-models").unwrap();
    insta::assert_snapshot!("generated_models_openai_yaml", &models_skill.openai_yaml);
}

// ---------------------------------------------------------------------------
// Contract-driven validation
// ---------------------------------------------------------------------------

/// Verify all generated skills pass agentskills.io constraints.
#[test]
fn all_skills_pass_agentskills_validation() {
    use dcx::skills::templates::validate_skill;
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);
    for skill in &skills {
        let validation = validate_skill(skill);
        assert!(
            validation.errors.is_empty(),
            "{}: agentskills.io violations: {:?}",
            validation.skill_name,
            validation.errors
        );
    }
}

/// Verify skill flag tables match the command contract (drift detection).
#[test]
fn skill_flag_tables_match_contracts() {
    let commands = load_generated_commands();
    let contracts = load_contracts();
    let skills = generator::generate_all(&commands, &contracts);

    for skill in &skills {
        // Extract flag names from the SKILL.md flags table.
        let skill_flags: Vec<String> = skill
            .skill_md
            .lines()
            .filter(|l| l.starts_with("| `--") && !l.contains("global flag"))
            .filter_map(|l| l.split('`').nth(1).map(|s| s.to_string()))
            .collect();

        // For each flag in the skill, verify it exists in the contract.
        for flag_name in &skill_flags {
            let cmd_path = find_command_for_flag(flag_name, &skill.skill_md);
            if let Some(path) = cmd_path {
                let contract = contracts.iter().find(|c| c.command == path);
                assert!(
                    contract.is_some(),
                    "{}: flag {} references command '{}' not in contracts",
                    skill.dir_name,
                    flag_name,
                    path
                );
                if let Some(c) = contract {
                    let in_contract = c.flags.iter().any(|f| f.name == *flag_name)
                        || c.global_flags.iter().any(|f| f.name == *flag_name);
                    assert!(
                        in_contract,
                        "{}: flag {} not found in contract for '{}'",
                        skill.dir_name, flag_name, path
                    );
                }
            }
        }
    }
}

/// Helper: find the command path for a flag by scanning the SKILL.md for the
/// preceding `### group action` heading.
fn find_command_for_flag(flag: &str, skill_md: &str) -> Option<String> {
    let mut current_cmd = None;
    for line in skill_md.lines() {
        if line.starts_with("### ") {
            let parts: Vec<&str> = line.trim_start_matches("### ").split_whitespace().collect();
            if parts.len() == 2 {
                current_cmd = Some(format!("dcx {} {}", parts[0], parts[1]));
            }
        }
        if line.contains(flag) && line.starts_with("| `") {
            return current_cmd.clone();
        }
    }
    None
}
