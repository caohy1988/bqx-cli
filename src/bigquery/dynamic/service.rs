use anyhow::{Context, Result};

use super::super::discovery::DiscoveryDocument;

/// Per-service configuration for discovery-driven dynamic command generation.
pub struct ServiceConfig {
    /// CLI namespace (e.g., "spanner", "cloudsql"). Empty = top-level (BigQuery).
    pub namespace: &'static str,
    /// Human-readable label for help text and error messages.
    pub service_label: &'static str,
    /// Discovery document `name` field (e.g., "bigquery", "spanner", "sqladmin").
    pub discovery_name: &'static str,
    /// Bundled discovery JSON (include_str!).
    pub bundled_json: &'static str,
    /// Allowlisted method IDs.
    pub allowed_methods: &'static [&'static str],
    /// Mapping from API path parameter name to the global CLI flag that supplies its value.
    /// E.g., `("projectId", "project_id")` means `--project-id` supplies `{projectId}`.
    pub global_params: &'static [(&'static str, &'static str)],
    /// Whether to prefer `flatPath` over `path` for URL templates.
    /// Spanner/AlloyDB need this; BigQuery/CloudSQL do not.
    pub use_flat_path: bool,
}

impl ServiceConfig {
    /// Load the bundled discovery document for this service.
    pub fn load_bundled(&self) -> Result<DiscoveryDocument> {
        serde_json::from_str(self.bundled_json)
            .with_context(|| format!("Failed to parse bundled {} discovery", self.service_label))
    }

    /// Names of API parameters that are provided by global CLI flags.
    pub fn global_param_names(&self) -> Vec<&'static str> {
        self.global_params.iter().map(|(api, _)| *api).collect()
    }
}

// ---------------------------------------------------------------------------
// Service definitions
// ---------------------------------------------------------------------------

pub fn bigquery() -> ServiceConfig {
    ServiceConfig {
        namespace: "",
        service_label: "BigQuery",
        discovery_name: "bigquery",
        bundled_json: include_str!("../../../assets/bigquery_v2_discovery.json"),
        allowed_methods: &[
            "bigquery.datasets.list",
            "bigquery.datasets.get",
            "bigquery.tables.list",
            "bigquery.tables.get",
            "bigquery.routines.list",
            "bigquery.routines.get",
            "bigquery.models.list",
            "bigquery.models.get",
        ],
        global_params: &[("projectId", "project_id"), ("datasetId", "dataset_id")],
        use_flat_path: false,
    }
}

pub fn spanner() -> ServiceConfig {
    ServiceConfig {
        namespace: "spanner",
        service_label: "Cloud Spanner",
        discovery_name: "spanner",
        bundled_json: include_str!("../../../assets/spanner_v1_discovery.json"),
        allowed_methods: &[
            "spanner.projects.instances.list",
            "spanner.projects.instances.get",
            "spanner.projects.instances.databases.list",
            "spanner.projects.instances.databases.get",
            "spanner.projects.instances.databases.getDdl",
        ],
        global_params: &[("projectsId", "project_id")],
        use_flat_path: true,
    }
}

pub fn alloydb() -> ServiceConfig {
    ServiceConfig {
        namespace: "alloydb",
        service_label: "AlloyDB",
        discovery_name: "alloydb",
        bundled_json: include_str!("../../../assets/alloydb_v1_discovery.json"),
        allowed_methods: &[
            "alloydb.projects.locations.clusters.list",
            "alloydb.projects.locations.clusters.get",
            "alloydb.projects.locations.clusters.instances.list",
            "alloydb.projects.locations.clusters.instances.get",
        ],
        global_params: &[("projectsId", "project_id")],
        use_flat_path: true,
    }
}

pub fn cloudsql() -> ServiceConfig {
    ServiceConfig {
        namespace: "cloudsql",
        service_label: "Cloud SQL",
        discovery_name: "sqladmin",
        bundled_json: include_str!("../../../assets/sqladmin_v1_discovery.json"),
        allowed_methods: &[
            "sql.instances.list",
            "sql.instances.get",
            "sql.databases.list",
            "sql.databases.get",
        ],
        global_params: &[("project", "project_id")],
        use_flat_path: false,
    }
}

/// Return configs for all supported services.
pub fn all_services() -> Vec<ServiceConfig> {
    vec![bigquery(), spanner(), alloydb(), cloudsql()]
}
