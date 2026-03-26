use anyhow::{bail, Result};
use async_trait::async_trait;

use super::models::*;
use crate::ca::profiles::CaProfile;

/// Trait for Looker API operations, enabling mock testing.
#[async_trait]
pub trait LookerClient: Send + Sync {
    async fn list_explores(&self, profile: &CaProfile) -> Result<Vec<ExploreSummary>>;
    async fn get_explore(
        &self,
        profile: &CaProfile,
        model: &str,
        explore: &str,
    ) -> Result<ExploreDetail>;
    async fn list_dashboards(&self, profile: &CaProfile) -> Result<Vec<DashboardSummary>>;
    async fn get_dashboard(
        &self,
        profile: &CaProfile,
        dashboard_id: &str,
    ) -> Result<DashboardDetail>;
}

/// HTTP-based Looker client that talks to the Looker REST API.
pub struct HttpLookerClient {
    http: reqwest::Client,
    /// Bearer token for Looker API authentication.
    /// This comes from GCP auth (for Looker instances that use Google auth)
    /// or from Looker API credentials in the profile.
    token: String,
}

impl HttpLookerClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }

    fn base_url(profile: &CaProfile) -> Result<String> {
        let url = profile
            .looker_instance_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Profile missing looker_instance_url"))?;
        // Ensure no trailing slash
        Ok(url.trim_end_matches('/').to_string())
    }
}

#[async_trait]
impl LookerClient for HttpLookerClient {
    async fn list_explores(&self, profile: &CaProfile) -> Result<Vec<ExploreSummary>> {
        let base = Self::base_url(profile)?;

        // Fetch all LookML models, then flatten their explores.
        let url = format!("{base}/api/4.0/lookml_models");
        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Looker API error (GET /api/4.0/lookml_models): {status} — {body}");
        }

        let models: Vec<LookmlModelSummary> = resp.json().await?;

