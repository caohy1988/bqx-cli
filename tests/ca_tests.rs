use anyhow::Result;
use async_trait::async_trait;

use bqx::ca::client::CaExecutor;
use bqx::ca::models::{CaQuestionRequest, CaQuestionResponse};
use bqx::cli::OutputFormat;
use bqx::commands::ca::ask::{build_request, validate_inputs};
use bqx::config::Config;

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
    assert!(validate_inputs("").is_err());
    assert!(validate_inputs("   ").is_err());
}

#[test]
fn validate_normal_question_succeeds() {
    assert!(validate_inputs("error rate for support_bot?").is_ok());
}

// ═══════════════════════════════════════════════
// Request Builder Tests
// ═══════════════════════════════════════════════

#[test]
fn build_request_sets_all_fields() {
    let req = build_request(
        "test question".into(),
        Some("my-agent".into()),
        Some(vec!["proj.ds.tbl".into()]),
        "us",
    )
    .unwrap();
    assert_eq!(req.question, "test question");
    assert_eq!(req.agent.as_deref(), Some("my-agent"));
    assert_eq!(req.tables.as_ref().unwrap().len(), 1);
    assert_eq!(req.tables.as_ref().unwrap()[0].project_id, "proj");
    assert_eq!(req.location, "us");
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
    assert!(result.unwrap_err().to_string().contains("Invalid table reference"));
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
    let result = bqx::commands::ca::ask::run_with_executor(
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
    let result = bqx::commands::ca::ask::run_with_executor(
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
    let result = bqx::commands::ca::ask::run_with_executor(
        &mock,
        "".into(),
        None,
        None,
        "us",
        &config,
    )
    .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Question cannot be empty"));
}
