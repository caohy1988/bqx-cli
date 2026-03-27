use std::collections::HashMap;

use anyhow::{bail, Result};

use super::model::{ApiMethod, ParamLocation};

/// A fully-resolved HTTP request ready to execute.
#[derive(Debug, Clone)]
pub struct DynamicRequest {
    /// Full URL with path params substituted.
    pub url: String,
    /// HTTP method (GET, POST, etc.)
    pub http_method: String,
    /// Query parameters to append.
    pub query_params: Vec<(String, String)>,
}

/// Build a DynamicRequest from an ApiMethod and the user-supplied arguments.
///
/// - `base_url`: the Discovery document's baseUrl (e.g. "https://bigquery.googleapis.com/bigquery/v2/")
/// - `method`: the ApiMethod from the model
/// - `project_id`: the global --project-id value
/// - `args`: the matched argument values (API param names → values)
/// - `project_id_params`: API parameter names that should be filled from `project_id`
pub fn build_request(
    base_url: &str,
    method: &ApiMethod,
    project_id: &str,
    args: &HashMap<String, String>,
    project_id_params: &[&str],
) -> Result<DynamicRequest> {
    // Substitute path parameters in the URL template.
    let mut path = method.path.clone();

    for param in &method.parameters {
        if param.location != ParamLocation::Path {
            continue;
        }
        let placeholder_plus = format!("{{+{}}}", param.name);
        let placeholder_bare = format!("{{{}}}", param.name);

        let value = if project_id_params.contains(&param.name.as_str()) {
            project_id.to_string()
        } else if let Some(val) = args.get(&param.name) {
            val.clone()
        } else {
            bail!("Missing path parameter: {}", param.name);
        };

        if path.contains(&placeholder_plus) {
            path = path.replace(&placeholder_plus, &value);
        } else if path.contains(&placeholder_bare) {
            path = path.replace(&placeholder_bare, &value);
        } else {
            bail!(
                "Path template does not contain placeholder for parameter: {}",
                param.name
            );
        }
    }

    // Build query parameters from non-path args.
    let mut query_params = Vec::new();
    for param in &method.parameters {
        if param.location != ParamLocation::Query {
            continue;
        }
        if let Some(value) = args.get(&param.name) {
            query_params.push((param.name.clone(), value.clone()));
        }
    }

    // Construct full URL.
    let base = base_url.trim_end_matches('/');
    let url = format!("{base}/{path}");

    Ok(DynamicRequest {
        url,
        http_method: method.http_method.clone(),
        query_params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bigquery::discovery::{self, DiscoverySource};
    use crate::bigquery::dynamic::model::{extract_methods, filter_allowed};
    use crate::bigquery::dynamic::service;

    fn get_bq_method(method_id: &str) -> ApiMethod {
        let cfg = service::bigquery();
        let doc = discovery::load(&DiscoverySource::Bundled).unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        allowed
            .into_iter()
            .find(|m| m.id == method_id)
            .unwrap_or_else(|| panic!("Method {method_id} not found"))
    }

    #[test]
    fn datasets_list_url() {
        let method = get_bq_method("bigquery.datasets.list");
        let args = HashMap::new();
        let req = build_request(
            "https://bigquery.googleapis.com/bigquery/v2/",
            &method,
            "my-project",
            &args,
            &["projectId"],
        )
        .unwrap();
        assert_eq!(
            req.url,
            "https://bigquery.googleapis.com/bigquery/v2/projects/my-project/datasets"
        );
        assert_eq!(req.http_method, "GET");
        assert!(req.query_params.is_empty());
    }

    #[test]
    fn datasets_list_with_query_params() {
        let method = get_bq_method("bigquery.datasets.list");
        let mut args = HashMap::new();
        args.insert("maxResults".to_string(), "10".to_string());
        args.insert("all".to_string(), "true".to_string());
        let req = build_request(
            "https://bigquery.googleapis.com/bigquery/v2/",
            &method,
            "my-project",
            &args,
            &["projectId"],
        )
        .unwrap();
        assert_eq!(req.query_params.len(), 2);
        assert!(req
            .query_params
            .iter()
            .any(|(k, v)| k == "maxResults" && v == "10"));
        assert!(req
            .query_params
            .iter()
            .any(|(k, v)| k == "all" && v == "true"));
    }

    #[test]
    fn datasets_get_url() {
        let method = get_bq_method("bigquery.datasets.get");
        let mut args = HashMap::new();
        args.insert("datasetId".to_string(), "analytics".to_string());
        let req = build_request(
            "https://bigquery.googleapis.com/bigquery/v2/",
            &method,
            "my-project",
            &args,
            &["projectId"],
        )
        .unwrap();
        assert_eq!(
            req.url,
            "https://bigquery.googleapis.com/bigquery/v2/projects/my-project/datasets/analytics"
        );
    }

    #[test]
    fn tables_get_url() {
        let method = get_bq_method("bigquery.tables.get");
        let mut args = HashMap::new();
        args.insert("datasetId".to_string(), "analytics".to_string());
        args.insert("tableId".to_string(), "events".to_string());
        let req = build_request(
            "https://bigquery.googleapis.com/bigquery/v2/",
            &method,
            "my-project",
            &args,
            &["projectId"],
        )
        .unwrap();
        assert_eq!(
            req.url,
            "https://bigquery.googleapis.com/bigquery/v2/projects/my-project/datasets/analytics/tables/events"
        );
    }

    #[test]
    fn missing_path_param_errors() {
        let method = get_bq_method("bigquery.datasets.get");
        let args = HashMap::new(); // missing datasetId
        let result = build_request(
            "https://bigquery.googleapis.com/bigquery/v2/",
            &method,
            "my-project",
            &args,
            &["projectId"],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("datasetId"));
    }

    #[test]
    fn base_url_trailing_slash_normalization() {
        let method = get_bq_method("bigquery.datasets.list");
        let args = HashMap::new();

        let req1 = build_request(
            "https://bigquery.googleapis.com/bigquery/v2/",
            &method,
            "p",
            &args,
            &["projectId"],
        )
        .unwrap();
        let req2 = build_request(
            "https://bigquery.googleapis.com/bigquery/v2",
            &method,
            "p",
            &args,
            &["projectId"],
        )
        .unwrap();
        assert_eq!(req1.url, req2.url);
    }

    #[test]
    fn spanner_flat_path_url() {
        let cfg = service::spanner();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        let inst_list = allowed
            .iter()
            .find(|m| m.id == "spanner.projects.instances.list")
            .unwrap();

        let args = HashMap::new();
        let req = build_request(
            &doc.base_url,
            inst_list,
            "my-project",
            &args,
            &["projectsId"],
        )
        .unwrap();
        assert_eq!(
            req.url,
            "https://spanner.googleapis.com/v1/projects/my-project/instances"
        );
    }

    #[test]
    fn spanner_databases_url() {
        let cfg = service::spanner();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        let db_list = allowed
            .iter()
            .find(|m| m.id == "spanner.projects.instances.databases.list")
            .unwrap();

        let mut args = HashMap::new();
        args.insert("instancesId".to_string(), "my-instance".to_string());
        let req =
            build_request(&doc.base_url, db_list, "my-project", &args, &["projectsId"]).unwrap();
        assert_eq!(
            req.url,
            "https://spanner.googleapis.com/v1/projects/my-project/instances/my-instance/databases"
        );
    }

    #[test]
    fn cloudsql_instances_url() {
        let cfg = service::cloudsql();
        let doc = cfg.load_bundled().unwrap();
        let methods = extract_methods(&doc, cfg.use_flat_path);
        let allowed = filter_allowed(&methods, cfg.allowed_methods);
        let inst_list = allowed
            .iter()
            .find(|m| m.id == "sql.instances.list")
            .unwrap();

        let args = HashMap::new();
        let req =
            build_request(&doc.base_url, inst_list, "my-project", &args, &["project"]).unwrap();
        assert_eq!(
            req.url,
            "https://sqladmin.googleapis.com/v1/projects/my-project/instances"
        );
    }
}
