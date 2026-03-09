use std::sync::Arc;

use anyhow::Result;
use gcp_auth::TokenProvider;

use super::store::AuthStore;

const BQ_SCOPE: &str = "https://www.googleapis.com/auth/bigquery";

/// The credential source that was used to obtain a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthSource {
    /// BQX_TOKEN env var or --token flag
    ExplicitToken,
    /// BQX_CREDENTIALS_FILE env var or --credentials-file flag
    CredentialsFile(String),
    /// Stored credentials from `bqx auth login`
    StoredLogin(String),
    /// GOOGLE_APPLICATION_CREDENTIALS or default ADC
    ApplicationDefault,
}

impl std::fmt::Display for AuthSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthSource::ExplicitToken => write!(f, "BQX_TOKEN / --token"),
            AuthSource::CredentialsFile(path) => write!(f, "credentials file: {path}"),
            AuthSource::StoredLogin(account) => write!(f, "bqx auth login ({account})"),
            AuthSource::ApplicationDefault => write!(f, "application default credentials (ADC)"),
        }
    }
}

/// Resolved credential ready to produce bearer tokens.
pub struct ResolvedAuth {
    pub source: AuthSource,
    inner: AuthInner,
}

enum AuthInner {
    StaticToken(String),
    GcpProvider(Arc<dyn TokenProvider>),
}

impl ResolvedAuth {
    /// Get a bearer token string.
    pub async fn token(&self) -> Result<String> {
        match &self.inner {
            AuthInner::StaticToken(t) => Ok(t.clone()),
            AuthInner::GcpProvider(p) => {
                let tok = p
                    .token(&[BQ_SCOPE])
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to get access token: {e}"))?;
                Ok(tok.as_str().to_string())
            }
        }
    }
}

/// Credential resolution options, typically populated from CLI flags and env vars.
pub struct AuthOptions {
    pub token: Option<String>,
    pub credentials_file: Option<String>,
}

/// Resolve credentials using the Phase 1 precedence chain:
///
/// 1. `BQX_TOKEN` env var / `--token` flag
/// 2. `BQX_CREDENTIALS_FILE` env var / `--credentials-file` flag
/// 3. Stored `bqx auth login` credentials
/// 4. `GOOGLE_APPLICATION_CREDENTIALS` / default ADC
pub async fn resolve(opts: &AuthOptions) -> Result<ResolvedAuth> {
    // 1. Explicit token
    if let Some(ref token) = opts.token {
        return Ok(ResolvedAuth {
            source: AuthSource::ExplicitToken,
            inner: AuthInner::StaticToken(token.clone()),
        });
    }

    // 2. Credentials file
    if let Some(ref path) = opts.credentials_file {
        // Set GOOGLE_APPLICATION_CREDENTIALS so gcp_auth picks it up
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", path);
        let provider = gcp_auth::provider()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load credentials from '{path}': {e}"))?;
        return Ok(ResolvedAuth {
            source: AuthSource::CredentialsFile(path.clone()),
            inner: AuthInner::GcpProvider(provider),
        });
    }

    // 3. Stored login credentials
    let store = AuthStore::new();
    if let Ok(Some(stored)) = store.load_token() {
        if let Some(ref account) = stored.account {
            // Verify the stored token is still usable
            return Ok(ResolvedAuth {
                source: AuthSource::StoredLogin(account.clone()),
                inner: AuthInner::StaticToken(stored.access_token),
            });
        }
    }

    // 4. ADC fallback (covers GOOGLE_APPLICATION_CREDENTIALS and gcloud ADC)
    let provider = gcp_auth::provider().await.map_err(|e| {
        anyhow::anyhow!(
            "No credentials found. Options:\n\
             - Set BQX_TOKEN or --token\n\
             - Set BQX_CREDENTIALS_FILE or --credentials-file\n\
             - Run 'bqx auth login'\n\
             - Run 'gcloud auth application-default login'\n\
             Error: {e}"
        )
    })?;

    Ok(ResolvedAuth {
        source: AuthSource::ApplicationDefault,
        inner: AuthInner::GcpProvider(provider),
    })
}
