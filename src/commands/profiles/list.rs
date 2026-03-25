use anyhow::Result;
use serde::Serialize;

use crate::ca::profiles;
use crate::cli::OutputFormat;
use crate::output;

#[derive(Debug, Serialize)]
pub struct ProfileListResponse {
    pub profiles: Vec<ProfileSummary>,
}

#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    pub name: String,
    pub source_type: String,
    pub family: String,
    pub project: String,
    pub origin: String,
}

pub fn run(format: &OutputFormat) -> Result<()> {
    let discovered = profiles::discover_all_profiles()?;

    let summaries: Vec<ProfileSummary> = discovered
        .into_iter()
        .map(|(p, origin)| ProfileSummary {
            name: p.name,
            source_type: p.source_type.to_string(),
            family: format!("{:?}", p.source_type.family()),
            project: p.project,
            origin,
        })
        .collect();

    let response = ProfileListResponse {
        profiles: summaries,
    };

    if *format == OutputFormat::Text {
        render_text(&response);
        return Ok(());
    }

    output::render(&response, format)
}

fn render_text(resp: &ProfileListResponse) {
    if resp.profiles.is_empty() {
        println!("No profiles found.");
        println!();
        println!("Profile search directories:");
        if let Some(user_dir) = profiles::user_profiles_dir() {
            println!("  - {} (user-local)", user_dir.display());
        } else {
            println!("  - ~/.config/dcx/profiles/ (not found)");
        }
        let repo_dir = profiles::repo_profiles_dir();
        if repo_dir.exists() {
            println!("  - {} (repo-local)", repo_dir.display());
        } else {
            println!("  - deploy/ca/profiles/ (not found)");
        }
        return;
    }

    let columns: Vec<String> = ["Name", "Source Type", "Family", "Project", "Origin"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let rows: Vec<Vec<String>> = resp
        .profiles
        .iter()
        .map(|p| {
            vec![
                p.name.clone(),
                p.source_type.clone(),
                p.family.clone(),
                p.project.clone(),
                p.origin.clone(),
            ]
        })
        .collect();

    println!("{}", output::fmt_rows_as_table(&columns, &rows));
}