        let mut explores = Vec::new();
        for model in models {
            if let Some(model_explores) = model.explores {
                for mut e in model_explores {
                    // The API nests explores under the model; ensure model_name is set.
                    if e.model_name.is_empty() {
                        e.model_name = model.name.clone();
                    }
                    explores.push(e);
                }
            }
        }
        Ok(explores)
    }

    async fn get_explore(
        &self,
        profile: &CaProfile,
        model: &str,
        explore: &str,
    ) -> Result<ExploreDetail> {
        let base = Self::base_url(profile)?;
        let url = format!(
            "{base}/api/4.0/lookml_models/{model}/explores/{explore}",
            model = urlencoding::encode(model),
            explore = urlencoding::encode(explore),
        );

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("fields", "model_name,name,label,description,hidden,fields")])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Looker API error (GET explore {model}/{explore}): {status} — {body}");
        }

        let detail: ExploreDetail = resp.json().await?;
        Ok(detail)
    }

    async fn list_dashboards(&self, profile: &CaProfile) -> Result<Vec<DashboardSummary>> {
        let base = Self::base_url(profile)?;
        let url = format!("{base}/api/4.0/dashboards");

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("fields", "id,title,description,folder,hidden,readonly")])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Looker API error (GET /api/4.0/dashboards): {status} — {body}");
        }

        let dashboards: Vec<DashboardSummary> = resp.json().await?;
        Ok(dashboards)
    }

    async fn get_dashboard(
        &self,
        profile: &CaProfile,
        dashboard_id: &str,
    ) -> Result<DashboardDetail> {
        let base = Self::base_url(profile)?;
        let url = format!(
            "{base}/api/4.0/dashboards/{id}",
            id = urlencoding::encode(dashboard_id),
        );

        let resp = self.http.get(&url).bearer_auth(&self.token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Looker API error (GET dashboard {dashboard_id}): {status} — {body}");
        }

        let detail: DashboardDetail = resp.json().await?;
        Ok(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_profile(instance_url: &str) -> CaProfile {
        CaProfile {
            name: "test-looker".into(),
            source_type: crate::ca::profiles::SourceType::Looker,
            project: "test-project".into(),
            location: Some("us-central1".into()),
            agent: None,
            tables: None,
            looker_instance_url: Some(instance_url.into()),
            looker_explores: Some(vec!["model/explore".into()]),
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: None,
            context_set_id: None,
            cluster_id: None,
            instance_id: None,
            database_id: None,
            db_type: None,
        }
    }

    #[tokio::test]
    async fn list_explores_parses_response() {
        let server = MockServer::start().await;
        let profile = test_profile(&server.uri());

        Mock::given(method("GET"))
            .and(path("/api/4.0/lookml_models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": "sales_model",
                    "explores": [
                        {
                            "model_name": "sales_model",
                            "name": "orders",
                            "label": "Orders",
                            "description": "Order data"
                        },
                        {
                            "model_name": "sales_model",
                            "name": "customers",
                            "label": "Customers"
                        }
                    ]
                }
            ])))
            .mount(&server)
            .await;

        let client = HttpLookerClient::new("test-token".into());
        let explores = client.list_explores(&profile).await.unwrap();

        assert_eq!(explores.len(), 2);
        assert_eq!(explores[0].name, "orders");
        assert_eq!(explores[0].model_name, "sales_model");
        assert_eq!(explores[0].label.as_deref(), Some("Orders"));
        assert_eq!(explores[1].name, "customers");
    }

    #[tokio::test]
    async fn get_explore_parses_detail() {
        let server = MockServer::start().await;
        let profile = test_profile(&server.uri());

        Mock::given(method("GET"))
            .and(path("/api/4.0/lookml_models/sales_model/explores/orders"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "model_name": "sales_model",
                "name": "orders",
                "label": "Orders",
                "description": "All orders",
                "fields": {
                    "dimensions": [
                        {"name": "order_id", "type": "number", "label": "Order ID"}
                    ],
                    "measures": [
                        {"name": "total_revenue", "type": "sum", "label": "Total Revenue"}
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client = HttpLookerClient::new("test-token".into());
        let detail = client
            .get_explore(&profile, "sales_model", "orders")
            .await
            .unwrap();

        assert_eq!(detail.name, "orders");
        assert_eq!(detail.model_name, "sales_model");
        let fields = detail.fields.unwrap();
        assert_eq!(fields.dimensions.as_ref().unwrap().len(), 1);
        assert_eq!(fields.measures.as_ref().unwrap().len(), 1);
        assert_eq!(fields.dimensions.unwrap()[0].name, "order_id");
    }

    #[tokio::test]
    async fn list_dashboards_parses_response() {
        let server = MockServer::start().await;
        let profile = test_profile(&server.uri());

        Mock::given(method("GET"))
            .and(path("/api/4.0/dashboards"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "42",
                    "title": "Sales Dashboard",
                    "description": "Main sales overview",
                    "folder": {"id": "1", "name": "Shared"},
                    "hidden": false
                },
                {
                    "id": "99",
                    "title": "Executive Report",
                    "hidden": true
                }
            ])))
            .mount(&server)
            .await;

        let client = HttpLookerClient::new("test-token".into());
        let dashboards = client.list_dashboards(&profile).await.unwrap();

        assert_eq!(dashboards.len(), 2);
        assert_eq!(dashboards[0].id.as_deref(), Some("42"));
        assert_eq!(dashboards[0].title.as_deref(), Some("Sales Dashboard"));
        assert_eq!(
            dashboards[0].folder.as_ref().unwrap().name.as_deref(),
            Some("Shared")
        );
        assert_eq!(dashboards[1].hidden, Some(true));
    }

    #[tokio::test]
    async fn get_dashboard_parses_detail() {
        let server = MockServer::start().await;
        let profile = test_profile(&server.uri());

        Mock::given(method("GET"))
            .and(path("/api/4.0/dashboards/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "42",
                "title": "Sales Dashboard",
                "description": "Main sales overview",
                "dashboard_elements": [
                    {
                        "id": "elem-1",
                        "title": "Revenue Chart",
                        "type": "vis",
                        "model": "sales_model",
                        "explore": "orders"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = HttpLookerClient::new("test-token".into());
        let detail = client.get_dashboard(&profile, "42").await.unwrap();

        assert_eq!(detail.id.as_deref(), Some("42"));
        assert_eq!(detail.title.as_deref(), Some("Sales Dashboard"));
        let elements = detail.dashboard_elements.unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].title.as_deref(), Some("Revenue Chart"));
        assert_eq!(elements[0].model.as_deref(), Some("sales_model"));
    }

    #[tokio::test]
    async fn list_explores_handles_api_error() {
        let server = MockServer::start().await;
        let profile = test_profile(&server.uri());

        Mock::given(method("GET"))
            .and(path("/api/4.0/lookml_models"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let client = HttpLookerClient::new("bad-token".into());
        let err = client.list_explores(&profile).await.unwrap_err();
        assert!(err.to_string().contains("401"));
    }
}
