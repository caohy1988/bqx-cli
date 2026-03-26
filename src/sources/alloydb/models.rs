use serde::{Deserialize, Serialize};

// ── API response types ──

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClustersListResponse {
    #[serde(default)]
    pub clusters: Option<Vec<AlloyDbCluster>>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlloyDbCluster {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub database_version: Option<String>,
    #[serde(default)]
    pub cluster_type: Option<String>,
    #[serde(default)]
    pub network: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancesListResponse {
    #[serde(default)]
    pub instances: Option<Vec<AlloyDbInstance>>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlloyDbInstance {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub instance_type: Option<String>,
    #[serde(default)]
    pub machine_config: Option<MachineConfig>,
    #[serde(default)]
    pub gce_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineConfig {
    #[serde(default)]
    pub cpu_count: Option<i32>,
}

// ── CLI response wrappers ──

#[derive(Debug, Serialize)]
pub struct AlloyDbClustersCliResponse {
    pub project: String,
    pub location: String,
    pub clusters: Vec<AlloyDbCluster>,
}

#[derive(Debug, Serialize)]
pub struct AlloyDbInstancesCliResponse {
    pub project: String,
    pub location: String,
    pub cluster: String,
    pub instances: Vec<AlloyDbInstance>,
}
