use serde::{Deserialize, Serialize};

/// Request sent to the Conversational Analytics API.
#[derive(Debug, Clone, Serialize)]
pub struct CaQuestionRequest {
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables: Option<Vec<TableRef>>,
    /// Location for the CA API (e.g. "us", "eu")
    #[serde(skip)]
    pub location: String,
}

/// A BigQuery table reference for inline context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRef {
    pub project_id: String,
    pub dataset_id: String,
    pub table_id: String,
}

/// Stable output contract for `bqx ca ask`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaQuestionResponse {
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(default)]
    pub results: Vec<serde_json::Map<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

// ── CA API wire types ──

/// A single message in the streaming chat response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaChatMessage {
    pub system_message: Option<SystemMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemMessage {
    pub text: Option<TextContent>,
    pub data: Option<DataContent>,
    pub error: Option<ErrorContent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TextContent {
    pub parts: Option<Vec<String>>,
    pub text_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataContent {
    pub generated_sql: Option<String>,
    pub result: Option<DataResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataResult {
    /// Row data as array of objects (actual CA API shape)
    pub data: Option<Vec<serde_json::Map<String, serde_json::Value>>>,
    /// Schema with field definitions (preserved for future use)
    #[allow(dead_code)]
    pub schema: Option<ResultSchema>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResultSchema {
    #[allow(dead_code)]
    pub fields: Option<Vec<SchemaField>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SchemaField {
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ErrorContent {
    pub text: Option<String>,
}

/// Parse a table reference string like "project.dataset.table" into a TableRef.
pub fn parse_table_ref(s: &str) -> anyhow::Result<TableRef> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid table reference: '{s}'. Expected format: project.dataset.table");
    }
    Ok(TableRef {
        project_id: parts[0].to_string(),
        dataset_id: parts[1].to_string(),
        table_id: parts[2].to_string(),
    })
}

/// Extract structured results from the streaming chat response messages.
pub(crate) fn extract_response(
    messages: &[CaChatMessage],
    question: &str,
    agent: Option<&str>,
) -> CaQuestionResponse {
    let mut sql = None;
    let mut results = Vec::new();
    let mut explanation = None;
    let mut error_text = None;

    for msg in messages {
        let Some(ref sys) = msg.system_message else {
            continue;
        };

        // Extract error
        if let Some(ref err) = sys.error {
            if let Some(ref t) = err.text {
                error_text = Some(t.clone());
            }
        }

        // Extract SQL and result data
        if let Some(ref data) = sys.data {
            if data.generated_sql.is_some() {
                sql = data.generated_sql.clone();
            }
            if let Some(ref result) = data.result {
                // The CA API returns results as an array of row objects in `data`
                if let Some(ref rows) = result.data {
                    for row in rows {
                        results.push(row.clone());
                    }
                }
            }
        }

        // Extract final response text as explanation
        if let Some(ref text) = sys.text {
            if text.text_type.as_deref() == Some("FINAL_RESPONSE") {
                if let Some(ref parts) = text.parts {
                    let joined = parts.join("\n");
                    if !joined.is_empty() {
                        explanation = Some(joined);
                    }
                }
            }
        }
    }

    // If there was an error and no explanation, use error as explanation
    if explanation.is_none() {
        if let Some(err) = error_text {
            explanation = Some(format!("Error: {err}"));
        }
    }

    CaQuestionResponse {
        question: question.to_string(),
        agent: agent.map(|s| s.to_string()),
        sql,
        results,
        explanation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_table_ref_valid() {
        let r = parse_table_ref("myproject.mydataset.mytable").unwrap();
        assert_eq!(r.project_id, "myproject");
        assert_eq!(r.dataset_id, "mydataset");
        assert_eq!(r.table_id, "mytable");
    }

    #[test]
    fn parse_table_ref_invalid() {
        assert!(parse_table_ref("just_a_table").is_err());
        assert!(parse_table_ref("project.dataset").is_err());
        assert!(parse_table_ref("a.b.c.d").is_err());
    }

    #[test]
    fn extract_response_from_messages() {
        let messages = vec![
            CaChatMessage {
                system_message: Some(SystemMessage {
                    text: Some(TextContent {
                        parts: Some(vec!["Thinking...".into()]),
                        text_type: Some("THOUGHT".into()),
                    }),
                    data: None,
                    error: None,
                }),
            },
            CaChatMessage {
                system_message: Some(SystemMessage {
                    text: None,
                    data: Some(DataContent {
                        generated_sql: Some("SELECT agent, COUNT(*) FROM t GROUP BY 1".into()),
                        result: Some(DataResult {
                            data: Some(vec![{
                                let mut map = serde_json::Map::new();
                                map.insert("agent".into(), serde_json::json!("support_bot"));
                                map.insert("count".into(), serde_json::json!(42));
                                map
                            }]),
                            schema: Some(ResultSchema {
                                fields: Some(vec![
                                    SchemaField {
                                        name: "agent".into(),
                                    },
                                    SchemaField {
                                        name: "count".into(),
                                    },
                                ]),
                            }),
                        }),
                    }),
                    error: None,
                }),
            },
            CaChatMessage {
                system_message: Some(SystemMessage {
                    text: Some(TextContent {
                        parts: Some(vec!["The support_bot has 42 events.".into()]),
                        text_type: Some("FINAL_RESPONSE".into()),
                    }),
                    data: None,
                    error: None,
                }),
            },
        ];

        let resp = extract_response(&messages, "how many events?", Some("my-agent"));
        assert_eq!(resp.question, "how many events?");
        assert_eq!(resp.agent.as_deref(), Some("my-agent"));
        assert_eq!(
            resp.sql.as_deref(),
            Some("SELECT agent, COUNT(*) FROM t GROUP BY 1")
        );
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0]["agent"], "support_bot");
        assert_eq!(resp.results[0]["count"], 42);
        assert_eq!(
            resp.explanation.as_deref(),
            Some("The support_bot has 42 events.")
        );
    }

    #[test]
    fn extract_response_empty_messages() {
        let resp = extract_response(&[], "test?", None);
        assert!(resp.results.is_empty());
        assert!(resp.sql.is_none());
        assert!(resp.explanation.is_none());
    }

    #[test]
    fn extract_response_error_message() {
        let messages = vec![CaChatMessage {
            system_message: Some(SystemMessage {
                text: None,
                data: None,
                error: Some(ErrorContent {
                    text: Some("Table not found".into()),
                }),
            }),
        }];

        let resp = extract_response(&messages, "test?", None);
        assert!(resp
            .explanation
            .as_deref()
            .unwrap()
            .contains("Table not found"));
    }

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
}
