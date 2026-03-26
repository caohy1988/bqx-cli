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
}

impl HttpAlloyDbClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }

    fn base() -> &'static str {
        "https://alloydb.googleapis.com/v1"
    }
}

#[async_trait]
impl AlloyDbClient for HttpAlloyDbClient {
    async fn list_clusters(&self, project: &str, location: &str) -> Result<Vec<AlloyDbCluster>> {
        let url = format!(
            "{}/projects/{project}/locations/{location}/clusters",
            Self::base()
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
            Self::base()
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_clusters_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/projects/proj/locations/-/clusters"))
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

        let url = format!("{}/v1/projects/proj/locations/-/clusters", server.uri());
        let client = HttpAlloyDbClient::new("test-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .query(&[("pageSize", "100")])
            .send()
            .await
            .unwrap();
        let body: ClustersListResponse = resp.json().await.unwrap();
        let clusters = body.clusters.unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].display_name.as_deref(), Some("Production"));
        assert_eq!(clusters[0].state.as_deref(), Some("READY"));
        assert_eq!(clusters[0].database_version.as_deref(), Some("POSTGRES_15"));
    }

    #[tokio::test]
    async fn list_instances_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/v1/projects/proj/locations/us-central1/clusters/prod/instances",
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

        let url = format!(
            "{}/v1/projects/proj/locations/us-central1/clusters/prod/instances",
            server.uri()
        );
        let client = HttpAlloyDbClient::new("test-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .query(&[("pageSize", "100")])
            .send()
            .await
            .unwrap();
        let body: InstancesListResponse = resp.json().await.unwrap();
        let instances = body.instances.unwrap();

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
            .and(path("/v1/projects/bad/locations/-/clusters"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let url = format!("{}/v1/projects/bad/locations/-/clusters", server.uri());
        let client = HttpAlloyDbClient::new("bad-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 403);
    }
}
