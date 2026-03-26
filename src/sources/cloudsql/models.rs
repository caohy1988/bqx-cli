use serde::{Deserialize, Serialize};

// ── API response types ──

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancesListResponse {
    #[serde(default)]
    pub items: Option<Vec<CloudSqlInstance>>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSqlInstance {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub database_version: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub gce_zone: Option<String>,
    #[serde(default)]
    pub settings: Option<CloudSqlSettings>,
    #[serde(default)]
    pub connection_name: Option<String>,
    #[serde(default)]
    pub instance_type: Option<String>,
    #[serde(default)]
    pub ip_addresses: Option<Vec<CloudSqlIpAddress>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSqlSettings {
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub data_disk_size_gb: Option<String>,
    #[serde(default)]
    pub data_disk_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSqlIpAddress {
    #[serde(rename = "type", default)]
    pub ip_type: Option<String>,
    #[serde(default)]
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabasesListResponse {
    #[serde(default)]
    pub items: Option<Vec<CloudSqlDatabase>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSqlDatabase {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub charset: Option<String>,
    #[serde(default)]
    pub collation: Option<String>,
}

// ── CLI response wrappers ──

#[derive(Debug, Serialize)]
pub struct CloudSqlInstancesCliResponse {
    pub project: String,
    pub instances: Vec<CloudSqlInstance>,
}

#[derive(Debug, Serialize)]
pub struct CloudSqlInstanceGetCliResponse {
    pub project: String,
    pub instance: CloudSqlInstance,
}

#[derive(Debug, Serialize)]
pub struct CloudSqlDatabasesCliResponse {
    pub project: String,
    pub instance: String,
    pub databases: Vec<CloudSqlDatabase>,
}
