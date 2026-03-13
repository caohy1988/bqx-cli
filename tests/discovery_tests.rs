use bqx::bigquery::discovery::{self, DiscoverySource};
use bqx::bigquery::dynamic::model::{
    extract_methods, filter_allowed, to_generated_command, ArgValueType, ParamLocation,
    ALLOWED_METHODS,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_mini_fixture() -> discovery::DiscoveryDocument {
    let raw = include_str!("fixtures/discovery/bigquery_v2_mini.json");
    serde_json::from_str(raw).expect("mini fixture should parse")
}

fn load_bundled() -> discovery::DiscoveryDocument {
    discovery::load(&DiscoverySource::Bundled).expect("bundled discovery should load")
}

// ---------------------------------------------------------------------------
// Loading tests
// ---------------------------------------------------------------------------

#[test]
fn bundled_discovery_loads_without_network() {
    let doc = load_bundled();
    assert_eq!(doc.name, "bigquery");
    assert_eq!(doc.version, "v2");
}

#[test]
fn bundled_discovery_has_expected_revision() {
    let doc = load_bundled();
    assert!(!doc.revision.is_empty(), "revision should be non-empty");
}

#[test]
fn bundled_discovery_base_url() {
    let doc = load_bundled();
    assert!(
        doc.base_url.contains("bigquery.googleapis.com"),
        "base_url should reference BigQuery: {}",
        doc.base_url
    );
}

// ---------------------------------------------------------------------------
// Parsing tests — mini fixture
// ---------------------------------------------------------------------------

#[test]
fn extract_methods_from_mini_fixture() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    assert_eq!(methods.len(), 3);

    let ids: Vec<&str> = methods.iter().map(|m| m.id.as_str()).collect();
    assert!(ids.contains(&"bigquery.datasets.list"));
    assert!(ids.contains(&"bigquery.datasets.get"));
    assert!(ids.contains(&"bigquery.tables.get"));
}

#[test]
fn mini_datasets_list_has_correct_params() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    let ds_list = methods
        .iter()
        .find(|m| m.id == "bigquery.datasets.list")
        .unwrap();

    assert_eq!(ds_list.http_method, "GET");
    assert_eq!(ds_list.path, "projects/{+projectId}/datasets");
    assert_eq!(ds_list.resource, "datasets");
    assert_eq!(ds_list.action, "list");
    assert!(ds_list.request_ref.is_none());
    assert_eq!(ds_list.response_ref.as_deref(), Some("DatasetList"));

    // Check projectId is path, required
    let project_param = ds_list
        .parameters
        .iter()
        .find(|p| p.name == "projectId")
        .unwrap();
    assert_eq!(project_param.location, ParamLocation::Path);
    assert!(project_param.required);
    assert_eq!(project_param.param_type, "string");

    // Check maxResults is query, optional, integer
    let max_param = ds_list
        .parameters
        .iter()
        .find(|p| p.name == "maxResults")
        .unwrap();
    assert_eq!(max_param.location, ParamLocation::Query);
    assert!(!max_param.required);
    assert_eq!(max_param.param_type, "integer");
    assert_eq!(max_param.format.as_deref(), Some("uint32"));

    // Check all is query, optional, boolean
    let all_param = ds_list.parameters.iter().find(|p| p.name == "all").unwrap();
    assert_eq!(all_param.location, ParamLocation::Query);
    assert!(!all_param.required);
    assert_eq!(all_param.param_type, "boolean");
}

