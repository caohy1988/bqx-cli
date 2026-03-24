use anyhow::Result;
use async_trait::async_trait;

use dcx::ca::client::{CaAgentManager, CaExecutor, CreateAgentParams};
use dcx::ca::models::{
    AddVerifiedQueryResponse, CaQuestionRequest, CaQuestionResponse, CreateAgentResponse,
    DataAgentSummary, ListAgentsResponse,
};
use dcx::cli::OutputFormat;
use dcx::commands::ca::ask::{build_request, validate_inputs};
use dcx::config::Config;

// ── MockCaExecutor ──

struct MockCaExecutor {
    response: CaQuestionResponse,
}

impl MockCaExecutor {
    fn new(response: CaQuestionResponse) -> Self {
        Self { response }
    }
}

#[async_trait]
impl CaExecutor for MockCaExecutor {
    async fn ask(&self, _project: &str, req: &CaQuestionRequest) -> Result<CaQuestionResponse> {
        let mut resp = self.response.clone();
        resp.question = req.question.clone();
        resp.agent = req.agent.clone();
        Ok(resp)
    }
}

fn test_config(format: OutputFormat) -> Config {
    Config {
        project_id: "test-project".into(),
        dataset_id: Some("test_dataset".into()),
        location: "US".into(),
        table: "agent_events".into(),
        format,
        sanitize_template: None,
    }
}

// ═══════════════════════════════════════════════
// Input Validation Tests
// ═══════════════════════════════════════════════

#[test]
fn validate_empty_question_fails() {
    assert!(validate_inputs("", None, None).is_err());
    assert!(validate_inputs("   ", None, None).is_err());
}

#[test]
fn validate_normal_question_succeeds() {
    assert!(validate_inputs("error rate for support_bot?", None, None).is_ok());
}

#[test]
fn validate_rejects_agent_and_tables_together() {
    let tables = vec!["p.d.t".into()];
    let result = validate_inputs("test?", Some("my-agent"), Some(&tables));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("--agent and --tables cannot be used together"),
        "Got: {err}"
    );
}

