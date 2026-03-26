use anyhow::{bail, Result};
use async_trait::async_trait;

use super::models::*;

#[async_trait]
pub trait CloudSqlClient: Send + Sync {
    async fn list_instances(&self, project: &str) -> Result<Vec<CloudSqlInstance>>;
    async fn get_instance(&self, project: &str, instance: &str) -> Result<CloudSqlInstance>;
    async fn list_databases(&self, project: &str, instance: &str) -> Result<Vec<CloudSqlDatabase>>;
}

pub struct HttpCloudSqlClient {
    http: reqwest::Client,
    token: String,
}

impl HttpCloudSqlClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }

    fn base() -> &'static str {
        "https://sqladmin.googleapis.com/v1"
    }
}

#[async_trait]
impl CloudSqlClient for HttpCloudSqlClient {
    async fn list_instances(&self, project: &str) -> Result<Vec<CloudSqlInstance>> {
        let url = format!("{}/projects/{project}/instances", Self::base());
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self.http.get(&url).bearer_auth(&self.token);
            if let Some(ref token) = page_token {
                req = req.query(&[("pageToken", token)]);
            }

            let resp = req.send().await?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                bail!("Cloud SQL API error (list instances): {status} — {body}");
            }

            let body: InstancesListResponse = resp.json().await?;
            if let Some(items) = body.items {
                all.extend(items);
            }
            match body.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }

        Ok(all)
    }

    async fn get_instance(&self, project: &str, instance: &str) -> Result<CloudSqlInstance> {
        let url = format!("{}/projects/{project}/instances/{instance}", Self::base());

        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Cloud SQL API error (get instance {instance}): {status} — {body}");
        }

        let inst: CloudSqlInstance = resp.json().await?;
        Ok(inst)
    }

    async fn list_databases(&self, project: &str, instance: &str) -> Result<Vec<CloudSqlDatabase>> {
        let url = format!(
            "{}/projects/{project}/instances/{instance}/databases",
            Self::base()
        );

        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Cloud SQL API error (list databases): {status} — {body}");
        }

        let body: DatabasesListResponse = resp.json().await?;
        Ok(body.items.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_instances_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/projects/proj/instances"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "kind": "sql#instancesList",
                "items": [
                    {
                        "name": "prod-db",
                        "databaseVersion": "POSTGRES_15",
                        "state": "RUNNABLE",
                        "region": "us-central1",
                        "settings": {"tier": "db-custom-4-15360"},
                        "connectionName": "proj:us-central1:prod-db",
                        "instanceType": "CLOUD_SQL_INSTANCE"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let url = format!("{}/v1/projects/proj/instances", server.uri());
        let client = HttpCloudSqlClient::new("test-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .send()
            .await
            .unwrap();
        let body: InstancesListResponse = resp.json().await.unwrap();
        let items = body.items.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name.as_deref(), Some("prod-db"));
        assert_eq!(items[0].database_version.as_deref(), Some("POSTGRES_15"));
        assert_eq!(items[0].state.as_deref(), Some("RUNNABLE"));
        assert_eq!(
            items[0].settings.as_ref().unwrap().tier.as_deref(),
            Some("db-custom-4-15360")
        );
    }

    #[tokio::test]
    async fn get_instance_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/projects/proj/instances/prod-db"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "name": "prod-db",
                "databaseVersion": "POSTGRES_15",
                "state": "RUNNABLE",
                "region": "us-central1",
                "settings": {"tier": "db-custom-4-15360"},
                "connectionName": "proj:us-central1:prod-db",
                "ipAddresses": [{"type": "PRIMARY", "ipAddress": "10.0.0.1"}]
            })))
            .mount(&server)
            .await;

        let url = format!("{}/v1/projects/proj/instances/prod-db", server.uri());
        let client = HttpCloudSqlClient::new("test-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .send()
            .await
            .unwrap();
        let inst: CloudSqlInstance = resp.json().await.unwrap();

        assert_eq!(inst.name.as_deref(), Some("prod-db"));
        assert_eq!(
            inst.connection_name.as_deref(),
            Some("proj:us-central1:prod-db")
        );
        let ips = inst.ip_addresses.unwrap();
        assert_eq!(ips[0].ip_type.as_deref(), Some("PRIMARY"));
        assert_eq!(ips[0].ip_address.as_deref(), Some("10.0.0.1"));
    }

    #[tokio::test]
    async fn list_databases_parses_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/projects/proj/instances/prod-db/databases"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "kind": "sql#databasesList",
                "items": [
                    {"name": "mydb", "charset": "UTF8", "collation": "en_US.UTF8"},
                    {"name": "postgres", "charset": "UTF8"}
                ]
            })))
            .mount(&server)
            .await;

        let url = format!(
            "{}/v1/projects/proj/instances/prod-db/databases",
            server.uri()
        );
        let client = HttpCloudSqlClient::new("test-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .send()
            .await
            .unwrap();
        let body: DatabasesListResponse = resp.json().await.unwrap();
        let dbs = body.items.unwrap();

        assert_eq!(dbs.len(), 2);
        assert_eq!(dbs[0].name.as_deref(), Some("mydb"));
        assert_eq!(dbs[0].charset.as_deref(), Some("UTF8"));
        assert_eq!(dbs[1].name.as_deref(), Some("postgres"));
    }

    #[tokio::test]
    async fn get_instance_handles_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/projects/proj/instances/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let url = format!("{}/v1/projects/proj/instances/missing", server.uri());
        let client = HttpCloudSqlClient::new("test-token".into());
        let resp = client
            .http
            .get(&url)
            .bearer_auth(&client.token)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 404);
    }
}
