use anyhow::{bail, Result};
use async_trait::async_trait;

use super::models::*;

#[async_trait]
pub trait AlloyDbClient: Send + Sync {
    async fn list_clusters(&self, project: &str, location: &str) -> Result<Vec<AlloyDbCluster>>;
    async fn list_instances(
        &self,
        project: &str,
        location: &str,
        cluster: &str,
    ) -> Result<Vec<AlloyDbInstance>>;
}

pub struct HttpAlloyDbClient {
    http: reqwest::Client,
    token: String,
    base_url: String,
}

impl HttpAlloyDbClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
            base_url: "https://alloydb.googleapis.com/v1".to_string(),
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
impl AlloyDbClient for HttpAlloyDbClient {
    async fn list_clusters(&self, project: &str, location: &str) -> Result<Vec<AlloyDbCluster>> {
        let url = format!(
            "{}/projects/{project}/locations/{location}/clusters",
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
                bail!("AlloyDB API error (list clusters): {status} — {body}");
            }

            let body: ClustersListResponse = resp.json().await?;
            if let Some(clusters) = body.clusters {
                all.extend(clusters);
            }
            match body.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }

        Ok(all)
    }

    async fn list_instances(
        &self,
        project: &str,
        location: &str,
        cluster: &str,
    ) -> Result<Vec<AlloyDbInstance>> {
        let url = format!(
            "{}/projects/{project}/locations/{location}/clusters/{cluster}/instances",
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
                bail!("AlloyDB API error (list instances): {status} — {body}");
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_clusters_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/proj/locations/-/clusters"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clusters": [
                    {
                        "name": "projects/proj/locations/us-central1/clusters/prod",
                        "displayName": "Production",
                        "state": "READY",
                        "databaseVersion": "POSTGRES_15",
                        "clusterType": "PRIMARY"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = HttpAlloyDbClient::with_base_url("test-token".into(), server.uri());
        let clusters = client.list_clusters("proj", "-").await.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].display_name.as_deref(), Some("Production"));
        assert_eq!(clusters[0].state.as_deref(), Some("READY"));
        assert_eq!(clusters[0].database_version.as_deref(), Some("POSTGRES_15"));
    }

    #[tokio::test]
    async fn list_clusters_paginates() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/proj/locations/-/clusters"))
            .and(query_param("pageSize", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clusters": [{"name": "projects/proj/locations/us-central1/clusters/c1", "state": "READY"}],
                "nextPageToken": "page2"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/projects/proj/locations/-/clusters"))
            .and(query_param("pageToken", "page2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "clusters": [{"name": "projects/proj/locations/us-central1/clusters/c2", "state": "READY"}]
            })))
            .mount(&server)
            .await;

        let client = HttpAlloyDbClient::with_base_url("test-token".into(), server.uri());
        let clusters = client.list_clusters("proj", "-").await.unwrap();

        assert_eq!(clusters.len(), 2);
        assert!(clusters[0].name.ends_with("/c1"));
        assert!(clusters[1].name.ends_with("/c2"));
    }

    #[tokio::test]
    async fn list_instances_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/projects/proj/locations/us-central1/clusters/prod/instances",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "instances": [
                    {
                        "name": "projects/proj/locations/us-central1/clusters/prod/instances/primary",
                        "displayName": "Primary",
                        "state": "READY",
                        "instanceType": "PRIMARY",
                        "machineConfig": {"cpuCount": 4},
                        "gceZone": "us-central1-a"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = HttpAlloyDbClient::with_base_url("test-token".into(), server.uri());
        let instances = client
            .list_instances("proj", "us-central1", "prod")
            .await
            .unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_type.as_deref(), Some("PRIMARY"));
        assert_eq!(
            instances[0].machine_config.as_ref().unwrap().cpu_count,
            Some(4)
        );
        assert_eq!(instances[0].gce_zone.as_deref(), Some("us-central1-a"));
    }

    #[tokio::test]
    async fn list_clusters_handles_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/bad/locations/-/clusters"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let client = HttpAlloyDbClient::with_base_url("bad-token".into(), server.uri());
        let result = client.list_clusters("bad", "-").await;

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("403"), "Should contain status code: {msg}");
    }
}
