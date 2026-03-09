use std::sync::Arc;

use anyhow::Result;
use gcp_auth::{Token, TokenProvider};

const BQ_SCOPE: &str = "https://www.googleapis.com/auth/bigquery";

pub struct AuthProvider {
    provider: Arc<dyn TokenProvider>,
}

impl AuthProvider {
    pub async fn new() -> Result<Self> {
        let provider = gcp_auth::provider().await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to initialize ADC authentication. \
                 Run 'gcloud auth application-default login' first.\n\
                 Error: {e}"
            )
        })?;
        Ok(Self { provider })
    }

    pub async fn token(&self) -> Result<Arc<Token>> {
        let token = self
            .provider
            .token(&[BQ_SCOPE])
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get access token: {e}"))?;
        Ok(token)
    }
}
