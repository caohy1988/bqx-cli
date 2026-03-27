use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::super::discovery::DiscoveryDocument;

/// A parsed API method, extracted from a Discovery document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMethod {
    /// Discovery method ID, e.g. "bigquery.datasets.list"
    pub id: String,
    /// Leaf resource family, e.g. "datasets"
    pub resource: String,
    /// Action verb, e.g. "list"
    pub action: String,
    /// URL path template (may be `path` or `flatPath`).
    pub path: String,
    /// HTTP method, e.g. "GET"
    pub http_method: String,
    /// Human-readable description from Discovery
    pub description: String,
    /// Parameters (path + query)
    pub parameters: Vec<ApiParam>,
    /// Request body schema ref, if any
    pub request_ref: Option<String>,
    /// Response schema ref, if any
    pub response_ref: Option<String>,
}

/// A parameter on an API method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParam {
    /// Parameter name, e.g. "projectId"
    pub name: String,
    /// Where the param goes: "path" or "query"
    pub location: ParamLocation,
    /// Discovery type: "string", "integer", "boolean"
    pub param_type: String,
    /// Optional format hint: "uint32", "int64", etc.
    pub format: Option<String>,
    /// Whether the parameter is required
    pub required: bool,
    /// Human description from Discovery
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParamLocation {
    Path,
    Query,
}

/// CLI-ready command metadata derived from ApiMethod.
/// No clap dependency — pure data.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedCommand {
    /// CLI subcommand group, e.g. "datasets"
    pub group: String,
    /// CLI subcommand action, e.g. "list"
    pub action: String,
    /// Help text
    pub about: String,
    /// CLI arguments
    pub args: Vec<GeneratedArgument>,
    /// Original method for request building
    pub method: ApiMethod,
}

