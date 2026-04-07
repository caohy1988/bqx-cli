use anyhow::Result;
use serde::Serialize;

use crate::auth;
use crate::ca::profiles;
use crate::cli::OutputFormat;
use crate::output;

#[derive(Debug, Serialize)]
pub struct TestResponse {
    pub name: String,
    pub source_type: String,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn run(
    profile_ref: &str,
    auth_opts: &auth::AuthOptions,
    format: &OutputFormat,
) -> Result<()> {
    // Step 1: structural validation
    let profile = match profiles::resolve_profile(profile_ref) {
        Ok(p) => p,
        Err(e) => {
            let response = TestResponse {
                name: profile_ref.to_string(),
                source_type: "unknown".to_string(),
                valid: false,
                auth_source: None,
                auth_valid: None,
                error: Some(e.to_string()),
            };
            emit(&response, format)?;
            std::process::exit(1);
        }
    };

    // Step 2: resolve auth, get a token, and verify it against Google tokeninfo
    let (auth_source, auth_valid, error) = match auth::resolve(auth_opts).await {
        Ok(resolved) => {
            let source = resolved.source.to_string();
            match resolved.token().await {
                Ok(token) => match auth::login::verify_token(&token).await {
                    Ok(_) => (Some(source), Some(true), None),
                    Err(e) => (Some(source), Some(false), Some(e.to_string())),
                },
                Err(e) => (Some(source), Some(false), Some(e.to_string())),
            }
        }
        Err(e) => (None, Some(false), Some(e.to_string())),
    };

    let all_valid = auth_valid == Some(true);
    let response = TestResponse {
        name: profile.name,
        source_type: profile.source_type.to_string(),
        valid: all_valid,
        auth_source,
        auth_valid,
        error,
    };

    emit(&response, format)?;

    if !all_valid {
        std::process::exit(1);
    }

    Ok(())
}

fn emit(response: &TestResponse, format: &OutputFormat) -> Result<()> {
    if *format == OutputFormat::Text {
        render_text(response);
        return Ok(());
    }
    output::render(response, format)
}

fn render_text(resp: &TestResponse) {
    if resp.valid {
        println!("OK  {} ({})", resp.name, resp.source_type);
        if let Some(ref src) = resp.auth_source {
            println!("  auth: {src}");
        }
    } else {
        println!("FAIL  {} ({})", resp.name, resp.source_type);
        if let Some(ref err) = resp.error {
            println!("  error: {err}");
        }
    }
}
