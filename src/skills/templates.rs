use crate::bigquery::dynamic::model::{GeneratedCommand, ParamLocation};

/// Parameters that are provided by global CLI flags and should be
/// documented as global rather than per-command flags.
const GLOBAL_PARAMS: &[&str] = &["projectId", "datasetId"];

/// A generated skill ready to be written to disk.
#[derive(Debug, Clone)]
pub struct SkillOutput {
    /// Skill directory name, e.g. "bqx-datasets"
    pub dir_name: String,
    /// SKILL.md content
    pub skill_md: String,
    /// agents/openai.yaml content
    pub openai_yaml: String,
}

/// Generate a skill for a resource group (e.g. "datasets") from its commands.
pub fn generate_resource_skill(group: &str, commands: &[&GeneratedCommand]) -> SkillOutput {
    let dir_name = format!("bqx-{group}");
    let display_name = capitalize(group);

    let skill_md = build_skill_md(group, &display_name, commands);
    let openai_yaml = build_openai_yaml(group, &display_name, commands);

    SkillOutput {
        dir_name,
        skill_md,
        openai_yaml,
    }
}

fn build_skill_md(group: &str, _display_name: &str, commands: &[&GeneratedCommand]) -> String {
    let mut out = String::new();

    // Frontmatter
    let action_list: Vec<&str> = commands.iter().map(|c| c.action.as_str()).collect();
    let actions_str = action_list.join(", ");
    out.push_str(&format!(
        "---\n\
         name: bqx-{group}\n\
         description: Use bqx to manage BigQuery {group} via the {actions_str} commands. \
         Generated from the BigQuery v2 Discovery API.\n\
         ---\n\n"
    ));

    // When to use
    out.push_str("## When to use this skill\n\n");
    out.push_str("Use when the user asks about:\n");
    for cmd in commands {
        out.push_str(&format!(
            "- \"{} a BigQuery {}\"\n",
            cmd.action,
            singular(group)
        ));
    }
    out.push_str(&format!(
        "- \"show me BigQuery {group}\"\n\
         - \"what {group} are in my project\"\n\n"
    ));
    out.push_str(
        "Do not use when the user wants analytics workflows (doctor, evaluate, get-trace) \
         — use bqx-analytics instead.\n\n",
    );

    // Prerequisites
    out.push_str("## Prerequisites\n\n");
    out.push_str("See **bqx-shared** for authentication and global flags.\n\n");

    let needs_dataset = commands.iter().any(|c| {
        c.method
            .parameters
            .iter()
            .any(|p| p.name == "datasetId" && p.location == ParamLocation::Path)
    });
    if needs_dataset {
        out.push_str("Requires: `--project-id` and `--dataset-id`\n\n");
    } else {
        out.push_str("Requires: `--project-id`\n\n");
    }

    // Commands section
    out.push_str("## Commands\n\n");
    for cmd in commands {
        out.push_str(&format!("### {} {}\n\n", group, cmd.action));
        out.push_str(&format!("{}\n\n", cmd.about));

        // Build usage line
        let mut usage = format!("```bash\nbqx {group} {}", cmd.action);
        let user_args: Vec<_> = cmd
            .args
            .iter()
            .filter(|a| !GLOBAL_PARAMS.contains(&a.api_name.as_str()))
            .collect();
        for arg in &user_args {
            if arg.required {
                usage.push_str(&format!(" \\\n  --{} <{}>", arg.flag_name, arg.api_name));
            }
        }
        for arg in &user_args {
            if !arg.required {
                usage.push_str(&format!(" \\\n  [--{}]", arg.flag_name));
            }
        }
        usage.push_str(" \\\n  [--dry-run] \\\n  [--format json|table|text]\n```\n\n");
        out.push_str(&usage);

        // Flags table
        if !user_args.is_empty() {
            out.push_str("| Flag | Required | Description |\n");
            out.push_str("|------|----------|-------------|\n");
            for arg in &user_args {
                let req = if arg.required { "Yes" } else { "No" };
                let help_text = arg.help.trim();
                let desc =
                    if help_text.is_empty() || help_text == "Required." || help_text == "Optional."
                    {
                        format!("{} parameter", arg.api_name)
                    } else {
                        truncate_description(help_text)
                    };
                out.push_str(&format!("| `--{}` | {} | {} |\n", arg.flag_name, req, desc));
            }
            out.push('\n');
        }
    }

    // Decision rules
    out.push_str("## Decision rules\n\n");
    out.push_str("- Use `--dry-run` to see the API request without executing it\n");
    out.push_str("- Use `--format table` for scanning results visually in a terminal\n");
    out.push_str("- Use `--format json` when piping output to other tools or scripts\n");
    if commands.iter().any(|c| c.action == "list") {
        out.push_str(&format!(
            "- Use `{group} list` to discover available {group} in a project\n"
        ));
    }
    if commands.iter().any(|c| c.action == "get") {
        out.push_str(&format!(
            "- Use `{group} get` to inspect a specific {singular}'s metadata\n",
            singular = singular(group)
        ));
    }
    out.push('\n');

    // Examples
    out.push_str("## Examples\n\n```bash\n");
    for cmd in commands {
        let cmd_needs_dataset = cmd
            .method
            .parameters
            .iter()
            .any(|p| p.name == "datasetId" && p.location == ParamLocation::Path);

        out.push_str(&format!("# {} {}\n", capitalize(&cmd.action), group));
        out.push_str(&format!("bqx {group} {}", cmd.action));
        out.push_str(" \\\n  --project-id my-proj");

        if cmd_needs_dataset {
            out.push_str(" \\\n  --dataset-id my_dataset");
        }

        let user_required: Vec<_> = cmd
            .args
            .iter()
            .filter(|a| a.required && !GLOBAL_PARAMS.contains(&a.api_name.as_str()))
            .collect();
        for arg in &user_required {
            out.push_str(&format!(
                " \\\n  --{} my_{}",
                arg.flag_name,
                arg.api_name.to_lowercase()
            ));
        }

        out.push_str(" \\\n  --format table\n\n");
    }
    out.push_str("```\n\n");

    // Constraints
    out.push_str("## Constraints\n\n");
    out.push_str(&format!(
        "- These commands are generated from the BigQuery v2 Discovery API\n\
         - Only read operations are supported in Phase 2\n\
         - Nested response objects are summarized in table format; use `--format json` for full detail\n\
         - Reference objects (e.g. {group}Reference) are automatically flattened in table output\n"
    ));

    out
}

