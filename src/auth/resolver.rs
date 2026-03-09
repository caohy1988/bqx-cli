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

    // 2. Credentials file (supports both service account and authorized user JSON)
    if let Some(ref path) = opts.credentials_file {
        let resolved = resolve_credentials_file(path).await?;
        return Ok(resolved);
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

/// Load credentials from a JSON file. Detects whether it's a service account
/// (has `private_key`) or an authorized user (has `client_id` + `refresh_token`)
/// and handles each appropriately.
async fn resolve_credentials_file(path: &str) -> Result<ResolvedAuth> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Cannot read credentials file '{path}': {e}"))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid JSON in credentials file '{path}': {e}"))?;

    let cred_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match cred_type {
        "service_account" => {
            // Service account: use gcp_auth via GOOGLE_APPLICATION_CREDENTIALS
            std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", path);
            let provider = gcp_auth::provider().await.map_err(|e| {
                anyhow::anyhow!("Failed to load service account from '{path}': {e}")
            })?;
            Ok(ResolvedAuth {
                source: AuthSource::CredentialsFile(path.to_string()),
                inner: AuthInner::GcpProvider(provider),
            })
        }
        "authorized_user" => {
            // Authorized user: exchange refresh_token for access_token
            let client_id = json["client_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing client_id in '{path}'"))?;
            let client_secret = json["client_secret"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing client_secret in '{path}'"))?;
            let refresh_token = json["refresh_token"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing refresh_token in '{path}'"))?;

            let token = refresh_access_token(client_id, client_secret, refresh_token).await?;
            Ok(ResolvedAuth {
                source: AuthSource::CredentialsFile(path.to_string()),
                inner: AuthInner::StaticToken(token),
            })
        }
        _ => {
            anyhow::bail!(
                "Unsupported credential type '{}' in '{path}'. \
                 Expected 'service_account' or 'authorized_user'.",
                if cred_type.is_empty() {
                    "(missing)"
                } else {
                    cred_type
                }
            );
        }
    }
}

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Exchange a refresh token for an access token.
async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String> {
    let http = reqwest::Client::new();
    let resp = http
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await?;
        anyhow::bail!("Token refresh failed: {body}");
    }

    let data: serde_json::Value = resp.json().await?;
    data["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No access_token in refresh response"))
}
