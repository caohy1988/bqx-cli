use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;

/// Where to load the Discovery Document from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverySource {
    /// Compiled-in pinned copy (default, deterministic).
    Bundled,
    /// Local filesystem cache.
    Cache,
    /// Fetch from Google's Discovery endpoint.
    Remote,
}

/// Top-level Discovery Document (fields we care about).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryDocument {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub base_url: String,
    #[serde(default)]
    pub resources: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub schemas: serde_json::Map<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Bundled asset
// ---------------------------------------------------------------------------

const BUNDLED_DISCOVERY: &str = include_str!("../../assets/bigquery_v2_discovery.json");

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Load a DiscoveryDocument from the given source.
pub fn load(source: &DiscoverySource) -> Result<DiscoveryDocument> {
    match source {
        DiscoverySource::Bundled => load_bundled(),
        DiscoverySource::Cache => read_cache(),
        DiscoverySource::Remote => {
            bail!("Remote discovery loading requires async context; use load_remote()")
        }
    }
}

fn load_bundled() -> Result<DiscoveryDocument> {
    serde_json::from_str(BUNDLED_DISCOVERY).context("Failed to parse bundled Discovery Document")
}

/// Fetch the Discovery Document from Google's endpoint.
pub async fn load_remote() -> Result<DiscoveryDocument> {
    let url = "https://bigquery.googleapis.com/$discovery/rest?version=v2";
    let resp = reqwest::get(url)
        .await
        .context("Failed to fetch Discovery Document")?;

    if !resp.status().is_success() {
        bail!(
            "Discovery endpoint returned HTTP {}",
            resp.status().as_u16()
        );
    }

    let raw = resp
        .text()
        .await
        .context("Failed to read Discovery response body")?;

    // Cache the raw response for future use.
    if let Err(e) = write_cache(&raw) {
        eprintln!("Warning: could not write discovery cache: {e}");
    }

    serde_json::from_str(&raw).context("Failed to parse remote Discovery Document")
}

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

const CACHE_FILENAME: &str = "bigquery_v2.json";

/// Return the cache directory path: <user-cache-dir>/bqx/discovery/
pub fn cache_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "bqx")
        .ok_or_else(|| anyhow::anyhow!("Cannot determine cache directory"))?;
    Ok(proj_dirs.cache_dir().join("discovery"))
}

/// Return the full path to the cached Discovery Document.
pub fn cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join(CACHE_FILENAME))
}

/// Read a DiscoveryDocument from the local cache.
pub fn read_cache() -> Result<DiscoveryDocument> {
    let path = cache_path()?;
    let raw =
        fs::read_to_string(&path).with_context(|| format!("No cached discovery at {}", path.display()))?;
    serde_json::from_str(&raw).context("Failed to parse cached Discovery Document")
}

/// Write raw Discovery JSON to the local cache.
pub fn write_cache(raw: &str) -> Result<()> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create cache directory: {}", dir.display()))?;
    let path = dir.join(CACHE_FILENAME);
    fs::write(&path, raw)
        .with_context(|| format!("Failed to write discovery cache: {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_loads_without_network() {
        let doc = load(&DiscoverySource::Bundled).expect("bundled discovery should load");
        assert_eq!(doc.name, "bigquery");
        assert_eq!(doc.version, "v2");
    }

    #[test]
    fn bundled_has_expected_fields() {
        let doc = load(&DiscoverySource::Bundled).unwrap();
        assert!(!doc.revision.is_empty(), "revision should be non-empty");
        assert!(
            doc.base_url.contains("bigquery.googleapis.com"),
            "base_url should point to BigQuery"
        );
        assert!(!doc.resources.is_empty(), "resources should be non-empty");
        assert!(!doc.schemas.is_empty(), "schemas should be non-empty");
    }

    #[test]
    fn bundled_contains_expected_resources() {
        let doc = load(&DiscoverySource::Bundled).unwrap();
        assert!(doc.resources.contains_key("datasets"));
        assert!(doc.resources.contains_key("tables"));
        assert!(doc.resources.contains_key("jobs"));
    }

    #[test]
    fn cache_round_trip() {
        let raw = r#"{
            "name": "bigquery",
            "version": "v2",
            "revision": "test123",
            "baseUrl": "https://example.com/",
            "resources": {},
            "schemas": {}
        }"#;

        let tmp = tempfile::tempdir().unwrap();
        let cache_file = tmp.path().join("bigquery_v2.json");
        std::fs::write(&cache_file, raw).unwrap();

        let doc: DiscoveryDocument =
            serde_json::from_str(&std::fs::read_to_string(&cache_file).unwrap()).unwrap();
        assert_eq!(doc.name, "bigquery");
        assert_eq!(doc.revision, "test123");
    }

    #[test]
    fn remote_source_errors_without_async() {
        let result = load(&DiscoverySource::Remote);
        assert!(result.is_err());
    }
}
