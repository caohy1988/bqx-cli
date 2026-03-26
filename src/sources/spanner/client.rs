use anyhow::{bail, Result};
use async_trait::async_trait;

use super::models::*;

#[async_trait]
pub trait SpannerClient: Send + Sync {
    async fn list_instances(&self, project: &str) -> Result<Vec<SpannerInstance>>;
    async fn list_databases(&self, project: &str, instance: &str) -> Result<Vec<SpannerDatabase>>;
}

pub struct HttpSpannerClient {
    http: reqwest::Client,
    token: String,
    base_url: String,
}

impl HttpSpannerClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
            base_url: "https://spanner.googleapis.com/v1".to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(token: String, base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
            base_url,
        }
    }
}

#[async_trait]
impl SpannerClient for HttpSpannerClient {
    async fn list_instances(&self, project: &str) -> Result<Vec<SpannerInstance>> {
        let url = format!("{}/projects/{project}/instances", self.base_url);
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.http.get(&url).bearer_auth(&self.token);
            req = req.query(&[("pageSize", "100")]);
            if let Some(ref token) = page_token {
                req = req.query(&[("pageToken", token)]);
            }

            let resp = req.send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("Spanner API error (list instances): {status} — {body}");
            }

            let body: InstancesListResponse = resp.json().await?;
            if let Some(instances) = body.instances {
                all.extend(instances);
            }
            match body.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }

        Ok(all)
    }

    async fn list_databases(&self, project: &str, instance: &str) -> Result<Vec<SpannerDatabase>> {
        let url = format!(
            "{}/projects/{project}/instances/{instance}/databases",
            self.base_url
        );
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.http.get(&url).bearer_auth(&self.token);
            req = req.query(&[("pageSize", "100")]);
            if let Some(ref token) = page_token {
                req = req.query(&[("pageToken", token)]);
            }

            let resp = req.send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("Spanner API error (list databases): {status} — {body}");
            }

            let body: DatabasesListResponse = resp.json().await?;
            if let Some(databases) = body.databases {
                all.extend(databases);
            }
            match body.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }

        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_instances_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/test-proj/instances"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "instances": [
                    {
                        "name": "projects/test-proj/instances/prod",
                        "displayName": "Production",
                        "config": "projects/test-proj/instanceConfigs/regional-us-central1",
                        "nodeCount": 3,
                        "state": "READY"
                    },
                    {
                        "name": "projects/test-proj/instances/dev",
                        "displayName": "Development",
                        "processingUnits": 100,
                        "state": "READY"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = HttpSpannerClient::with_base_url("test-token".into(), server.uri());
        let instances = client.list_instances("test-proj").await.unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].name, "projects/test-proj/instances/prod");
        assert_eq!(instances[0].display_name.as_deref(), Some("Production"));
        assert_eq!(instances[0].node_count, Some(3));
        assert_eq!(instances[0].state.as_deref(), Some("READY"));
        assert_eq!(instances[1].processing_units, Some(100));
    }

    #[tokio::test]
    async fn list_instances_paginates() {
        let server = MockServer::start().await;

        // Page 1: return one instance + nextPageToken
        Mock::given(method("GET"))
            .and(path("/projects/proj/instances"))
            .and(query_param("pageSize", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "instances": [{"name": "projects/proj/instances/first", "state": "READY"}],
                "nextPageToken": "tok2"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Page 2: return second instance, no nextPageToken
        Mock::given(method("GET"))
            .and(path("/projects/proj/instances"))
            .and(query_param("pageToken", "tok2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "instances": [{"name": "projects/proj/instances/second", "state": "READY"}]
            })))
            .mount(&server)
            .await;

        let client = HttpSpannerClient::with_base_url("test-token".into(), server.uri());
        let instances = client.list_instances("proj").await.unwrap();

        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].name, "projects/proj/instances/first");
        assert_eq!(instances[1].name, "projects/proj/instances/second");
    }

    #[tokio::test]
    async fn list_databases_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/test-proj/instances/prod/databases"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "databases": [
                    {
                        "name": "projects/test-proj/instances/prod/databases/ledger",
                        "state": "READY",
                        "databaseDialect": "GOOGLE_STANDARD_SQL"
                    },
                    {
                        "name": "projects/test-proj/instances/prod/databases/analytics",
                        "state": "READY",
                        "databaseDialect": "POSTGRESQL"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = HttpSpannerClient::with_base_url("test-token".into(), server.uri());
        let databases = client.list_databases("test-proj", "prod").await.unwrap();

        assert_eq!(databases.len(), 2);
        assert_eq!(
            databases[0].name,
            "projects/test-proj/instances/prod/databases/ledger"
        );
        assert_eq!(
            databases[0].database_dialect.as_deref(),
            Some("GOOGLE_STANDARD_SQL")
        );
        assert_eq!(databases[1].database_dialect.as_deref(), Some("POSTGRESQL"));
    }

    #[tokio::test]
    async fn list_instances_handles_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/bad/instances"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let client = HttpSpannerClient::with_base_url("bad-token".into(), server.uri());
        let result = client.list_instances("bad").await;

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("403"), "Should contain status code: {msg}");
    }
}
