use anyhow::Result;
use serde::Serialize;

use crate::ca::profiles;
use crate::cli::OutputFormat;
use crate::output;

#[derive(Debug, Serialize)]
pub struct ValidateResponse {
    pub name: String,
    pub source_type: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn run(profile_ref: &str, format: &OutputFormat) -> Result<()> {
    let result = profiles::resolve_profile(profile_ref);

    let response = match result {
        Ok(profile) => ValidateResponse {
            name: profile.name,
            source_type: profile.source_type.to_string(),
            valid: true,
            error: None,
        },
        Err(e) => ValidateResponse {
            name: profile_ref.to_string(),
            source_type: "unknown".to_string(),
            valid: false,
            error: Some(e.to_string()),
        },
    };

    if *format == OutputFormat::Text {
        render_text(&response);
        if !response.valid {
            std::process::exit(1);
        }
        return Ok(());
    }

    let is_valid = response.valid;
    output::render(&response, format)?;
    if !is_valid {
        std::process::exit(1);
    }
    Ok(())
}

fn render_text(resp: &ValidateResponse) {
    if resp.valid {
        println!("OK  {} ({})", resp.name, resp.source_type);
    } else {
        println!("FAIL  {}", resp.name);
        if let Some(ref err) = resp.error {
            println!("  Error: {}", err);
        }
    }
}
