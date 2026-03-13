use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use super::super::discovery::DiscoveryDocument;

/// Allowlisted BigQuery methods for Phase 2.
pub const ALLOWED_METHODS: &[&str] = &[
    "bigquery.datasets.list",
    "bigquery.datasets.get",
    "bigquery.tables.list",
    "bigquery.tables.get",
];

/// A parsed BigQuery API method, extracted from Discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMethod {
    /// Discovery method ID, e.g. "bigquery.datasets.list"
    pub id: String,
    /// Resource family, e.g. "datasets"
    pub resource: String,
    /// Action verb, e.g. "list"
    pub action: String,
    /// URL path template, e.g. "projects/{+projectId}/datasets"
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

/// Extract all ApiMethods from a DiscoveryDocument by walking its resources.
/// Methods that cannot be parsed (unexpected shape, missing fields, etc.)
/// are skipped rather than aborting the entire document, so that unrelated
/// upstream Discovery changes do not break the allowlisted command set.
pub fn extract_methods(doc: &DiscoveryDocument) -> Vec<ApiMethod> {
    let mut methods = Vec::new();
    for (resource_name, resource_value) in &doc.resources {
        let resource_methods = resource_value.get("methods").and_then(|m| m.as_object());
        if let Some(method_map) = resource_methods {
            for (_action_name, method_value) in method_map {
                match parse_method(resource_name, method_value) {
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
    }
    methods.sort_by(|a, b| a.id.cmp(&b.id));
    methods
}

/// Filter methods to only those in the allowlist.
pub fn filter_allowed(methods: &[ApiMethod]) -> Vec<ApiMethod> {
    let mut result: Vec<ApiMethod> = methods
        .iter()
        .filter(|m| ALLOWED_METHODS.contains(&m.id.as_str()))
        .cloned()
        .collect();
    // Return in allowlist order for determinism.
    result.sort_by_key(|m| {
        ALLOWED_METHODS
            .iter()
            .position(|&a| a == m.id)
            .unwrap_or(usize::MAX)
    });
    result
}

fn parse_method(resource_name: &str, value: &serde_json::Value) -> Result<ApiMethod> {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let (resource, action) = match normalize_method_id(&id) {
        Some(pair) => pair,
        None => bail!("Cannot normalize method ID: {id}"),
    };

    // Sanity check that resource matches the parent key.
    if resource != resource_name {
        bail!(
            "Method {id}: parsed resource '{resource}' does not match parent resource '{resource_name}'"
        );
    }

    let path = value
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

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

    let parameters = parse_params(value.get("parameters"))?;

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

/// Normalize BigQuery method ID to (resource, action).
/// "bigquery.datasets.list" → ("datasets", "list")
pub fn normalize_method_id(id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    if parts[0] != "bigquery" {
        return None;
    }
    Some((parts[1].to_string(), parts[2].to_string()))
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
        .map(|p| GeneratedArgument {
            flag_name: to_kebab_case(&p.name),
            api_name: p.name.clone(),
            required: p.required,
            help: p.description.clone(),
            value_type: discovery_type_to_arg_type(&p.param_type),
        })
        .collect();

    GeneratedCommand {
        group: method.resource.clone(),
        action: method.action.clone(),
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
        assert_eq!(
            normalize_method_id("bigquery.jobs.get"),
            Some(("jobs".into(), "get".into()))
        );
    }

    #[test]
    fn normalize_method_id_rejects_malformed() {
        assert_eq!(normalize_method_id("datasets.list"), None);
        assert_eq!(normalize_method_id(""), None);
        assert_eq!(normalize_method_id("bigquery.datasets"), None);
        assert_eq!(normalize_method_id("bigquery.datasets.list.extra"), None);
    }

    #[test]
    fn normalize_method_id_rejects_wrong_service() {
        assert_eq!(normalize_method_id("storage.buckets.list"), None);
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
}