#[test]
fn validate_rejects_malformed_agent_id() {
    let result = validate_inputs("test?", Some("bad/agent"), None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid agent_id"), "Got: {err}");
}

#[test]
fn validate_accepts_agent_only() {
    assert!(validate_inputs("test?", Some("my-agent"), None).is_ok());
}

#[test]
fn validate_accepts_tables_only() {
    let tables = vec!["p.d.t".into()];
    assert!(validate_inputs("test?", None, Some(&tables)).is_ok());
}

// ═══════════════════════════════════════════════
// Request Builder Tests
// ═══════════════════════════════════════════════

#[test]
fn build_request_with_agent() {
    let req = build_request("test question".into(), Some("my-agent".into()), None, "us").unwrap();
    assert_eq!(req.question, "test question");
    assert_eq!(req.agent.as_deref(), Some("my-agent"));
    assert!(req.tables.is_none());
    assert_eq!(req.location, "us");
}

#[test]
fn build_request_with_tables() {
    let req = build_request(
        "test question".into(),
        None,
        Some(vec!["proj.ds.tbl".into()]),
        "us",
    )
    .unwrap();
    assert_eq!(req.question, "test question");
    assert!(req.agent.is_none());
    assert_eq!(req.tables.as_ref().unwrap().len(), 1);
    assert_eq!(req.tables.as_ref().unwrap()[0].project_id, "proj");
}

#[test]
fn build_request_with_no_optional_fields() {
    let req = build_request("just a question".into(), None, None, "us").unwrap();
    assert_eq!(req.question, "just a question");
    assert!(req.agent.is_none());
    assert!(req.tables.is_none());
}

#[test]
fn build_request_rejects_invalid_table_ref() {
    let result = build_request(
        "test".into(),
        None,
        Some(vec!["invalid_table".into()]),
        "us",
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid table reference"));
}

// ═══════════════════════════════════════════════
// Response Mapping Tests
// ═══════════════════════════════════════════════

#[test]
fn question_response_serializes_to_json() {
    let resp = CaQuestionResponse {
        question: "error rate?".into(),
        agent: Some("agent-analytics".into()),
        sql: Some("SELECT COUNT(*) FROM t".into()),
        results: vec![{
            let mut map = serde_json::Map::new();
            map.insert("count".into(), serde_json::json!(42));
            map
        }],
        explanation: Some("Shows total count".into()),
    };

    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["question"], "error rate?");
    assert_eq!(json["agent"], "agent-analytics");
    assert_eq!(json["sql"], "SELECT COUNT(*) FROM t");
    assert_eq!(json["results"][0]["count"], 42);
    assert_eq!(json["explanation"], "Shows total count");
}

#[test]
fn question_response_omits_none_fields() {
    let resp = CaQuestionResponse {
        question: "test?".into(),
        agent: None,
        sql: None,
        results: vec![],
        explanation: None,
    };

    let json = serde_json::to_value(&resp).unwrap();
    assert!(!json.as_object().unwrap().contains_key("agent"));
    assert!(!json.as_object().unwrap().contains_key("sql"));
    assert!(!json.as_object().unwrap().contains_key("explanation"));
}

// ═══════════════════════════════════════════════
// Integration Tests (through run_with_executor)
// ═══════════════════════════════════════════════

#[tokio::test]
async fn run_with_executor_json_output() {
    let mock = MockCaExecutor::new(CaQuestionResponse {
        question: String::new(),
        agent: None,
        sql: Some("SELECT 1".into()),
        results: vec![{
            let mut map = serde_json::Map::new();
            map.insert("val".into(), serde_json::json!(1));
            map
        }],
        explanation: Some("Returns 1".into()),
    });

    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::ca::ask::run_with_executor(
        &mock,
        "test question".into(),
        Some("my-agent".into()),
        None,
        "us",
        &config,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn run_with_executor_text_output() {
    let mock = MockCaExecutor::new(CaQuestionResponse {
        question: String::new(),
        agent: None,
        sql: Some("SELECT 1".into()),
        results: vec![],
        explanation: None,
    });

    let config = test_config(OutputFormat::Text);
    let result = dcx::commands::ca::ask::run_with_executor(
        &mock,
        "test question".into(),
        None,
        None,
        "us",
        &config,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn run_with_executor_empty_question_fails() {
    let mock = MockCaExecutor::new(CaQuestionResponse {
        question: String::new(),
        agent: None,
        sql: None,
        results: vec![],
        explanation: None,
    });

    let config = test_config(OutputFormat::Json);
    let result =
        dcx::commands::ca::ask::run_with_executor(&mock, "".into(), None, None, "us", &config)
            .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Question cannot be empty"));
}

// ═══════════════════════════════════════════════
// MockCaAgentManager
// ═══════════════════════════════════════════════

struct MockCaAgentManager;

#[async_trait]
impl CaAgentManager for MockCaAgentManager {
    async fn create_agent(
        &self,
        project: &str,
        location: &str,
        params: &CreateAgentParams<'_>,
    ) -> Result<CreateAgentResponse> {
        Ok(CreateAgentResponse {
            agent_id: params.agent_id.to_string(),
            name: format!(
                "projects/{project}/locations/{location}/dataAgents/{}",
                params.agent_id
            ),
            display_name: params.display_name.map(|s| s.to_string()),
            location: location.to_string(),
            create_time: Some("2026-03-13T00:00:00Z".into()),
            tables_count: params.tables.len() - params.views_count,
            views_count: params.views_count,
            verified_queries_count: params.verified_queries.len(),
        })
    }

    async fn list_agents(&self, _project: &str, _location: &str) -> Result<ListAgentsResponse> {
        Ok(ListAgentsResponse {
            agents: vec![DataAgentSummary {
                agent_id: "test-agent".into(),
                name: "projects/p/locations/us/dataAgents/test-agent".into(),
                display_name: Some("Test Agent".into()),
                create_time: Some("2026-03-13T00:00:00Z".into()),
                update_time: None,
            }],
        })
    }

    async fn add_verified_query(
        &self,
        _project: &str,
        _location: &str,
        agent_id: &str,
        question: &str,
        _query: &str,
    ) -> Result<AddVerifiedQueryResponse> {
        Ok(AddVerifiedQueryResponse {
            agent_id: agent_id.to_string(),
            question: question.to_string(),
            total_verified_queries: 5,
            status: "added".to_string(),
        })
    }
}

// ═══════════════════════════════════════════════
// Create Agent Validation Tests
// ═══════════════════════════════════════════════

#[test]
fn create_agent_validate_rejects_empty_tables() {
    use dcx::commands::ca::create_agent::validate_inputs;
    let result = validate_inputs("my-agent", &[], None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("--tables is required"));
}

#[test]
fn create_agent_validate_rejects_invalid_name() {
    use dcx::commands::ca::create_agent::validate_inputs;
    let result = validate_inputs("bad/name", &["p.d.t".into()], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid agent_id"));
}

#[test]
fn create_agent_validate_rejects_invalid_table_ref() {
    use dcx::commands::ca::create_agent::validate_inputs;
    let result = validate_inputs("my-agent", &["bad_ref".into()], None);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid table reference"));
}

#[test]
fn create_agent_validate_accepts_valid_inputs() {
    use dcx::commands::ca::create_agent::validate_inputs;
    assert!(validate_inputs("my-agent", &["p.d.t".into()], None).is_ok());
}

// ═══════════════════════════════════════════════
// Add Verified Query Validation Tests
// ═══════════════════════════════════════════════

#[test]
fn add_vq_validate_rejects_empty_question() {
    use dcx::commands::ca::add_verified_query::validate_inputs;
    let result = validate_inputs("my-agent", "", "SELECT 1");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("--question cannot be empty"));
}

#[test]
fn add_vq_validate_rejects_empty_query() {
    use dcx::commands::ca::add_verified_query::validate_inputs;
    let result = validate_inputs("my-agent", "test?", "  ");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("--query cannot be empty"));
}

#[test]
fn add_vq_validate_rejects_invalid_agent() {
    use dcx::commands::ca::add_verified_query::validate_inputs;
    let result = validate_inputs("bad/agent", "test?", "SELECT 1");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid agent_id"));
}

#[test]
fn add_vq_validate_accepts_valid_inputs() {
    use dcx::commands::ca::add_verified_query::validate_inputs;
    assert!(validate_inputs("my-agent", "How many?", "SELECT COUNT(*) FROM t").is_ok());
}

// ═══════════════════════════════════════════════
// Verified Queries YAML Tests
// ═══════════════════════════════════════════════

#[test]
fn bundled_verified_queries_load() {
    let queries = dcx::ca::verified_queries::load(None).unwrap();
    assert!(queries.len() >= 4);
}

// ═══════════════════════════════════════════════
// Agent Management Integration Tests
// ═══════════════════════════════════════════════

#[tokio::test]
async fn create_agent_with_executor_json() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::ca::create_agent::run_with_executor(
        &mock,
        "my-agent".into(),
        vec!["p.d.t".into()],
        None,
        None,
        None,
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn create_agent_with_executor_text() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Text);
    let result = dcx::commands::ca::create_agent::run_with_executor(
        &mock,
        "my-agent".into(),
        vec!["p.d.t".into()],
        None,
        None,
        Some("Test instructions".into()),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn create_agent_empty_tables_fails() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::ca::create_agent::run_with_executor(
        &mock,
        "my-agent".into(),
        vec![],
        None,
        None,
        None,
        &config,
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_agents_with_executor() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::ca::list_agents::run_with_executor(&mock, &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn list_agents_with_executor_text() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Text);
    let result = dcx::commands::ca::list_agents::run_with_executor(&mock, &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn add_verified_query_with_executor() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::ca::add_verified_query::run_with_executor(
        &mock,
        "my-agent".into(),
        "What is the error rate?".into(),
        "SELECT COUNT(*) FROM t".into(),
        &config,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn add_verified_query_empty_question_fails() {
    let mock = MockCaAgentManager;
    let config = test_config(OutputFormat::Json);
    let result = dcx::commands::ca::add_verified_query::run_with_executor(
        &mock,
        "my-agent".into(),
        "".into(),
        "SELECT 1".into(),
        &config,
    )
    .await;
    assert!(result.is_err());
}
