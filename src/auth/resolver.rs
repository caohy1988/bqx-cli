use std::sync::Arc;

use anyhow::Result;
use gcp_auth::TokenProvider;

use super::store::AuthStore;

const BQ_SCOPE: &str = "https://www.googleapis.com/auth/bigquery";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// The credential source that was used to obtain a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthSource {
    /// BQX_TOKEN env var or --token flag
    ExplicitToken,
    /// BQX_CREDENTIALS_FILE env var or --credentials-file flag
    CredentialsFile(String),
    /// Stored credentials from `bqx auth login`
    StoredLogin(String),
    /// GOOGLE_APPLICATION_CREDENTIALS env var
    GoogleApplicationCredentials(String),
    /// Default ADC (gcloud auth application-default login or metadata server)
    DefaultAdc,
}

impl std::fmt::Display for AuthSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthSource::ExplicitToken => write!(f, "BQX_TOKEN / --token"),
            AuthSource::CredentialsFile(path) => write!(f, "credentials file: {path}"),
            AuthSource::StoredLogin(account) => write!(f, "bqx auth login ({account})"),
            AuthSource::GoogleApplicationCredentials(path) => {
                write!(f, "GOOGLE_APPLICATION_CREDENTIALS: {path}")
            }
            AuthSource::DefaultAdc => {
                write!(f, "application default credentials (gcloud ADC)")
            }
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
    Refreshable(RefreshableToken),
    GcpProvider(Arc<dyn TokenProvider>),
}

/// A token that can be refreshed using a stored refresh_token.
struct RefreshableToken {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    token_url: String,
}

impl ResolvedAuth {
    /// Get a bearer token string.
    pub async fn token(&self) -> Result<String> {
        match &self.inner {
            AuthInner::StaticToken(t) => Ok(t.clone()),
            AuthInner::Refreshable(r) => {
                refresh_access_token(
                    &r.token_url,
                    &r.client_id,
                    &r.client_secret,
                    &r.refresh_token,
                )
                .await
            }
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
/// 3. Stored `bqx auth login` credentials (uses refresh_token)
/// 4. `GOOGLE_APPLICATION_CREDENTIALS`
/// 5. Default ADC / `gcloud auth application-default`
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
        return resolve_credentials_file(path).await;
    }

    // 3. Stored login credentials — use refresh_token for durable auth
    let store = AuthStore::new();
    if let Ok(Some(stored)) = store.load_token() {
        if let Some(ref account) = stored.account {
            // Only use refreshable path if we have all three: refresh_token + client_id + client_secret
            match (
                &stored.refresh_token,
                &stored.client_id,
                &stored.client_secret,
            ) {
                (Some(refresh_token), Some(client_id), Some(client_secret))
                    if !client_id.is_empty() && !client_secret.is_empty() =>
                {
                    return Ok(ResolvedAuth {
                        source: AuthSource::StoredLogin(account.clone()),
                        inner: AuthInner::Refreshable(RefreshableToken {
                            client_id: client_id.clone(),
                            client_secret: client_secret.clone(),
                            refresh_token: refresh_token.clone(),
                            token_url: GOOGLE_TOKEN_URL.to_string(),
                        }),
                    });
                }
                _ => {
                    // Legacy stored token or missing client credentials — use access_token
                    // as static fallback. User should re-login for durable auth.
                    return Ok(ResolvedAuth {
                        source: AuthSource::StoredLogin(account.clone()),
                        inner: AuthInner::StaticToken(stored.access_token),
                    });
                }
            }
        }
    }

    // 4. GOOGLE_APPLICATION_CREDENTIALS (explicit env var)
    if let Ok(gac_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        let provider = gcp_auth::provider().await.map_err(|e| {
            anyhow::anyhow!(
                "Failed to load credentials from GOOGLE_APPLICATION_CREDENTIALS='{gac_path}': {e}"
            )
        })?;
        return Ok(ResolvedAuth {
            source: AuthSource::GoogleApplicationCredentials(gac_path),
            inner: AuthInner::GcpProvider(provider),
        });
    }

    // 5. Default ADC fallback (gcloud ADC or metadata server)
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
        source: AuthSource::DefaultAdc,
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
            // Authorized user: use refresh_token to get fresh access tokens
            let client_id = json["client_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing client_id in '{path}'"))?;
            let client_secret = json["client_secret"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing client_secret in '{path}'"))?;
            let refresh_token = json["refresh_token"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing refresh_token in '{path}'"))?;

            Ok(ResolvedAuth {
                source: AuthSource::CredentialsFile(path.to_string()),
                inner: AuthInner::Refreshable(RefreshableToken {
                    client_id: client_id.to_string(),
                    client_secret: client_secret.to_string(),
                    refresh_token: refresh_token.to_string(),
                    token_url: GOOGLE_TOKEN_URL.to_string(),
                }),
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

/// Construct a `ResolvedAuth` that will refresh using the given credentials.
/// Exposed for testing only.
#[cfg(test)]
pub(crate) fn make_refreshable(
    source: AuthSource,
    client_id: String,
    client_secret: String,
    refresh_token: String,
    token_url: String,
) -> ResolvedAuth {
    ResolvedAuth {
        source,
        inner: AuthInner::Refreshable(RefreshableToken {
            client_id,
            client_secret,
            refresh_token,
            token_url,
        }),
    }
}

/// Exchange a refresh token for an access token.
async fn refresh_access_token(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<String> {
    let http = reqwest::Client::new();
    let resp = http
        .post(token_url)
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Verify the Refreshable path exchanges a refresh token for an access token.
    #[tokio::test]
    async fn refreshable_token_exchanges_via_token_endpoint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=test-refresh"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"access_token": "fresh-token"})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let auth = make_refreshable(
            AuthSource::StoredLogin("test@example.com".into()),
            "test-client-id".into(),
            "test-client-secret".into(),
            "test-refresh".into(),
            mock_server.uri(),
        );

        let token = auth.token().await.unwrap();
        assert_eq!(token, "fresh-token");
    }

    /// Verify the Refreshable path surfaces token endpoint errors.
    #[tokio::test]
    async fn refreshable_token_surfaces_endpoint_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(401).set_body_string(r#"{"error":"invalid_grant"}"#),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let auth = make_refreshable(
            AuthSource::StoredLogin("test@example.com".into()),
            "bad-id".into(),
            "bad-secret".into(),
            "bad-refresh".into(),
            mock_server.uri(),
        );

        let result = auth.token().await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Token refresh failed"),
            "Expected 'Token refresh failed', got: {err}"
        );
    }

    /// Verify static token path returns the token directly without network.
    #[tokio::test]
    async fn static_token_returns_immediately() {
        let auth = ResolvedAuth {
            source: AuthSource::ExplicitToken,
            inner: AuthInner::StaticToken("my-bearer-token".into()),
        };

        let token = auth.token().await.unwrap();
        assert_eq!(token, "my-bearer-token");
    }
}
