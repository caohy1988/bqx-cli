use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::bigquery::dynamic::model::GeneratedCommand;

use super::templates::{self, SkillOutput};

/// Generate skill files for all resource groups from the generated commands.
///
/// Groups commands by resource (e.g. "datasets", "tables") and generates
/// one skill directory per group containing SKILL.md and agents/openai.yaml.
pub fn generate_all(commands: &[GeneratedCommand]) -> Vec<SkillOutput> {
    let mut groups: HashMap<&str, Vec<&GeneratedCommand>> = HashMap::new();
    for cmd in commands {
        groups.entry(cmd.group.as_str()).or_default().push(cmd);
    }

    let mut group_names: Vec<&&str> = groups.keys().collect();
    group_names.sort();

    let mut skills = Vec::new();
    for group_name in group_names {
        let group_cmds = &groups[group_name];
        let mut sorted_cmds = group_cmds.clone();
        sorted_cmds.sort_by_key(|c| &c.action);
        let skill = templates::generate_resource_skill(group_name, &sorted_cmds);
        skills.push(skill);
    }

    skills
}

/// Filter generated skills by a list of skill directory names.
/// If the filter is empty, all skills are returned.
pub fn filter_skills(skills: Vec<SkillOutput>, filter: &[String]) -> Vec<SkillOutput> {
    if filter.is_empty() {
        return skills;
    }
    skills
        .into_iter()
        .filter(|s| filter.iter().any(|f| f == &s.dir_name))
        .collect()
}

/// Write generated skills to the output directory.
///
/// Creates `<output_dir>/<skill_dir>/SKILL.md` and
/// `<output_dir>/<skill_dir>/agents/openai.yaml` for each skill.
pub fn write_skills(output_dir: &Path, skills: &[SkillOutput]) -> Result<Vec<String>> {
    let mut written = Vec::new();

    for skill in skills {
        let skill_dir = output_dir.join(&skill.dir_name);
        let agents_dir = skill_dir.join("agents");

        std::fs::create_dir_all(&agents_dir)
            .with_context(|| format!("Failed to create directory: {}", agents_dir.display()))?;

        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, &skill.skill_md)
            .with_context(|| format!("Failed to write: {}", skill_path.display()))?;

        let yaml_path = agents_dir.join("openai.yaml");
        std::fs::write(&yaml_path, &skill.openai_yaml)
            .with_context(|| format!("Failed to write: {}", yaml_path.display()))?;

        written.push(skill.dir_name.clone());
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bigquery::discovery::{self, DiscoverySource};
    use crate::bigquery::dynamic::model::{extract_methods, filter_allowed, to_generated_command};

    fn load_generated_commands() -> Vec<GeneratedCommand> {
        let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
        let methods = extract_methods(&doc);
        let allowed = filter_allowed(&methods);
        allowed.iter().map(to_generated_command).collect()
    }

    #[test]
    fn generate_all_produces_expected_groups() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        let names: Vec<&str> = skills.iter().map(|s| s.dir_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["dcx-datasets", "dcx-models", "dcx-routines", "dcx-tables"]
        );
    }

    #[test]
    fn generated_skill_md_has_frontmatter() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        for skill in &skills {
            assert!(
                skill.skill_md.starts_with("---\n"),
                "{}: missing frontmatter start",
                skill.dir_name
            );
            assert!(
                skill.skill_md.contains("name: "),
                "{}: missing name in frontmatter",
                skill.dir_name
            );
            assert!(
                skill.skill_md.contains("description: "),
                "{}: missing description in frontmatter",
                skill.dir_name
            );
        }
    }

    #[test]
    fn generated_skill_md_has_required_sections() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        for skill in &skills {
            let md = &skill.skill_md;
            assert!(md.contains("## When to use"), "{}", skill.dir_name);
            assert!(md.contains("## Prerequisites"), "{}", skill.dir_name);
            assert!(md.contains("## Commands"), "{}", skill.dir_name);
            assert!(md.contains("## Decision rules"), "{}", skill.dir_name);
            assert!(md.contains("## Examples"), "{}", skill.dir_name);
            assert!(md.contains("## Constraints"), "{}", skill.dir_name);
        }
    }

    #[test]
    fn generated_openai_yaml_has_required_fields() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        for skill in &skills {
            let yaml = &skill.openai_yaml;
            assert!(yaml.contains("display_name:"), "{}", skill.dir_name);
            assert!(yaml.contains("short_description:"), "{}", skill.dir_name);
            assert!(yaml.contains("default_prompt:"), "{}", skill.dir_name);
            assert!(
                yaml.contains("allow_implicit_invocation: true"),
                "{}",
                skill.dir_name
            );
        }
    }

    #[test]
    fn filter_skills_empty_returns_all() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        let filtered = filter_skills(skills.clone(), &[]);
        assert_eq!(filtered.len(), skills.len());
    }

    #[test]
    fn filter_skills_by_name() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        let filtered = filter_skills(skills, &["dcx-datasets".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].dir_name, "dcx-datasets");
    }

    #[test]
    fn filter_skills_nonexistent_returns_empty() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);
        let filtered = filter_skills(skills, &["dcx-nonexistent".to_string()]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn write_skills_creates_files() {
        let commands = load_generated_commands();
        let skills = generate_all(&commands);

        let tmp = tempfile::tempdir().unwrap();
        let written = write_skills(tmp.path(), &skills).unwrap();

        assert_eq!(written.len(), skills.len());
        for skill in &skills {
            let skill_md = tmp.path().join(&skill.dir_name).join("SKILL.md");
            let yaml = tmp.path().join(&skill.dir_name).join("agents/openai.yaml");
            assert!(skill_md.exists(), "Missing SKILL.md for {}", skill.dir_name);
            assert!(
                yaml.exists(),
                "Missing agents/openai.yaml for {}",
                skill.dir_name
            );

            let content = std::fs::read_to_string(&skill_md).unwrap();
            assert_eq!(content, skill.skill_md);
        }
    }
}
