use serde_json::json;
use wiremock::matchers::{bearer_token, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use dcx::auth::resolver::{AuthSource, ResolvedAuth};
use dcx::bigquery::client::{BigQueryClient, QueryExecutor, QueryRequest};

fn static_auth(token: &str) -> ResolvedAuth {
    ResolvedAuth::static_token(AuthSource::ExplicitToken, token.to_string())
}

fn test_request() -> QueryRequest {
    QueryRequest {
        query: "SELECT 1".into(),
        use_legacy_sql: false,
        location: "US".into(),
        max_results: None,
        timeout_ms: Some(30000),
    }
}

// ── Happy path ──

#[tokio::test]
async fn query_returns_rows_with_schema() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects/test-proj/queries"))
        .and(bearer_token("test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "schema": {
                "fields": [
                    {"name": "name", "type": "STRING"},
                    {"name": "count", "type": "INTEGER"}
                ]
            },
            "rows": [
                {"f": [{"v": "alice"}, {"v": "10"}]},
                {"f": [{"v": "bob"}, {"v": "20"}]}
            ],
            "totalRows": "2"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("test-token"), server.uri());
    let result = client.query("test-proj", test_request()).await.unwrap();

    assert_eq!(result.total_rows, 2);
    assert_eq!(result.schema.fields.len(), 2);
    assert_eq!(result.schema.fields[0].name, "name");
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].get("name").unwrap(), "alice");
    assert_eq!(result.rows[1].get("count").unwrap(), "20");
}

#[tokio::test]
async fn query_sends_bearer_token() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(bearer_token("my-secret-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "schema": {"fields": []},
            "rows": [],
            "totalRows": "0"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("my-secret-token"), server.uri());
    let result = client.query("proj", test_request()).await;
    assert!(result.is_ok());
}

// ── Error handling ──

#[tokio::test]
async fn query_surfaces_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": {
                "code": 403,
                "message": "Access denied: Dataset test-proj:my_dataset"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let err = client.query("proj", test_request()).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("403"), "Expected 403 in: {msg}");
    assert!(
        msg.contains("Access denied"),
        "Expected 'Access denied' in: {msg}"
    );
}

#[tokio::test]
async fn query_handles_404_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": {
                "code": 404,
                "message": "Not found: Table test-proj:ds.tbl"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let err = client.query("proj", test_request()).await.unwrap_err();
    assert!(err.to_string().contains("404"));
}

#[tokio::test]
async fn query_handles_malformed_error_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let err = client.query("proj", test_request()).await.unwrap_err();
    // Should still produce a useful error (falls back to status code)
    assert!(err.to_string().contains("500"));
}

// ── TIMESTAMP coercion ──

#[tokio::test]
async fn timestamp_values_are_coerced_to_iso8601() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "schema": {
                "fields": [
                    {"name": "ts", "type": "TIMESTAMP"},
                    {"name": "name", "type": "STRING"}
                ]
            },
            "rows": [
                {"f": [{"v": "1709640000.0"}, {"v": "alice"}]}
            ],
            "totalRows": "1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let result = client.query("proj", test_request()).await.unwrap();

    let ts = result.rows[0].get("ts").unwrap().as_str().unwrap();
    assert!(ts.contains("2024-03-05"), "Expected ISO date in: {ts}");
    assert!(ts.contains("UTC"), "Expected UTC in: {ts}");
}

// ── Empty result ──

#[tokio::test]
async fn query_handles_empty_result() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "schema": {
                "fields": [{"name": "id", "type": "STRING"}]
            },
            "totalRows": "0"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let result = client.query("proj", test_request()).await.unwrap();
    assert_eq!(result.total_rows, 0);
    assert!(result.rows.is_empty());
}

// ── Polling (jobComplete=false) ──

#[tokio::test]
async fn query_polls_when_job_not_complete() {
    let server = MockServer::start().await;

    // Initial POST returns jobComplete=false with a jobReference
    Mock::given(method("POST"))
        .and(path("/projects/proj/queries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": false,
            "jobReference": {
                "jobId": "job-123",
                "projectId": "proj",
                "location": "US"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Poll GET returns jobComplete=true with results
    Mock::given(method("GET"))
        .and(path("/projects/proj/queries/job-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "jobReference": {
                "jobId": "job-123",
                "projectId": "proj"
            },
            "schema": {
                "fields": [
                    {"name": "id", "type": "STRING"},
                    {"name": "val", "type": "INTEGER"}
                ]
            },
            "rows": [
                {"f": [{"v": "row1"}, {"v": "100"}]}
            ],
            "totalRows": "1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let result = client.query("proj", test_request()).await.unwrap();

    assert_eq!(result.total_rows, 1);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].get("id").unwrap(), "row1");
    assert_eq!(result.rows[0].get("val").unwrap(), "100");
}

// ── Pagination (pageToken) ──

#[tokio::test]
async fn query_paginates_with_page_token() {
    let server = MockServer::start().await;

    // Initial POST returns first page with a pageToken
    Mock::given(method("POST"))
        .and(path("/projects/proj/queries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "jobReference": {
                "jobId": "job-456",
                "projectId": "proj"
            },
            "schema": {
                "fields": [{"name": "name", "type": "STRING"}]
            },
            "rows": [
                {"f": [{"v": "page1-row1"}]},
                {"f": [{"v": "page1-row2"}]}
            ],
            "totalRows": "4",
            "pageToken": "next-page-token-abc"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // GET for page 2 — returns remaining rows, no more pageToken
    Mock::given(method("GET"))
        .and(path("/projects/proj/queries/job-456"))
        .and(query_param("pageToken", "next-page-token-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jobComplete": true,
            "rows": [
                {"f": [{"v": "page2-row1"}]},
                {"f": [{"v": "page2-row2"}]}
            ],
            "totalRows": "4"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = BigQueryClient::with_base_url(static_auth("tok"), server.uri());
    let result = client.query("proj", test_request()).await.unwrap();

    assert_eq!(result.total_rows, 4);
    assert_eq!(result.rows.len(), 4);
    assert_eq!(result.rows[0].get("name").unwrap(), "page1-row1");
    assert_eq!(result.rows[1].get("name").unwrap(), "page1-row2");
    assert_eq!(result.rows[2].get("name").unwrap(), "page2-row1");
    assert_eq!(result.rows[3].get("name").unwrap(), "page2-row2");
}
