use std::io::{BufRead, Write};
use std::net::TcpListener;

use anyhow::{bail, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use url::Url;

use super::store::{AuthStore, StoredToken};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const SCOPES: &str =
    "https://www.googleapis.com/auth/bigquery https://www.googleapis.com/auth/userinfo.email";

// Default OAuth client for bqx CLI (installed-app / desktop type).
// Users can override via BQX_CLIENT_ID / BQX_CLIENT_SECRET env vars.
const DEFAULT_CLIENT_ID: &str =
    "764086051850-6qr4p6gpi6hn506pt8ejuq83di341hur.apps.googleusercontent.com";
const DEFAULT_CLIENT_SECRET: &str = "d-FL95Q19q7MQmFpd7hHD0Ty";

/// Run the interactive login flow with PKCE and state protection.
pub async fn run_login() -> Result<()> {
    let client_id = std::env::var("BQX_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.into());
    let client_secret =
        std::env::var("BQX_CLIENT_SECRET").unwrap_or_else(|_| DEFAULT_CLIENT_SECRET.into());

    // Bind to a random port for the OAuth callback
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://localhost:{port}");

    // Generate PKCE code_verifier and code_challenge (S256)
    let code_verifier = generate_random_string(64);
    let code_challenge = {
        let hash = Sha256::digest(code_verifier.as_bytes());
        URL_SAFE_NO_PAD.encode(hash)
    };

    // Generate state parameter for CSRF protection
    let state = generate_random_string(32);

    // Build the authorization URL
    let auth_url = format!(
        "{GOOGLE_AUTH_URL}?\
         client_id={client_id}\
         &redirect_uri={redirect_uri}\
         &response_type=code\
         &scope={}\
         &access_type=offline\
         &prompt=consent\
         &state={state}\
         &code_challenge={code_challenge}\
         &code_challenge_method=S256",
        urlencoding(&SCOPES.replace(' ', "+"))
    );

    eprintln!("Opening browser for authentication...");
    eprintln!("If the browser does not open, visit this URL:");
    eprintln!();
    eprintln!("  {auth_url}");
    eprintln!();

    // Try to open the browser
    let _ = open::that(&auth_url);

    // Wait for the OAuth callback
    eprintln!("Waiting for authentication...");
    let auth_code = wait_for_callback(listener, &state)?;

    // Exchange code for tokens (with PKCE code_verifier)
    eprintln!("Exchanging authorization code for tokens...");
    let http = reqwest::Client::new();
    let token_resp = http
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("code", auth_code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .await?;

    if !token_resp.status().is_success() {
        let body = token_resp.text().await?;
        bail!("Token exchange failed: {body}");
    }

    let token_data: serde_json::Value = token_resp.json().await?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No access_token in response"))?
        .to_string();
    let refresh_token = token_data["refresh_token"].as_str().map(|s| s.to_string());

    // Get user email
    let account = fetch_user_email(&http, &access_token).await.ok();

    // Store credentials (including client_id/secret for refresh)
    let store = AuthStore::new();
    let stored = StoredToken {
        access_token,
        refresh_token,
        client_id: Some(client_id),
        client_secret: Some(client_secret),
        account: account.clone(),
    };
    store.save_token(&stored)?;

    let display_account = account.as_deref().unwrap_or("unknown");
    eprintln!();
    eprintln!("Authenticated as: {display_account}");
    eprintln!("Credentials stored securely in OS keychain.");
    if let Some(dir) = store.config_dir() {
        eprintln!("Metadata saved to: {}", dir.display());
    }

    Ok(())
}

/// Run the logout flow — clear stored credentials.
pub fn run_logout() -> Result<()> {
    let store = AuthStore::new();
    store.clear()?;
    eprintln!("Stored credentials cleared.");
    Ok(())
}

/// Show current auth status.
pub async fn run_status(opts: &super::AuthOptions) -> Result<()> {
    use super::resolver::{self, AuthSource};

    match resolver::resolve(opts).await {
        Ok(resolved) => {
            eprintln!("Active credential source: {}", resolved.source);
            match &resolved.source {
                AuthSource::ExplicitToken => {
                    eprintln!("  via: BQX_TOKEN environment variable or --token flag");
                }
                AuthSource::CredentialsFile(path) => {
                    eprintln!("  via: {path}");
                }
                AuthSource::StoredLogin(account) => {
                    eprintln!("  account: {account}");
                    let store = AuthStore::new();
                    if let Ok(Some(meta)) = store.load_metadata() {
                        eprintln!("  logged in: {}", meta.created_at);
                    }
                }
                AuthSource::GoogleApplicationCredentials(path) => {
                    eprintln!("  via: GOOGLE_APPLICATION_CREDENTIALS={path}");
                }
                AuthSource::DefaultAdc => {
                    eprintln!("  via: gcloud auth application-default login or metadata server");
                }
            }

            // Try to verify the token works
            match resolved.token().await {
                Ok(_) => eprintln!("\nToken: valid"),
                Err(e) => eprintln!("\nToken: error — {e}"),
            }
        }
        Err(e) => {
            eprintln!("No active credentials found.");
            eprintln!("Error: {e}");
            eprintln!();
            eprintln!("To authenticate, run one of:");
            eprintln!("  bqx auth login");
            eprintln!("  gcloud auth application-default login");
            eprintln!("  export BQX_TOKEN=<your-token>");
        }
    }

    Ok(())
}

/// Wait for the OAuth redirect callback, validate state, and extract the authorization code.
fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    let (mut stream, _) = listener.accept()?;
    let mut reader = std::io::BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse: GET /?code=xxx&state=yyy&scope=... HTTP/1.1
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Invalid HTTP request from callback"))?;

    let url = Url::parse(&format!("http://localhost{path}"))?;

    // Check for error
    let error = url
        .query_pairs()
        .find(|(k, _)| k == "error")
        .map(|(_, v)| v.to_string());

    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.to_string());

    let returned_state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string());

    // Send response to browser
    let (status, body) = if error.is_some() {
        (
            "400 Bad Request",
            "Authentication failed. Please check the terminal for details.",
        )
    } else if code.is_some() {
        (
            "200 OK",
            "Authentication successful! You can close this tab and return to the terminal.",
        )
    } else {
        (
            "400 Bad Request",
            "Authentication failed. No authorization code received.",
        )
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
         <html><body><h2>{body}</h2></body></html>"
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    if let Some(err) = error {
        bail!("Authentication failed: {err}");
    }

    // Validate state to prevent CSRF
    match returned_state {
        Some(ref s) if s == expected_state => {}
        Some(s) => {
            bail!("OAuth state mismatch (possible CSRF). Expected: {expected_state}, got: {s}")
        }
        None => bail!("OAuth callback missing state parameter"),
    }

    code.ok_or_else(|| anyhow::anyhow!("No authorization code received in callback"))
}

async fn fetch_user_email(http: &reqwest::Client, access_token: &str) -> Result<String> {
    let resp: serde_json::Value = http
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await?
        .json()
        .await?;

    resp["email"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not determine email"))
}

/// Minimal URL encoding for scope string (spaces already replaced with +).
fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F")
}

/// Generate a cryptographically random URL-safe string (cross-platform).
fn generate_random_string(len: usize) -> String {
    use rand::RngExt;
    let mut bytes = vec![0u8; len];
    rand::rng().fill(&mut bytes[..]);
    let encoded = URL_SAFE_NO_PAD.encode(&bytes);
    encoded[..len].to_string()
}
