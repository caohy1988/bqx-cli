use serde::{Deserialize, Serialize};

// ── Explores ──

/// Summary of a LookML explore as returned by the Looker API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreSummary {
    pub model_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: Option<bool>,
}

/// Detailed explore metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreDetail {
    pub model_name: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub fields: Option<ExploreFields>,
}

/// Dimension and measure fields within an explore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreFields {
    #[serde(default)]
    pub dimensions: Option<Vec<FieldSummary>>,
    #[serde(default)]
    pub measures: Option<Vec<FieldSummary>>,
}

/// A single field (dimension or measure) within an explore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSummary {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type", default)]
    pub field_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

// ── LookML Models ──

/// Summary of a LookML model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookmlModelSummary {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub explores: Option<Vec<ExploreSummary>>,
}

// ── Dashboards ──

/// Summary of a Looker dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub folder: Option<FolderRef>,
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub readonly: Option<bool>,
}

/// Dashboard detail with element information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardDetail {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub folder: Option<FolderRef>,
    #[serde(default)]
    pub hidden: Option<bool>,
    #[serde(default)]
    pub readonly: Option<bool>,
    #[serde(default)]
    pub dashboard_elements: Option<Vec<DashboardElement>>,
}

/// Reference to a Looker folder/space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRef {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// A single dashboard element (tile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardElement {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "type", default)]
    pub element_type: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub explore: Option<String>,
}

// ── CLI Response Types ──

/// CLI response for explores list.
#[derive(Debug, Serialize)]
pub struct ExploresListResponse {
    pub instance_url: String,
    pub explores: Vec<ExploreSummary>,
}

/// CLI response for explore get.
#[derive(Debug, Serialize)]
pub struct ExploreGetResponse {
    pub instance_url: String,
    pub explore: ExploreDetail,
}

/// CLI response for dashboards list.
#[derive(Debug, Serialize)]
pub struct DashboardsListResponse {
    pub instance_url: String,
    pub dashboards: Vec<DashboardSummary>,
}

/// CLI response for dashboard get.
#[derive(Debug, Serialize)]
pub struct DashboardGetResponse {
    pub instance_url: String,
    pub dashboard: DashboardDetail,
}
