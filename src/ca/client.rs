use anyhow::{bail, Result};
use async_trait::async_trait;

use crate::auth::ResolvedAuth;

use super::models::{
    extract_response, AddVerifiedQueryResponse, CaChatMessage, CaQuestionRequest,
    CaQuestionResponse, CreateAgentResponse, DataAgentSummary, ListAgentsResponse, TableRef,
};
use super::profiles::{parse_looker_explore, CaProfile};
use super::verified_queries::VerifiedQuery;

const CA_BASE_URL: &str = "https://geminidataanalytics.googleapis.com/v1beta";

/// Trait for executing CA queries. Implemented by CaClient for production
/// and by test fixtures for testing.
#[async_trait]
pub trait CaExecutor: Send + Sync {
    async fn ask(&self, project: &str, req: &CaQuestionRequest) -> Result<CaQuestionResponse>;
}

/// Parameters for creating a CA data agent.
pub struct CreateAgentParams<'a> {
    pub agent_id: &'a str,
    pub display_name: Option<&'a str>,
    pub tables: &'a [TableRef],
    pub views_count: usize,
    pub instructions: Option<&'a str>,
    pub verified_queries: &'a [VerifiedQuery],
}

/// Trait for agent management operations (create, list, update).
#[async_trait]
pub trait CaAgentManager: Send + Sync {
    async fn create_agent(
        &self,
        project: &str,
        location: &str,
        params: &CreateAgentParams<'_>,
    ) -> Result<CreateAgentResponse>;

    async fn list_agents(&self, project: &str, location: &str) -> Result<ListAgentsResponse>;

    async fn add_verified_query(
        &self,
        project: &str,
        location: &str,
        agent_id: &str,
        question: &str,
        query: &str,
    ) -> Result<AddVerifiedQueryResponse>;
}

pub struct CaClient {
    http: reqwest::Client,
    auth: ResolvedAuth,
    base_url: String,
}

impl CaClient {
    pub fn new(auth: ResolvedAuth) -> Self {
        Self {
            http: reqwest::Client::new(),
            auth,
            base_url: CA_BASE_URL.to_string(),
        }
    }

    /// Create a client with a custom base URL (for testing with wiremock).
    pub fn with_base_url(auth: ResolvedAuth, base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            auth,
            base_url,
        }
    }
}

#[async_trait]
impl CaExecutor for CaClient {
    async fn ask(&self, project: &str, req: &CaQuestionRequest) -> Result<CaQuestionResponse> {
        let location = &req.location;
        // POST /projects/{project}/locations/{location}:chat
        let url = format!(
            "{}/projects/{project}/locations/{location}:chat",
            self.base_url
        );

        let token = self.auth.token().await?;

        // Build the request body.
        let user_message = serde_json::json!({
            "userMessage": {
                "text": req.question,
            }
        });

        let mut body = serde_json::json!({
            "messages": [user_message],
        });

        // If an agent is specified, use data_agent_context.
        if let Some(ref agent) = req.agent {
            let agent_resource =
                format!("projects/{project}/locations/{location}/dataAgents/{agent}");
            body["data_agent_context"] = serde_json::json!({
                "data_agent": agent_resource,
            });
        }

        // If tables are specified (and no agent), use inlineContext.
        if req.agent.is_none() {
            if let Some(ref tables) = req.tables {
                let table_refs: Vec<serde_json::Value> = tables
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "projectId": t.project_id,
                            "datasetId": t.dataset_id,
                            "tableId": t.table_id,
                        })
                    })
                    .collect();
                body["inlineContext"] = serde_json::json!({
                    "datasource_references": {
                        "bq": {
                            "tableReferences": table_refs,
                        }
                    }
                });
            }
        }

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .header("x-server-timeout", "300")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CA API error {status}: {body}");
        }

        // The response is a JSON array of streaming messages.
        let response_text = resp.text().await?;
        let messages: Vec<CaChatMessage> = parse_streaming_response(&response_text)?;

        Ok(extract_response(
            &messages,
            &req.question,
            req.agent.as_deref(),
        ))
    }
}