fn build_openai_yaml(group: &str, display_name: &str, commands: &[&GeneratedCommand]) -> String {
    let action_list: Vec<&str> = commands.iter().map(|c| c.action.as_str()).collect();
    let actions_str = action_list.join("/");

    format!(
        "interface:\n  \
         display_name: \"bqx {display_name}\"\n  \
         short_description: \"{actions_str} BigQuery {group} via bqx\"\n  \
         default_prompt: \"Use $bqx-{group} to {actions_str} BigQuery {group} using the bqx CLI.\"\n\n\
         policy:\n  \
         allow_implicit_invocation: true\n"
    )
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn singular(group: &str) -> &str {
    match group {
        "datasets" => "dataset",
        "tables" => "table",
        "jobs" => "job",
        "routines" => "routine",
        "models" => "model",
        _ => group,
    }
}

/// Truncate Discovery descriptions to keep table cells readable.
fn truncate_description(s: &str) -> String {
    let first_sentence = s.split(". ").next().unwrap_or(s);
    if first_sentence.len() > 120 {
        format!("{}...", &first_sentence[..117])
    } else {
        first_sentence.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalize_works() {
        assert_eq!(capitalize("datasets"), "Datasets");
        assert_eq!(capitalize("tables"), "Tables");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn singular_works() {
        assert_eq!(singular("datasets"), "dataset");
        assert_eq!(singular("tables"), "table");
        assert_eq!(singular("unknown"), "unknown");
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate_description("A short desc"), "A short desc");
    }

    #[test]
    fn truncate_multi_sentence() {
        let desc = "First sentence. Second sentence. Third.";
        assert_eq!(truncate_description(desc), "First sentence");
    }

    #[test]
    fn truncate_long_single_sentence() {
        let long = "A".repeat(200);
        let result = truncate_description(&long);
        assert!(result.len() <= 120);
        assert!(result.ends_with("..."));
    }
}