/// A CLI argument derived from an ApiParam.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedArgument {
    /// CLI flag name (kebab-case), e.g. "project-id"
    pub flag_name: String,
    /// Original API parameter name, e.g. "projectId"
    pub api_name: String,
    /// Whether this is required
    pub required: bool,
    /// Help text
    pub help: String,
    /// Value type hint for clap
    pub value_type: ArgValueType,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ArgValueType {
    String,
    Integer,
    Boolean,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Extract all ApiMethods from a DiscoveryDocument by recursively walking its
/// resources. Methods that cannot be parsed are skipped.
///
/// `use_flat_path`: when true, prefer `flatPath` over `path` and extract
/// path parameters from the template (needed for Spanner/AlloyDB).
pub fn extract_methods(doc: &DiscoveryDocument, use_flat_path: bool) -> Vec<ApiMethod> {
    let mut methods = Vec::new();
    extract_methods_recursive(&doc.resources, use_flat_path, &mut methods);
    methods.sort_by(|a, b| a.id.cmp(&b.id));
    methods
}

fn extract_methods_recursive(
    resources: &serde_json::Map<String, serde_json::Value>,
    use_flat_path: bool,
    methods: &mut Vec<ApiMethod>,
) {
    for (resource_name, resource_value) in resources {
        if let Some(method_map) = resource_value.get("methods").and_then(|m| m.as_object()) {
            for (_action_name, method_value) in method_map {
                match parse_method(resource_name, method_value, use_flat_path) {
                    Ok(method) => methods.push(method),
                    Err(e) => {
                        let id = method_value
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("<unknown>");
                        eprintln!("Warning: skipping unparseable method {id}: {e}");
                    }
                }
            }
        }
        // Recurse into nested resources.
        if let Some(sub_resources) = resource_value.get("resources").and_then(|r| r.as_object()) {
            extract_methods_recursive(sub_resources, use_flat_path, methods);
        }
    }
}

/// Filter methods to only those in the given allowlist.
pub fn filter_allowed(methods: &[ApiMethod], allowed: &[&str]) -> Vec<ApiMethod> {
    let mut result: Vec<ApiMethod> = methods
        .iter()
        .filter(|m| allowed.contains(&m.id.as_str()))
        .cloned()
        .collect();
    // Return in allowlist order for determinism.
    result.sort_by_key(|m| {
        allowed
            .iter()
            .position(|&a| a == m.id)
            .unwrap_or(usize::MAX)
    });
    result
}

fn parse_method(
    resource_name: &str,
    value: &serde_json::Value,
    use_flat_path: bool,
) -> Result<ApiMethod> {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let (resource, action) = match normalize_method_id(&id) {
        Some(pair) => pair,
        None => bail!("Cannot normalize method ID: {id}"),
    };

    // Sanity check: the parsed resource should match the parent resource key.
    if resource != resource_name {
        bail!("Method {id}: parsed resource '{resource}' does not match parent '{resource_name}'");
    }

    // Choose path template: prefer flatPath when configured.
    let path = if use_flat_path {
        value
            .get("flatPath")
            .or_else(|| value.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    } else {
        value
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    let http_method = value
        .get("httpMethod")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_string();

    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    // Parse parameters: from flatPath template if using flat_path, otherwise from Discovery params.
    let parameters = if use_flat_path && value.get("flatPath").is_some() {
        parse_flat_path_params(
            value
                .get("flatPath")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            value.get("parameters"),
        )?
    } else {
        parse_params(value.get("parameters"))?
    };

    let request_ref = value
        .get("request")
        .and_then(|v| v.get("$ref"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let response_ref = value
        .get("response")
        .and_then(|v| v.get("$ref"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(ApiMethod {
        id,
        resource,
        action,
        path,
        http_method,
        description,
        parameters,
        request_ref,
        response_ref,
    })
}

/// Parse parameters from a flatPath template + Discovery parameter definitions.
///
/// Path parameters are extracted from the `{paramName}` placeholders in the
/// flatPath template. Query parameters come from the Discovery `parameters` object.
fn parse_flat_path_params(
    flat_path: &str,
    params_value: Option<&serde_json::Value>,
) -> Result<Vec<ApiParam>> {
    let mut params = Vec::new();

    // Extract path params from flatPath template (e.g., {projectsId}, {instancesId}).
    let re = Regex::new(r"\{(\w+)\}").unwrap();
    for cap in re.captures_iter(flat_path) {
        let name = cap[1].to_string();
        params.push(ApiParam {
            name,
            location: ParamLocation::Path,
            param_type: "string".to_string(),
            format: None,
            required: true,
            description: String::new(),
        });
    }

    // Add query parameters from Discovery parameters object.
    if let Some(obj) = params_value.and_then(|v| v.as_object()) {
        for (name, val) in obj {
            let location_str = val
                .get("location")
                .and_then(|v| v.as_str())
                .unwrap_or("query");
            if location_str == "path" {
                // Path params already extracted from flatPath.
                continue;
            }
            params.push(ApiParam {
                name: name.clone(),
                location: ParamLocation::Query,
                param_type: val
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("string")
                    .to_string(),
                format: val
                    .get("format")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                required: val
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                description: val
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }

    // Sort: path params first (alphabetically), then query params (alphabetically).
    params.sort_by(|a, b| {
        let loc_ord = |p: &ApiParam| match p.location {
            ParamLocation::Path => 0,
            ParamLocation::Query => 1,
        };
        loc_ord(a)
            .cmp(&loc_ord(b))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(params)
}

fn parse_params(params_value: Option<&serde_json::Value>) -> Result<Vec<ApiParam>> {
    let Some(obj) = params_value.and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };

    let mut params: Vec<ApiParam> = obj
        .iter()
        .map(|(name, val)| {
            let location_str = val
                .get("location")
                .and_then(|v| v.as_str())
                .unwrap_or("query");
            let location = if location_str == "path" {
                ParamLocation::Path
            } else {
                ParamLocation::Query
            };

            let param_type = val
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("string")
                .to_string();

            let format = val
                .get("format")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let required = val
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let description = val
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            ApiParam {
                name: name.clone(),
                location,
                param_type,
                format,
                required,
                description,
            }
        })
        .collect();

    // Sort: path params first (alphabetically), then query params (alphabetically).
    params.sort_by(|a, b| {
        let loc_ord = |p: &ApiParam| match p.location {
            ParamLocation::Path => 0,
            ParamLocation::Query => 1,
        };
        loc_ord(a)
            .cmp(&loc_ord(b))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(params)
}

// ---------------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------------

/// Normalize a Discovery method ID to (resource, action).
///
/// Works for any service with any number of dot-separated parts:
///   "bigquery.datasets.list"                      → ("datasets", "list")
///   "spanner.projects.instances.list"              → ("instances", "list")
///   "spanner.projects.instances.databases.list"    → ("databases", "list")
///   "sql.instances.list"                           → ("instances", "list")
pub fn normalize_method_id(id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    let resource = parts[parts.len() - 2].to_string();
    let action = parts[parts.len() - 1].to_string();
    Some((resource, action))
}

/// Normalize a flatPath parameter name for friendlier CLI flag names.
///
/// Strips the plural-form suffix: "projectsId" → "projectId",
/// "instancesId" → "instanceId". This produces nicer kebab-case flags
/// like `--instance-id` instead of `--instances-id`.
fn normalize_flat_param_name(name: &str) -> String {
    if name.ends_with("sId") && name.len() > 3 {
        format!("{}Id", &name[..name.len() - 3])
    } else {
        name.to_string()
    }
}

/// Convert camelCase API param name to kebab-case CLI flag.
/// "projectId" → "project-id"
/// "maxResults" → "max-results"
pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Map Discovery type string to ArgValueType.
fn discovery_type_to_arg_type(type_str: &str) -> ArgValueType {
    match type_str {
        "integer" => ArgValueType::Integer,
        "boolean" => ArgValueType::Boolean,
        _ => ArgValueType::String,
    }
}

// ---------------------------------------------------------------------------
// GeneratedCommand construction
// ---------------------------------------------------------------------------

/// Convert an ApiMethod into a GeneratedCommand (clap-independent).
pub fn to_generated_command(method: &ApiMethod) -> GeneratedCommand {
    let args: Vec<GeneratedArgument> = method
        .parameters
        .iter()
        .map(|p| {
            let cli_name = normalize_flat_param_name(&p.name);
            GeneratedArgument {
                flag_name: to_kebab_case(&cli_name),
                api_name: p.name.clone(),
                required: p.required,
                help: p.description.clone(),
                value_type: discovery_type_to_arg_type(&p.param_type),
            }
        })
        .collect();

    GeneratedCommand {
        group: method.resource.clone(),
        action: to_kebab_case(&method.action),
        about: method.description.clone(),
        args,
        method: method.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_method_id_standard() {
        assert_eq!(
            normalize_method_id("bigquery.datasets.list"),
            Some(("datasets".into(), "list".into()))
        );
        assert_eq!(
            normalize_method_id("bigquery.tables.get"),
            Some(("tables".into(), "get".into()))
        );
    }

    #[test]
    fn normalize_method_id_multi_part() {
        assert_eq!(
            normalize_method_id("spanner.projects.instances.list"),
            Some(("instances".into(), "list".into()))
        );
        assert_eq!(
            normalize_method_id("spanner.projects.instances.databases.list"),
            Some(("databases".into(), "list".into()))
        );
        assert_eq!(
            normalize_method_id("alloydb.projects.locations.clusters.list"),
            Some(("clusters".into(), "list".into()))
        );
        assert_eq!(
            normalize_method_id("sql.instances.list"),
            Some(("instances".into(), "list".into()))
        );
    }

    #[test]
    fn normalize_method_id_rejects_malformed() {
        assert_eq!(normalize_method_id("datasets.list"), None);
        assert_eq!(normalize_method_id(""), None);
        assert_eq!(normalize_method_id("bigquery.datasets"), None);
    }

    #[test]
    fn normalize_flat_param_name_strips_plural() {
        assert_eq!(normalize_flat_param_name("projectsId"), "projectId");
        assert_eq!(normalize_flat_param_name("instancesId"), "instanceId");
        assert_eq!(normalize_flat_param_name("clustersId"), "clusterId");
        assert_eq!(normalize_flat_param_name("locationsId"), "locationId");
    }

    #[test]
    fn normalize_flat_param_name_passthrough() {
        assert_eq!(normalize_flat_param_name("projectId"), "projectId");
        assert_eq!(normalize_flat_param_name("project"), "project");
        assert_eq!(normalize_flat_param_name("instance"), "instance");
    }

    #[test]
    fn to_kebab_case_conversions() {
        assert_eq!(to_kebab_case("projectId"), "project-id");
        assert_eq!(to_kebab_case("maxResults"), "max-results");
        assert_eq!(to_kebab_case("datasetId"), "dataset-id");
        assert_eq!(to_kebab_case("all"), "all");
        assert_eq!(to_kebab_case("useLegacySql"), "use-legacy-sql");
        assert_eq!(to_kebab_case("filter"), "filter");
        assert_eq!(to_kebab_case("pageToken"), "page-token");
    }

    #[test]
    fn discovery_type_mapping() {
        assert_eq!(discovery_type_to_arg_type("string"), ArgValueType::String);
        assert_eq!(discovery_type_to_arg_type("integer"), ArgValueType::Integer);
        assert_eq!(discovery_type_to_arg_type("boolean"), ArgValueType::Boolean);
        assert_eq!(discovery_type_to_arg_type("unknown"), ArgValueType::String);
    }

    #[test]
    fn param_location_serde_round_trip() {
        let path = ParamLocation::Path;
        let json = serde_json::to_string(&path).unwrap();
        assert_eq!(json, "\"path\"");
        let back: ParamLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ParamLocation::Path);

        let query = ParamLocation::Query;
        let json = serde_json::to_string(&query).unwrap();
        assert_eq!(json, "\"query\"");
        let back: ParamLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ParamLocation::Query);
    }

    #[test]
    fn extract_methods_bigquery_bundled() {
        use super::super::super::discovery::{self, DiscoverySource};
        let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
        let methods = extract_methods(&doc, false);
        // BigQuery has many methods; ensure datasets.list is present.
        assert!(methods.iter().any(|m| m.id == "bigquery.datasets.list"));
        assert!(methods.iter().any(|m| m.id == "bigquery.tables.list"));
    }

    #[test]
    fn extract_methods_spanner_bundled() {
        use crate::bigquery::dynamic::service;
        let cfg = service::spanner();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, true);
        assert!(methods
            .iter()
            .any(|m| m.id == "spanner.projects.instances.list"));
        assert!(methods
            .iter()
            .any(|m| m.id == "spanner.projects.instances.databases.list"));
    }

    #[test]
    fn filter_allowed_preserves_order() {
        use super::super::super::discovery::{self, DiscoverySource};
        let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
        let methods = extract_methods(&doc, false);
        let allowed = &["bigquery.tables.list", "bigquery.datasets.list"];
        let filtered = filter_allowed(&methods, allowed);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, "bigquery.tables.list");
        assert_eq!(filtered[1].id, "bigquery.datasets.list");
    }

    #[test]
    fn spanner_method_uses_flat_path() {
        use crate::bigquery::dynamic::service;
        let cfg = service::spanner();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, true);
        let inst_list = methods
            .iter()
            .find(|m| m.id == "spanner.projects.instances.list")
            .unwrap();
        // flatPath decomposes the `parent` param into `projectsId`.
        assert!(
            inst_list.path.contains("{projectsId}"),
            "Should use flatPath: {}",
            inst_list.path
        );
        assert!(inst_list.parameters.iter().any(|p| p.name == "projectsId"));
    }

    #[test]
    fn cloudsql_method_uses_path() {
        use crate::bigquery::dynamic::service;
        let cfg = service::cloudsql();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, false);
        let inst_list = methods
            .iter()
            .find(|m| m.id == "sql.instances.list")
            .unwrap();
        assert!(inst_list.parameters.iter().any(|p| p.name == "project"));
    }
}