#[async_trait]
impl CaAgentManager for CaClient {
    async fn create_agent(
        &self,
        project: &str,
        location: &str,
        params: &CreateAgentParams<'_>,
    ) -> Result<CreateAgentResponse> {
        let agent_id = params.agent_id;
        let url = format!(
            "{}/projects/{project}/locations/{location}/dataAgents:createSync?dataAgentId={agent_id}",
            self.base_url
        );

        let token = self.auth.token().await?;

        let table_refs: Vec<serde_json::Value> = params
            .tables
            .iter()
            .map(|t| {
                serde_json::json!({
                    "projectId": t.project_id,
                    "datasetId": t.dataset_id,
                    "tableId": t.table_id,
                })
            })
            .collect();

        let example_queries: Vec<serde_json::Value> = params
            .verified_queries
            .iter()
            .map(|vq| {
                serde_json::json!({
                    "naturalLanguageQuestion": vq.question,
                    "sqlQuery": vq.query,
                })
            })
            .collect();

        let mut published_context = serde_json::json!({
            "datasourceReferences": {
                "bq": {
                    "tableReferences": table_refs,
                }
            },
            "exampleQueries": example_queries,
        });

        if let Some(instr) = params.instructions {
            published_context["systemInstruction"] = serde_json::json!(instr);
        }

        let mut body = serde_json::json!({
            "dataAnalyticsAgent": {
                "publishedContext": published_context,
            }
        });

        let name = params.display_name.unwrap_or(agent_id);
        body["displayName"] = serde_json::json!(name);

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .header("x-server-timeout", "300")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CA API error {status}: {body}");
        }

        let agent: serde_json::Value = resp.json().await?;

        let tables_count = params.tables.len() - params.views_count;
        Ok(CreateAgentResponse {
            agent_id: agent_id.to_string(),
            name: agent["name"].as_str().unwrap_or("").to_string(),
            display_name: agent["displayName"].as_str().map(|s| s.to_string()),
            location: location.to_string(),
            create_time: agent["createTime"].as_str().map(|s| s.to_string()),
            tables_count,
            views_count: params.views_count,
            verified_queries_count: params.verified_queries.len(),
        })
    }

    async fn list_agents(&self, project: &str, location: &str) -> Result<ListAgentsResponse> {
        let url = format!(
            "{}/projects/{project}/locations/{location}/dataAgents",
            self.base_url
        );

        let token = self.auth.token().await?;

        let resp = self.http.get(&url).bearer_auth(&token).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CA API error {status}: {body}");
        }

        let body: serde_json::Value = resp.json().await?;

        let agents = body["dataAgents"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|a| {
                let full_name = a["name"].as_str().unwrap_or("");
                let id = full_name.rsplit('/').next().unwrap_or(full_name);
                DataAgentSummary {
                    agent_id: id.to_string(),
                    name: full_name.to_string(),
                    display_name: a["displayName"].as_str().map(|s| s.to_string()),
                    create_time: a["createTime"].as_str().map(|s| s.to_string()),
                    update_time: a["updateTime"].as_str().map(|s| s.to_string()),
                }
            })
            .collect();

        Ok(ListAgentsResponse { agents })
    }

    async fn add_verified_query(
        &self,
        project: &str,
        location: &str,
        agent_id: &str,
        question: &str,
        query: &str,
    ) -> Result<AddVerifiedQueryResponse> {
        let agent_name = format!("projects/{project}/locations/{location}/dataAgents/{agent_id}");

        let token = self.auth.token().await?;

        // 1. GET the existing agent to read current exampleQueries.
        let get_url = format!("{}/{agent_name}", self.base_url);
        let get_resp = self.http.get(&get_url).bearer_auth(&token).send().await?;

        let status = get_resp.status();
        if !status.is_success() {
            let body = get_resp.text().await.unwrap_or_default();
            bail!("CA API error {status}: {body}");
        }

        let mut agent: serde_json::Value = get_resp.json().await?;

        // 2. Append the new example query.
        let published = &mut agent["dataAnalyticsAgent"]["publishedContext"];
        let queries = published["exampleQueries"]
            .as_array_mut()
            .map(|a| a as &mut Vec<serde_json::Value>);

        let new_query = serde_json::json!({
            "naturalLanguageQuestion": question,
            "sqlQuery": query,
        });

        let total = if let Some(arr) = queries {
            arr.push(new_query);
            arr.len()
        } else {
            published["exampleQueries"] = serde_json::json!([new_query]);
            1
        };

        // 3. PATCH (updateSync) the agent with the updated context.
        let update_url = format!(
            "{}/{}:updateSync?updateMask=dataAnalyticsAgent.publishedContext.exampleQueries",
            self.base_url, agent_name
        );

        let update_body = serde_json::json!({
            "name": agent_name,
            "dataAnalyticsAgent": {
                "publishedContext": published,
            }
        });

        let patch_resp = self
            .http
            .patch(&update_url)
            .bearer_auth(&token)
            .header("x-server-timeout", "300")
            .json(&update_body)
            .send()
            .await?;

        let patch_status = patch_resp.status();
        if !patch_status.is_success() {
            let body = patch_resp.text().await.unwrap_or_default();
            bail!("CA API error {patch_status}: {body}");
        }

        Ok(AddVerifiedQueryResponse {
            agent_id: agent_id.to_string(),
            question: question.to_string(),
            total_verified_queries: total,
            status: "added".to_string(),
        })
    }
}

