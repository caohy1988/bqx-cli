use anyhow::Result;
use serde::{Deserialize, Serialize};

const KEYRING_SERVICE: &str = "bqx-cli";
const KEYRING_USER: &str = "default";
const CONFIG_FILE: &str = "auth.json";

/// Metadata about the stored login, persisted to the config directory.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredAuthMetadata {
    pub account: Option<String>,
    pub created_at: String,
}

/// Token data stored in the OS keychain.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub account: Option<String>,
}

pub struct AuthStore {
    config_dir: Option<std::path::PathBuf>,
}

impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthStore {
    pub fn new() -> Self {
        let config_dir =
            directories::ProjectDirs::from("", "", "bqx").map(|d| d.config_dir().to_path_buf());
        Self { config_dir }
    }

    /// Store a token in the OS keychain and metadata in the config directory.
    pub fn save_token(&self, token: &StoredToken) -> Result<()> {
        // Store the token JSON in the keychain
        let token_json = serde_json::to_string(token)?;
        let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
            .map_err(|e| anyhow::anyhow!("Failed to access keychain: {e}"))?;
        entry
            .set_password(&token_json)
            .map_err(|e| anyhow::anyhow!("Failed to store credentials in keychain: {e}"))?;

        // Store metadata in config directory
        if let Some(ref dir) = self.config_dir {
            std::fs::create_dir_all(dir)?;
            let meta = StoredAuthMetadata {
                account: token.account.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            let meta_json = serde_json::to_string_pretty(&meta)?;
            std::fs::write(dir.join(CONFIG_FILE), meta_json)?;
        }

        Ok(())
    }

    /// Load a stored token from the OS keychain.
    pub fn load_token(&self) -> Result<Option<StoredToken>> {
        let entry = match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };
        match entry.get_password() {
            Ok(json) => {
                let token: StoredToken = serde_json::from_str(&json)?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Load metadata from the config directory.
    pub fn load_metadata(&self) -> Result<Option<StoredAuthMetadata>> {
        let dir = match &self.config_dir {
            Some(d) => d,
            None => return Ok(None),
        };
        let path = dir.join(CONFIG_FILE);
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let meta: StoredAuthMetadata = serde_json::from_str(&json)?;
        Ok(Some(meta))
    }

    /// Clear stored credentials.
    pub fn clear(&self) -> Result<()> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
            let _ = entry.delete_credential();
        }
        if let Some(ref dir) = self.config_dir {
            let path = dir.join(CONFIG_FILE);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// Return the config directory path.
    pub fn config_dir(&self) -> Option<&std::path::Path> {
        self.config_dir.as_deref()
    }
}
