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

/// Stable output contract for `dcx ca ask`.
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

// ── Agent management output contracts ──

/// Stable output contract for `dcx ca create-agent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentResponse {
    pub agent_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    pub tables_count: usize,
    pub views_count: usize,
    pub verified_queries_count: usize,
}

/// Stable output contract for `dcx ca list-agents`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<DataAgentSummary>,
}

/// Summary of a data agent (used in list output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAgentSummary {
    pub agent_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
}

/// Stable output contract for `dcx ca add-verified-query`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVerifiedQueryResponse {
    pub agent_id: String,
    pub question: String,
    pub total_verified_queries: usize,
    pub status: String,
}

// ── CA API wire types (chat) ──

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
    /// Looker responses return the query under `query.looker` instead of `generatedSql`.
    pub query: Option<QueryContent>,
    pub result: Option<DataResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryContent {
    pub looker: Option<LookerQuery>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LookerQuery {
    /// The generated Looker query URL or description.
    pub query_url: Option<String>,
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

// ── QueryData API wire types (database sources) ──

/// Response from the QueryData API endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryDataApiResponse {
    pub generated_query: Option<String>,
    pub intent_explanation: Option<String>,
    pub query_result: Option<ExecutedQueryResult>,
    pub natural_language_answer: Option<String>,
    #[allow(dead_code)]
    pub disambiguation_question: Option<Vec<String>>,
}

/// Result of executing a generated query.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutedQueryResult {
    pub columns: Option<Vec<QueryDataColumn>>,
    pub rows: Option<Vec<QueryDataRow>>,
    #[allow(dead_code)]
    pub total_row_count: Option<String>,
    #[allow(dead_code)]
    pub partial_result: Option<bool>,
    pub query_execution_error: Option<String>,
}

/// Column metadata from QueryData result.
#[derive(Debug, Deserialize)]
pub(crate) struct QueryDataColumn {
    pub name: String,
}

/// A row of values from QueryData result.
#[derive(Debug, Deserialize)]
pub(crate) struct QueryDataRow {
    pub values: Vec<QueryDataValue>,
}

/// A cell value in a QueryData result row.
#[derive(Debug, Deserialize)]
pub(crate) struct QueryDataValue {
    pub value: Option<String>,
}

/// Convert a QueryData API response to our stable CaQuestionResponse.
pub(crate) fn convert_querydata_response(
    resp: &QueryDataApiResponse,
    question: &str,
) -> CaQuestionResponse {
    let mut results = Vec::new();

    if let Some(ref qr) = resp.query_result {
        if let Some(ref error) = qr.query_execution_error {
            return CaQuestionResponse {
                question: question.to_string(),
                agent: None,
                sql: resp.generated_query.clone(),
                results: vec![],
                explanation: Some(format!("Query execution error: {error}")),
            };
        }

        let col_names: Vec<&str> = qr
            .columns
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|c| c.name.as_str())
            .collect();

        if let Some(ref rows) = qr.rows {
            for row in rows {
                let mut map = serde_json::Map::new();
                for (i, val) in row.values.iter().enumerate() {
                    let col_name = col_names.get(i).copied().unwrap_or("_unknown").to_string();
                    let json_val = match &val.value {
                        Some(v) => serde_json::Value::String(v.clone()),
                        None => serde_json::Value::Null,
                    };
                    map.insert(col_name, json_val);
                }
                results.push(map);
            }
        }
    }

    // Prefer natural_language_answer, fall back to intent_explanation
    let explanation = resp
        .natural_language_answer
        .clone()
        .or_else(|| resp.intent_explanation.clone());

    CaQuestionResponse {
        question: question.to_string(),
        agent: None,
        sql: resp.generated_query.clone(),
        results,
        explanation,
    }
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
            // Looker responses use query.looker.query_url instead of generatedSql
            if sql.is_none() {
                if let Some(ref q) = data.query {
                    if let Some(ref looker) = q.looker {
                        if let Some(ref url) = looker.query_url {
                            sql = Some(url.clone());
                        }
                    }
                }
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
                        query: None,
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
