use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// A single verified query (example query) for a data agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedQuery {
    pub question: String,
    pub query: String,
}

/// Top-level YAML structure for the verified queries file.
#[derive(Debug, Deserialize)]
pub struct VerifiedQueriesFile {
    pub verified_queries: Vec<VerifiedQuery>,
}

/// The bundled verified queries YAML shipped with the SDK.
const BUNDLED_VERIFIED_QUERIES: &str = include_str!("../../deploy/ca/verified_queries.yaml");

/// Load verified queries from a file path or the bundled default.
pub fn load(path: Option<&str>) -> Result<Vec<VerifiedQuery>> {
    let content = match path {
        Some(p) => std::fs::read_to_string(p)
            .map_err(|e| anyhow::anyhow!("Failed to read verified queries file '{p}': {e}"))?,
        None => BUNDLED_VERIFIED_QUERIES.to_string(),
    };
    parse(&content)
}

/// Parse verified queries from YAML content.
pub fn parse(yaml: &str) -> Result<Vec<VerifiedQuery>> {
    let file: VerifiedQueriesFile = serde_yaml::from_str(yaml)
        .map_err(|e| anyhow::anyhow!("Invalid verified queries YAML: {e}"))?;
    validate(&file.verified_queries)?;
    Ok(file.verified_queries)
}

/// Validate that each verified query has a non-empty question and query.
fn validate(queries: &[VerifiedQuery]) -> Result<()> {
    if queries.is_empty() {
        bail!("Verified queries file must contain at least one query");
    }
    for (i, q) in queries.iter().enumerate() {
        if q.question.trim().is_empty() {
            bail!("Verified query #{}: question cannot be empty", i + 1);
        }
        if q.query.trim().is_empty() {
            bail!("Verified query #{}: query cannot be empty", i + 1);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_loads_successfully() {
        let queries = load(None).unwrap();
        assert!(queries.len() >= 4, "Expected at least 4 bundled queries");
    }

    #[test]
    fn bundled_has_expected_questions() {
        let queries = load(None).unwrap();
        let questions: Vec<&str> = queries.iter().map(|q| q.question.as_str()).collect();
        assert!(questions.iter().any(|q| q.contains("error rate")));
        assert!(questions.iter().any(|q| q.contains("p95 latency")));
        assert!(questions.iter().any(|q| q.contains("tools fail")));
        assert!(questions.iter().any(|q| q.contains("highest latency")));
    }

    #[test]
    fn parse_valid_yaml() {
        let yaml = r#"
verified_queries:
  - question: "How many users?"
    query: "SELECT COUNT(*) FROM users"
"#;
        let queries = parse(yaml).unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].question, "How many users?");
    }

    #[test]
    fn parse_rejects_empty_list() {
        let yaml = "verified_queries: []";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn parse_rejects_empty_question() {
        let yaml = r#"
verified_queries:
  - question: ""
    query: "SELECT 1"
"#;
        let err = parse(yaml).unwrap_err().to_string();
        assert!(err.contains("question cannot be empty"), "Got: {err}");
    }

    #[test]
    fn parse_rejects_empty_query() {
        let yaml = r#"
verified_queries:
  - question: "test?"
    query: "  "
"#;
        let err = parse(yaml).unwrap_err().to_string();
        assert!(err.contains("query cannot be empty"), "Got: {err}");
    }

    #[test]
    fn parse_rejects_invalid_yaml() {
        let yaml = "not: valid: yaml: [";
        assert!(parse(yaml).is_err());
    }
}
