//! Model Armor integration for response sanitization.
//!
//! When `--sanitize <template>` is provided, API responses are screened
//! through Model Armor before rendering. Responses flagged by the template's
//! filters are redacted to prevent prompt injection content from reaching
//! downstream consumers.

use anyhow::{bail, Result};
use serde::Serialize;

use crate::auth::ResolvedAuth;

/// Result of screening content through Model Armor.
#[derive(Debug, Clone, Serialize)]
pub struct SanitizeResult {
    /// Whether any content was flagged and redacted.
    pub sanitized: bool,
    /// The (possibly modified) response content.
    pub content: serde_json::Value,
    /// Human-readable summary of findings, if any.
    pub finding_summary: Option<String>,
}

/// Screen API response content against a Model Armor template.
///
/// The `template` parameter is a fully-qualified Model Armor template resource
/// name, e.g. `projects/my-proj/locations/us-central1/templates/my-template`.
///
/// Returns the original content unmodified if no filters match, or a redacted
/// version with a summary of findings if content is flagged.
pub async fn sanitize_response(
    auth: &ResolvedAuth,
    template: &str,
    content: &serde_json::Value,
) -> Result<SanitizeResult> {
    let text = serde_json::to_string(content)?;

    let client = reqwest::Client::new();
    let token = auth.token().await?;

    let url = format!(
        "https://modelarmor.googleapis.com/v1/{template}:sanitizeModelResponse",
    );

    let body = serde_json::json!({
        "model_response_data": {
            "text": {
                "text": text
            }
        }
    });

    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        bail!("Model Armor API error {}: {err_body}", status.as_u16());
    }

    let result: serde_json::Value = resp.json().await?;
    parse_sanitize_response(&result, content)
}

/// Parse Model Armor response and determine if content was flagged.
fn parse_sanitize_response(
    ma_response: &serde_json::Value,
    original_content: &serde_json::Value,
) -> Result<SanitizeResult> {
    let match_state = ma_response
        .pointer("/sanitizationResult/filterMatchState")
        .and_then(|v| v.as_str())
        .unwrap_or("NO_MATCH_FOUND");

    if match_state == "MATCH_FOUND" {
        let summary = extract_finding_summary(ma_response);
        Ok(SanitizeResult {
            sanitized: true,
            content: serde_json::json!({
                "_sanitized": true,
                "_sanitization_message": "Response content was redacted by Model Armor",
                "_finding_summary": summary,
            }),
            finding_summary: Some(summary),
        })
    } else {
        Ok(SanitizeResult {
            sanitized: false,
            content: original_content.clone(),
            finding_summary: None,
        })
    }
}

/// Extract a human-readable summary from Model Armor filter results.
fn extract_finding_summary(ma_response: &serde_json::Value) -> String {
    let filter_results = ma_response
        .pointer("/sanitizationResult/filterResults")
        .and_then(|v| v.as_object());

    let Some(filters) = filter_results else {
        return "Content was flagged by Model Armor".to_string();
    };

    let mut findings: Vec<String> = Vec::new();
    for (filter_name, filter_value) in filters {
        let state = filter_value
            .pointer("/filterMatchState")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");
        if state == "MATCH_FOUND" {
            findings.push(filter_name.clone());
        }
    }

    if findings.is_empty() {
        "Content was flagged by Model Armor".to_string()
    } else {
        format!("Flagged by: {}", findings.join(", "))
    }
}

/// Render a sanitization notice to stderr so it's visible regardless of format.
pub fn print_sanitization_notice(result: &SanitizeResult) {
    if result.sanitized {
        let summary = result
            .finding_summary
            .as_deref()
            .unwrap_or("Content was redacted");
        eprintln!("[sanitize] Response was redacted by Model Armor: {summary}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_match() {
        let ma_response = serde_json::json!({
            "sanitizationResult": {
                "filterMatchState": "NO_MATCH_FOUND",
                "filterResults": {}
            }
        });
        let original = serde_json::json!({"data": "safe content"});
        let result = parse_sanitize_response(&ma_response, &original).unwrap();
        assert!(!result.sanitized);
        assert_eq!(result.content, original);
        assert!(result.finding_summary.is_none());
    }

    #[test]
    fn parse_match_found_redacts_content() {
        let ma_response = serde_json::json!({
            "sanitizationResult": {
                "filterMatchState": "MATCH_FOUND",
                "filterResults": {
                    "piAndJailbreakFilterResult": {
                        "filterMatchState": "MATCH_FOUND"
                    },
                    "sdpFilterResult": {
                        "filterMatchState": "NO_MATCH_FOUND"
                    }
                }
            }
        });
        let original = serde_json::json!({"data": "injected content"});
        let result = parse_sanitize_response(&ma_response, &original).unwrap();
        assert!(result.sanitized);
        assert!(result.content.get("_sanitized").is_some());
        assert_eq!(
            result.finding_summary.as_deref(),
            Some("Flagged by: piAndJailbreakFilterResult")
        );
    }

    #[test]
    fn parse_match_with_no_filter_details() {
        let ma_response = serde_json::json!({
            "sanitizationResult": {
                "filterMatchState": "MATCH_FOUND"
            }
        });
        let original = serde_json::json!({"data": "flagged"});
        let result = parse_sanitize_response(&ma_response, &original).unwrap();
        assert!(result.sanitized);
        assert_eq!(
            result.finding_summary.as_deref(),
            Some("Content was flagged by Model Armor")
        );
    }

    #[test]
    fn extract_summary_multiple_filters() {
        let ma_response = serde_json::json!({
            "sanitizationResult": {
                "filterMatchState": "MATCH_FOUND",
                "filterResults": {
                    "piAndJailbreakFilterResult": {
                        "filterMatchState": "MATCH_FOUND"
                    },
                    "maliciousUriFilterResult": {
                        "filterMatchState": "MATCH_FOUND"
                    },
                    "sdpFilterResult": {
                        "filterMatchState": "NO_MATCH_FOUND"
                    }
                }
            }
        });
        let summary = extract_finding_summary(&ma_response);
        assert!(summary.contains("piAndJailbreakFilterResult"));
        assert!(summary.contains("maliciousUriFilterResult"));
        assert!(!summary.contains("sdpFilterResult"));
    }
}