#[test]
fn mini_tables_get_has_three_path_params() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    let tbl_get = methods
        .iter()
        .find(|m| m.id == "bigquery.tables.get")
        .unwrap();

    let path_params: Vec<&str> = tbl_get
        .parameters
        .iter()
        .filter(|p| p.location == ParamLocation::Path)
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(path_params, vec!["datasetId", "projectId", "tableId"]);

    for p in &tbl_get.parameters {
        if p.location == ParamLocation::Path {
            assert!(p.required, "{} should be required", p.name);
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing tests — bundled (full document)
// ---------------------------------------------------------------------------

#[test]
fn extract_methods_from_bundled_returns_nonempty() {
    let doc = load_bundled();
    let methods = extract_methods(&doc);
    assert!(
        methods.len() > 30,
        "expected >30 methods from full discovery, got {}",
        methods.len()
    );
}

#[test]
fn bundled_contains_all_allowed_methods() {
    let doc = load_bundled();
    let methods = extract_methods(&doc);
    let ids: Vec<&str> = methods.iter().map(|m| m.id.as_str()).collect();

    for allowed in ALLOWED_METHODS {
        assert!(
            ids.contains(allowed),
            "bundled discovery missing allowlisted method: {allowed}"
        );
    }
}

// ---------------------------------------------------------------------------
// Allowlist filtering
// ---------------------------------------------------------------------------

#[test]
fn filter_allowed_returns_expected_count() {
    let doc = load_bundled();
    let methods = extract_methods(&doc);
    let filtered = filter_allowed(&methods);
    assert_eq!(filtered.len(), 8);
}

#[test]
fn filter_allowed_preserves_allowlist_order() {
    let doc = load_bundled();
    let methods = extract_methods(&doc);
    let filtered = filter_allowed(&methods);
    let ids: Vec<&str> = filtered.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ALLOWED_METHODS);
}

#[test]
fn filter_allowed_on_mini_returns_two() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    let filtered = filter_allowed(&methods);
    // Mini has datasets.list, datasets.get, tables.get — all three are in allowlist
    assert_eq!(filtered.len(), 3);
    let ids: Vec<&str> = filtered.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "bigquery.datasets.list",
            "bigquery.datasets.get",
            "bigquery.tables.get",
        ]
    );
}

// ---------------------------------------------------------------------------
// GeneratedCommand tests
// ---------------------------------------------------------------------------

#[test]
fn generated_command_shape() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    let ds_list = methods
        .iter()
        .find(|m| m.id == "bigquery.datasets.list")
        .unwrap();
    let cmd = to_generated_command(ds_list);

    assert_eq!(cmd.group, "datasets");
    assert_eq!(cmd.action, "list");
    assert!(!cmd.about.is_empty());

    // Check args
    let project_arg = cmd.args.iter().find(|a| a.api_name == "projectId").unwrap();
    assert_eq!(project_arg.flag_name, "project-id");
    assert!(project_arg.required);
    assert_eq!(project_arg.value_type, ArgValueType::String);

    let max_arg = cmd
        .args
        .iter()
        .find(|a| a.api_name == "maxResults")
        .unwrap();
    assert_eq!(max_arg.flag_name, "max-results");
    assert!(!max_arg.required);
    assert_eq!(max_arg.value_type, ArgValueType::Integer);

    let all_arg = cmd.args.iter().find(|a| a.api_name == "all").unwrap();
    assert_eq!(all_arg.flag_name, "all");
    assert!(!all_arg.required);
    assert_eq!(all_arg.value_type, ArgValueType::Boolean);
}

#[test]
fn generated_command_snapshot_datasets_list() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    let ds_list = methods
        .iter()
        .find(|m| m.id == "bigquery.datasets.list")
        .unwrap();
    let cmd = to_generated_command(ds_list);
    insta::assert_json_snapshot!("generated_command_datasets_list", cmd);
}

#[test]
fn generated_command_snapshot_tables_get() {
    let doc = load_mini_fixture();
    let methods = extract_methods(&doc);
    let tbl_get = methods
        .iter()
        .find(|m| m.id == "bigquery.tables.get")
        .unwrap();
    let cmd = to_generated_command(tbl_get);
    insta::assert_json_snapshot!("generated_command_tables_get", cmd);
}

// ---------------------------------------------------------------------------
// Snapshot: all allowed methods from bundled
// ---------------------------------------------------------------------------

#[test]
fn allowed_methods_snapshot() {
    let doc = load_bundled();
    let methods = extract_methods(&doc);
    let filtered = filter_allowed(&methods);
    let summary: Vec<serde_json::Value> = filtered
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "resource": m.resource,
                "action": m.action,
                "http_method": m.http_method,
                "path": m.path,
                "param_count": m.parameters.len(),
                "request_ref": m.request_ref,
                "response_ref": m.response_ref,
            })
        })
        .collect();
    insta::assert_json_snapshot!("allowed_methods_summary", summary);
}