impl CaClient {
    /// Ask using a Looker profile (explore references + optional OAuth credentials).
    pub async fn ask_looker(
        &self,
        profile: &CaProfile,
        question: &str,
    ) -> Result<CaQuestionResponse> {
        let location = profile.location.as_deref().unwrap_or("us");
        let project = &profile.project;
        let url = format!(
            "{}/projects/{project}/locations/{location}:chat",
            self.base_url
        );

        let token = self.auth.token().await?;

        let user_message = serde_json::json!({
            "userMessage": { "text": question }
        });

        // Build explore references from profile
        let instance_url = profile.looker_instance_url.as_deref().unwrap_or("");
        let explores = profile.looker_explores.as_deref().unwrap_or(&[]);
        let explore_refs: Vec<serde_json::Value> = explores
            .iter()
            .filter_map(|e| {
                let (model, explore) = parse_looker_explore(e).ok()?;
                Some(serde_json::json!({
                    "lookerInstanceUri": instance_url,
                    "lookmlModel": model,
                    "explore": explore,
                }))
            })
            .collect();

        let mut looker_context = serde_json::json!({
            "exploreReferences": explore_refs,
        });

        // Add OAuth credentials if provided
        if let (Some(client_id), Some(client_secret)) =
            (&profile.looker_client_id, &profile.looker_client_secret)
        {
            looker_context["credentials"] = serde_json::json!({
                "oauth": {
                    "secret": {
                        "clientId": client_id,
                        "clientSecret": client_secret,
                    }
                }
            });
        }

        let body = serde_json::json!({
            "messages": [user_message],
            "inlineContext": {
                "datasourceReferences": {
                    "looker": looker_context,
                }
            }
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .header("x-server-timeout", "300")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CA API error {status}: {body}");
        }

        let response_text = resp.text().await?;
        let messages: Vec<CaChatMessage> = parse_streaming_response(&response_text)?;

        Ok(extract_response(&messages, question, None))
    }

    /// Ask using a Looker Studio profile (datasource reference).
    pub async fn ask_studio(
        &self,
        profile: &CaProfile,
        question: &str,
    ) -> Result<CaQuestionResponse> {
        let location = profile.location.as_deref().unwrap_or("us");
        let project = &profile.project;
        let url = format!(
            "{}/projects/{project}/locations/{location}:chat",
            self.base_url
        );

        let token = self.auth.token().await?;

        let user_message = serde_json::json!({
            "userMessage": { "text": question }
        });

        let datasource_id = profile.studio_datasource_id.as_deref().unwrap_or("");
        let body = serde_json::json!({
            "messages": [user_message],
            "inlineContext": {
                "datasourceReferences": {
                    "studio": {
                        "studioReferences": [{
                            "datasourceId": datasource_id,
                        }]
                    }
                }
            }
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .header("x-server-timeout", "300")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("CA API error {status}: {body}");
        }

        let response_text = resp.text().await?;
        let messages: Vec<CaChatMessage> = parse_streaming_response(&response_text)?;

        Ok(extract_response(&messages, question, None))
    }
}

/// Parse the CA API streaming response.
/// The API returns either a JSON array of messages or newline-delimited JSON.
fn parse_streaming_response(text: &str) -> Result<Vec<CaChatMessage>> {
    let trimmed = text.trim();

    // Try JSON array first
    if trimmed.starts_with('[') {
        let messages: Vec<CaChatMessage> = serde_json::from_str(trimmed)
            .map_err(|e| anyhow::anyhow!("Failed to parse CA response as JSON array: {e}"))?;
        return Ok(messages);
    }

    // Try newline-delimited JSON
    let mut messages = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let msg: CaChatMessage = serde_json::from_str(line)
            .map_err(|e| anyhow::anyhow!("Failed to parse CA message: {e}\nLine: {line}"))?;
        messages.push(msg);
    }

    Ok(messages)
}

/// Parse a table reference string "project.dataset.table" into a TableRef.
pub fn parse_table_refs(tables: &[String]) -> Result<Vec<TableRef>> {
    tables
        .iter()
        .map(|t| super::models::parse_table_ref(t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::resolver::AuthSource;
    use crate::auth::ResolvedAuth;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_auth() -> ResolvedAuth {
        ResolvedAuth::static_token(AuthSource::ExplicitToken, "test-token".into())
    }

    #[tokio::test]
    async fn ask_sends_question_and_parses_response() {
        let mock_server = MockServer::start().await;

        // Simulate CA API streaming response (JSON array)
        let response_body = serde_json::json!([
            {
                "systemMessage": {
                    "data": {
                        "generatedSql": "SELECT agent, COUNT(*) as cnt FROM t GROUP BY 1",
                        "result": {
                            "data": [{"agent": "support_bot", "cnt": 42}],
                            "schema": {
                                "fields": [{"name": "agent"}, {"name": "cnt"}]
                            }
                        }
                    }
                }
            },
            {
                "systemMessage": {
                    "text": {
                        "parts": ["The support_bot has 42 events."],
                        "textType": "FINAL_RESPONSE"
                    }
                }
            }
        ]);

        Mock::given(method("POST"))
            .and(path("/projects/test-project/locations/us:chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let req = CaQuestionRequest {
            question: "how many events per agent?".to_string(),
            agent: Some("my-agent".to_string()),
            tables: None,
            location: "us".to_string(),
        };

        let resp = client.ask("test-project", &req).await.unwrap();
        assert_eq!(resp.question, "how many events per agent?");
        assert_eq!(resp.agent.as_deref(), Some("my-agent"));
        assert_eq!(
            resp.sql.as_deref(),
            Some("SELECT agent, COUNT(*) as cnt FROM t GROUP BY 1")
        );
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0]["agent"], "support_bot");
        assert_eq!(resp.results[0]["cnt"], 42);
        assert_eq!(
            resp.explanation.as_deref(),
            Some("The support_bot has 42 events.")
        );
    }

    #[tokio::test]
    async fn ask_handles_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/projects/test-project/locations/us:chat"))
            .respond_with(
                ResponseTemplate::new(403).set_body_string(r#"{"error":"Permission denied"}"#),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let req = CaQuestionRequest {
            question: "test?".to_string(),
            agent: None,
            tables: None,
            location: "us".to_string(),
        };

        let result = client.ask("test-project", &req).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("CA API error"), "Got: {err}");
    }

    #[tokio::test]
    async fn ask_with_inline_tables() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/projects/test-project/locations/us:chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let req = CaQuestionRequest {
            question: "test?".to_string(),
            agent: None,
            tables: Some(vec![TableRef {
                project_id: "proj".into(),
                dataset_id: "ds".into(),
                table_id: "tbl".into(),
            }]),
            location: "us".to_string(),
        };

        let resp = client.ask("test-project", &req).await.unwrap();
        assert_eq!(resp.question, "test?");
    }

