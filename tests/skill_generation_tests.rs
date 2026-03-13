use bqx::bigquery::discovery::{self, DiscoverySource};
use bqx::bigquery::dynamic::model::{extract_methods, filter_allowed, to_generated_command};
use bqx::skills::generator;

fn load_generated_commands() -> Vec<bqx::bigquery::dynamic::model::GeneratedCommand> {
    let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
    let methods = extract_methods(&doc);
    let allowed = filter_allowed(&methods);
    allowed.iter().map(to_generated_command).collect()
}

// ---------------------------------------------------------------------------
// Snapshot tests for generated skills
// ---------------------------------------------------------------------------

#[test]
fn snapshot_datasets_skill_md() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);
    let datasets_skill = skills
        .iter()
        .find(|s| s.dir_name == "bqx-datasets")
        .unwrap();
    insta::assert_snapshot!("generated_datasets_skill_md", &datasets_skill.skill_md);
}

#[test]
fn snapshot_tables_skill_md() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);
    let tables_skill = skills.iter().find(|s| s.dir_name == "bqx-tables").unwrap();
    insta::assert_snapshot!("generated_tables_skill_md", &tables_skill.skill_md);
}

#[test]
fn snapshot_datasets_openai_yaml() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);
    let datasets_skill = skills
        .iter()
        .find(|s| s.dir_name == "bqx-datasets")
        .unwrap();
    insta::assert_snapshot!(
        "generated_datasets_openai_yaml",
        &datasets_skill.openai_yaml
    );
}

#[test]
fn snapshot_tables_openai_yaml() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);
    let tables_skill = skills.iter().find(|s| s.dir_name == "bqx-tables").unwrap();
    insta::assert_snapshot!("generated_tables_openai_yaml", &tables_skill.openai_yaml);
}

// ---------------------------------------------------------------------------
// End-to-end: generate and write to disk
// ---------------------------------------------------------------------------

#[test]
fn generate_skills_writes_all_expected_files() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);

    let tmp = tempfile::tempdir().unwrap();
    let written = generator::write_skills(tmp.path(), &skills).unwrap();

    // Verify all expected skill dirs were written.
    assert!(written.contains(&"bqx-datasets".to_string()));
    assert!(written.contains(&"bqx-tables".to_string()));

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
    let skills = generator::generate_all(&commands);
    let filtered = generator::filter_skills(skills, &["bqx-tables".to_string()]);

    let tmp = tempfile::tempdir().unwrap();
    let written = generator::write_skills(tmp.path(), &filtered).unwrap();

    assert_eq!(written, vec!["bqx-tables"]);
    assert!(tmp.path().join("bqx-tables/SKILL.md").exists());
    assert!(!tmp.path().join("bqx-datasets").exists());
}

#[test]
fn generated_skill_md_references_bqx_shared() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);

    for skill in &skills {
        assert!(
            skill.skill_md.contains("bqx-shared"),
            "{}: should reference bqx-shared for auth guidance",
            skill.dir_name
        );
    }
}

#[test]
fn generated_skill_md_contains_command_examples() {
    let commands = load_generated_commands();
    let skills = generator::generate_all(&commands);

    let datasets_skill = skills
        .iter()
        .find(|s| s.dir_name == "bqx-datasets")
        .unwrap();
    assert!(datasets_skill.skill_md.contains("bqx datasets list"));
    assert!(datasets_skill.skill_md.contains("bqx datasets get"));

    let tables_skill = skills.iter().find(|s| s.dir_name == "bqx-tables").unwrap();
    assert!(tables_skill.skill_md.contains("bqx tables list"));
    assert!(tables_skill.skill_md.contains("bqx tables get"));
}
