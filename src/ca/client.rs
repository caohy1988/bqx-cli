use anyhow::{bail, Result};
use async_trait::async_trait;

use crate::auth::ResolvedAuth;

use super::models::{
    extract_response, CaChatMessage, CaQuestionRequest, CaQuestionResponse, TableRef,
};

const CA_BASE_URL: &str = "https://geminidataanalytics.googleapis.com/v1beta";

/// Trait for executing CA queries. Implemented by CaClient for production
/// and by test fixtures for testing.
#[async_trait]
pub trait CaExecutor: Send + Sync {
    async fn ask(&self, project: &str, req: &CaQuestionRequest) -> Result<CaQuestionResponse>;
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
}