    #[test]
    fn parse_streaming_json_array() {
        let text =
            r#"[{"systemMessage":{"text":{"parts":["hello"],"textType":"FINAL_RESPONSE"}}}]"#;
        let messages = parse_streaming_response(text).unwrap();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn parse_streaming_ndjson() {
        let text = r#"{"systemMessage":{"text":{"parts":["line1"],"textType":"THOUGHT"}}}
{"systemMessage":{"text":{"parts":["line2"],"textType":"FINAL_RESPONSE"}}}"#;
        let messages = parse_streaming_response(text).unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn parse_table_refs_valid() {
        let refs = parse_table_refs(&["a.b.c".into(), "x.y.z".into()]).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].project_id, "a");
        assert_eq!(refs[1].table_id, "z");
    }

    #[test]
    fn parse_table_refs_invalid() {
        let result = parse_table_refs(&["invalid".into()]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_agent_sends_correct_request() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "name": "projects/test-project/locations/us/dataAgents/my-agent",
            "displayName": "my-agent",
            "createTime": "2026-03-13T00:00:00Z",
        });

        Mock::given(method("POST"))
            .and(path(
                "/projects/test-project/locations/us/dataAgents:createSync",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let tables = vec![TableRef {
            project_id: "proj".into(),
            dataset_id: "ds".into(),
            table_id: "tbl".into(),
        }];
        let vqs = vec![VerifiedQuery {
            question: "How many errors?".into(),
            query: "SELECT COUNT(*) FROM t".into(),
        }];

        let params = CreateAgentParams {
            agent_id: "my-agent",
            display_name: Some("my-agent"),
            tables: &tables,
            views_count: 0,
            instructions: Some("Test instructions"),
            verified_queries: &vqs,
        };

        let resp = client
            .create_agent("test-project", "us", &params)
            .await
            .unwrap();

        assert_eq!(resp.agent_id, "my-agent");
        assert_eq!(resp.tables_count, 1);
        assert_eq!(resp.views_count, 0);
        assert_eq!(resp.verified_queries_count, 1);
        assert_eq!(resp.location, "us");
    }

    #[tokio::test]
    async fn list_agents_parses_response() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "dataAgents": [
                {
                    "name": "projects/test-project/locations/us/dataAgents/agent-1",
                    "displayName": "Agent One",
                    "createTime": "2026-03-13T00:00:00Z",
                    "updateTime": "2026-03-13T01:00:00Z",
                },
                {
                    "name": "projects/test-project/locations/us/dataAgents/agent-2",
                    "displayName": "Agent Two",
                    "createTime": "2026-03-12T00:00:00Z",
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/projects/test-project/locations/us/dataAgents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let resp = client.list_agents("test-project", "us").await.unwrap();

        assert_eq!(resp.agents.len(), 2);
        assert_eq!(resp.agents[0].agent_id, "agent-1");
        assert_eq!(resp.agents[0].display_name.as_deref(), Some("Agent One"));
        assert_eq!(resp.agents[1].agent_id, "agent-2");
    }

    #[tokio::test]
    async fn list_agents_empty() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/test-project/locations/us/dataAgents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let resp = client.list_agents("test-project", "us").await.unwrap();

        assert!(resp.agents.is_empty());
    }

    #[tokio::test]
    async fn add_verified_query_appends_to_existing() {
        let mock_server = MockServer::start().await;

        // GET returns agent with one existing example query
        let get_response = serde_json::json!({
            "name": "projects/test-project/locations/us/dataAgents/my-agent",
            "dataAnalyticsAgent": {
                "publishedContext": {
                    "exampleQueries": [
                        {
                            "naturalLanguageQuestion": "Existing question?",
                            "sqlQuery": "SELECT 1",
                        }
                    ]
                }
            }
        });

        Mock::given(method("GET"))
            .and(path(
                "/projects/test-project/locations/us/dataAgents/my-agent",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&get_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        // PATCH succeeds
        Mock::given(method("PATCH"))
            .and(path(
                "/projects/test-project/locations/us/dataAgents/my-agent:updateSync",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let resp = client
            .add_verified_query(
                "test-project",
                "us",
                "my-agent",
                "New question?",
                "SELECT 2",
            )
            .await
            .unwrap();

        assert_eq!(resp.agent_id, "my-agent");
        assert_eq!(resp.question, "New question?");
        assert_eq!(resp.total_verified_queries, 2); // 1 existing + 1 new
        assert_eq!(resp.status, "added");
    }

    fn looker_profile() -> CaProfile {
        CaProfile {
            name: "test-looker".into(),
            source_type: super::super::profiles::SourceType::Looker,
            project: "test-project".into(),
            location: Some("us".into()),
            agent: None,
            tables: None,
            looker_instance_url: Some("https://looker.example.com".into()),
            looker_explores: Some(vec!["sales_model/orders".into()]),
            looker_client_id: Some("my-client-id".into()),
            looker_client_secret: Some("my-client-secret".into()),
            studio_datasource_id: None,
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        }
    }

    fn studio_profile() -> CaProfile {
        CaProfile {
            name: "test-studio".into(),
            source_type: super::super::profiles::SourceType::LookerStudio,
            project: "test-project".into(),
            location: Some("us".into()),
            agent: None,
            tables: None,
            looker_instance_url: None,
            looker_explores: None,
            looker_client_id: None,
            looker_client_secret: None,
            studio_datasource_id: Some("ds-12345".into()),
            context_set_id: None,
            datasource_ref: None,
            db_type: None,
            connection_name: None,
        }
    }

    #[tokio::test]
    async fn ask_looker_sends_explore_references() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!([
            {
                "systemMessage": {
                    "data": {
                        "query": {
                            "looker": {
                                "queryUrl": "https://looker.example.com/explore/sales_model/orders?q=..."
                            }
                        },
                        "result": {
                            "data": [{"order_id": 1, "total": 100.0}],
                            "schema": {
                                "fields": [{"name": "order_id"}, {"name": "total"}]
                            }
                        }
                    }
                }
            },
            {
                "systemMessage": {
                    "text": {
                        "parts": ["There is 1 order totaling $100."],
                        "textType": "FINAL_RESPONSE"
                    }
                }
            }
        ]);

        Mock::given(method("POST"))
            .and(path("/projects/test-project/locations/us:chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let profile = looker_profile();
        let resp = client
            .ask_looker(&profile, "how many orders?")
            .await
            .unwrap();

        assert_eq!(resp.question, "how many orders?");
        assert!(resp.sql.is_some()); // Looker query URL
        assert!(resp.sql.unwrap().contains("looker.example.com"));
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0]["order_id"], 1);
        assert_eq!(
            resp.explanation.as_deref(),
            Some("There is 1 order totaling $100.")
        );
    }

    #[tokio::test]
    async fn ask_looker_handles_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/projects/test-project/locations/us:chat"))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_string(r#"{"error":"Looker permission denied"}"#),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let profile = looker_profile();
        let result = client.ask_looker(&profile, "test?").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CA API error"));
    }

    #[tokio::test]
    async fn ask_studio_sends_datasource_reference() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!([
            {
                "systemMessage": {
                    "data": {
                        "generatedSql": "SELECT metric FROM studio_data",
                        "result": {
                            "data": [{"metric": "views", "value": 5000}],
                            "schema": {
                                "fields": [{"name": "metric"}, {"name": "value"}]
                            }
                        }
                    }
                }
            },
            {
                "systemMessage": {
                    "text": {
                        "parts": ["The dashboard shows 5000 views."],
                        "textType": "FINAL_RESPONSE"
                    }
                }
            }
        ]);

        Mock::given(method("POST"))
            .and(path("/projects/test-project/locations/us:chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = CaClient::with_base_url(test_auth(), mock_server.uri());
        let profile = studio_profile();
        let resp = client.ask_studio(&profile, "show me views").await.unwrap();

        assert_eq!(resp.question, "show me views");
        assert_eq!(resp.sql.as_deref(), Some("SELECT metric FROM studio_data"));
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0]["value"], 5000);
        assert_eq!(
            resp.explanation.as_deref(),
            Some("The dashboard shows 5000 views.")
        );
    }
}
