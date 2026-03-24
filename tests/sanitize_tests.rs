use dcx::bigquery::sanitize::{self, SanitizeResult};

#[test]
fn print_sanitization_notice_flagged_output_on_stderr() {
    let result = SanitizeResult {
        sanitized: true,
        content: serde_json::json!({"_sanitized": true}),
        finding_summary: Some("Flagged by: piAndJailbreakFilterResult".into()),
    };

    // Should not panic — prints to stderr.
    sanitize::print_sanitization_notice(&result);
}

#[test]
fn print_sanitization_notice_clean_is_silent() {
    let result = SanitizeResult {
        sanitized: false,
        content: serde_json::json!({"data": "clean"}),
        finding_summary: None,
    };

    // Should not panic, should produce no output.
    sanitize::print_sanitization_notice(&result);
}

#[test]
fn sanitize_result_serializes_correctly() {
    let result = SanitizeResult {
        sanitized: true,
        content: serde_json::json!({"_sanitized": true, "_sanitization_message": "redacted"}),
        finding_summary: Some("Flagged by: piAndJailbreakFilterResult".into()),
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["sanitized"], true);
    assert_eq!(json["content"]["_sanitized"], true);
    assert_eq!(
        json["finding_summary"],
        "Flagged by: piAndJailbreakFilterResult"
    );
}

#[test]
fn sanitize_result_clean_serializes_correctly() {
    let original = serde_json::json!({"rows": [{"id": 1}]});
    let result = SanitizeResult {
        sanitized: false,
        content: original.clone(),
        finding_summary: None,
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["sanitized"], false);
    assert_eq!(json["content"], original);
    assert!(json["finding_summary"].is_null());
}
