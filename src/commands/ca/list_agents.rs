use anyhow::Result;

use crate::auth::{self, AuthOptions};
use crate::ca::client::{CaAgentManager, CaClient};
use crate::ca::models::ListAgentsResponse;
use crate::cli::OutputFormat;
use crate::config::Config;
use crate::output;

pub async fn run(auth_opts: &AuthOptions, config: &Config) -> Result<()> {
    let resolved = auth::resolve(auth_opts).await?;
    let client = CaClient::new(resolved);
    let resp = client
        .list_agents(&config.project_id, &config.location)
        .await?;

    render_response(&resp, &config.format)
}

pub async fn run_with_executor(executor: &dyn CaAgentManager, config: &Config) -> Result<()> {
    let resp = executor
        .list_agents(&config.project_id, &config.location)
        .await?;

    render_response(&resp, &config.format)
}

fn render_response(resp: &ListAgentsResponse, format: &OutputFormat) -> Result<()> {
    if *format == OutputFormat::Text {
        output::text::render_list_agents(resp);
        return Ok(());
    }
    output::render(resp, format)
}
