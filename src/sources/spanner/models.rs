use serde::{Deserialize, Serialize};

// ── API response types ──

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancesListResponse {
    #[serde(default)]
    pub instances: Option<Vec<SpannerInstance>>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpannerInstance {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub config: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub node_count: Option<i32>,
    #[serde(default)]
    pub processing_units: Option<i32>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabasesListResponse {
    #[serde(default)]
    pub databases: Option<Vec<SpannerDatabase>>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpannerDatabase {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub database_dialect: Option<String>,
    #[serde(default)]
    pub version_retention_period: Option<String>,
}

// ── CLI response wrappers ──

#[derive(Debug, Serialize)]
pub struct SpannerInstancesCliResponse {
    pub project: String,
    pub instances: Vec<SpannerInstance>,
}

#[derive(Debug, Serialize)]
pub struct SpannerDatabasesCliResponse {
    pub project: String,
    pub instance: String,
    pub databases: Vec<SpannerDatabase>,
}
