use serde::Serialize;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BqxError {
    #[error("Evaluation failed")]
    EvalFailed { exit_code: i32 },
    /// Infrastructure error (connection, auth, bad input).
    /// Matches SDK exit-code 2.
    #[error("{message}")]
    InfraError { message: String },
}

impl BqxError {
    /// SDK-compatible exit code: 1 = eval failure, 2 = infra error.
    pub fn exit_code(&self) -> i32 {
        match self {
            BqxError::EvalFailed { exit_code } => *exit_code,
            BqxError::InfraError { .. } => 2,
        }
    }
}

// ---------------------------------------------------------------------------
// Typed error envelope
// ---------------------------------------------------------------------------

/// Machine-readable error code for structured error dispatch.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    MissingArgument,
    InvalidIdentifier,
    InvalidConfig,
    UnknownCommand,
    AuthError,
    ApiError,
    NotFound,
    Conflict,
    EvalFailed,
    InfraError,
    Internal,
}

/// Typed error envelope emitted on stderr as JSON.
///
/// Shape: `{"error": {"code": "...", "message": "...", ...}}`
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub exit_code: i32,
    pub retryable: bool,
    pub status: &'static str,
}

impl ErrorEnvelope {
    pub fn new(code: ErrorCode, message: impl Into<String>, exit_code: i32) -> Self {
        Self {
            code,
            message: message.into(),
            hint: None,
            exit_code,
            retryable: false,
            status: "error",
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    /// Inspect the message for API error status codes and transport-level
    /// failures, and set `retryable` for transient errors agents should retry.
    pub fn detect_retryable(mut self) -> Self {
        if let Some(code) = extract_http_status(&self.message) {
            self.retryable = matches!(code, 408 | 429 | 500 | 502 | 503 | 504);
        } else {
            self.retryable = is_transient_transport_error(&self.message);
        }
        self
    }

    /// Refine error code and exit code based on HTTP status in the message.
    ///
    /// - 401/403 → `AuthError`, exit 3
    /// - 404 → `NotFound`, exit 4
    /// - 409 → `Conflict`, exit 5
    pub fn detect_semantic_exit_code(mut self) -> Self {
        if let Some(status) = extract_http_status(&self.message) {
            match status {
                401 | 403 => {
                    self.code = ErrorCode::AuthError;
                    self.exit_code = 3;
                }
                404 => {
                    self.code = ErrorCode::NotFound;
                    self.exit_code = 4;
                }
                409 => {
                    self.code = ErrorCode::Conflict;
                    self.exit_code = 5;
                }
                _ => {}
            }
        }
        self
    }

    /// Emit the envelope to stderr and exit.
    pub fn emit_and_exit(self) -> ! {
        eprintln!("{}", json!({"error": self}));
        std::process::exit(self.exit_code);
    }

    /// Emit the envelope to stderr without exiting.
    pub fn emit(&self) {
        eprintln!("{}", json!({"error": self}));
    }
}

// ---------------------------------------------------------------------------
// Confirmation envelope (exit code 2)
// ---------------------------------------------------------------------------

/// Structured confirmation envelope emitted when a mutation requires user
/// confirmation but `--yes` was not passed and stdin is not a TTY.
///
/// Agents can parse this to decide whether to re-run with `--yes`.
#[derive(Debug, Clone, Serialize)]
pub struct ConfirmationEnvelope {
    pub status: &'static str,
    pub changes: Vec<String>,
    pub confirm_command: String,
    pub exit_code: i32,
}

impl ConfirmationEnvelope {
    pub fn new(changes: Vec<String>, confirm_command: impl Into<String>) -> Self {
        Self {
            status: "confirmation_required",
            changes,
            confirm_command: confirm_command.into(),
            exit_code: 2,
        }
    }

    /// Emit the envelope to stdout and exit.
    pub fn emit_and_exit(self) -> ! {
        println!("{}", serde_json::json!(self));
        std::process::exit(self.exit_code);
    }
}

impl From<&BqxError> for ErrorEnvelope {
    fn from(e: &BqxError) -> Self {
        match e {
            BqxError::EvalFailed { exit_code } => {
                ErrorEnvelope::new(ErrorCode::EvalFailed, "Evaluation failed", *exit_code)
            }
            BqxError::InfraError { message } => {
                ErrorEnvelope::new(ErrorCode::InfraError, message.clone(), 2).detect_retryable()
            }
        }
    }
}

/// Common transport-level error substrings that indicate transient failures.
const TRANSIENT_PATTERNS: &[&str] = &[
    "error sending request",
    "connection refused",
    "connection reset",
    "connection closed",
    "timed out",
    "timeout",
    "dns error",
    "broken pipe",
    "reset by peer",
];

/// Check whether an error message indicates a transient transport failure.
fn is_transient_transport_error(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    TRANSIENT_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Extract HTTP status code from error messages matching "API error NNN:".
fn extract_http_status(msg: &str) -> Option<u16> {
    let marker = "API error ";
    let start = msg.find(marker)? + marker.len();
    let rest = &msg[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialization_shape() {
        let env = ErrorEnvelope::new(ErrorCode::MissingArgument, "missing --project-id", 1)
            .with_hint("Set DCX_PROJECT or pass --project-id.");
        let json = serde_json::to_value(json!({"error": env})).unwrap();
        let err = &json["error"];
        assert_eq!(err["code"], "MISSING_ARGUMENT");
        assert_eq!(err["message"], "missing --project-id");
        assert_eq!(err["hint"], "Set DCX_PROJECT or pass --project-id.");
        assert_eq!(err["exit_code"], 1);
        assert_eq!(err["retryable"], false);
        assert_eq!(err["status"], "error");
    }

    #[test]
    fn envelope_no_hint_omitted() {
        let env = ErrorEnvelope::new(ErrorCode::ApiError, "timeout", 2);
        let json = serde_json::to_value(&env).unwrap();
        assert!(json.get("hint").is_none());
    }

    #[test]
    fn envelope_retryable() {
        let env = ErrorEnvelope::new(ErrorCode::ApiError, "503", 2).retryable();
        assert!(env.retryable);
    }

    #[test]
    fn envelope_from_bqx_error() {
        let eval = BqxError::EvalFailed { exit_code: 1 };
        let env = ErrorEnvelope::from(&eval);
        assert_eq!(env.exit_code, 1);
        assert!(matches!(env.code, ErrorCode::EvalFailed));

        let infra = BqxError::InfraError {
            message: "connection refused".into(),
        };
        let env = ErrorEnvelope::from(&infra);
        assert_eq!(env.exit_code, 2);
        assert_eq!(env.message, "connection refused");
    }

    #[test]
    fn detect_retryable_on_transient_errors() {
        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "BigQuery API error 503: Service Unavailable",
            2,
        )
        .detect_retryable();
        assert!(env.retryable);

        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "CA API error 429 Too Many Requests: rate limited",
            2,
        )
        .detect_retryable();
        assert!(env.retryable);

        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "Spanner API error 500: Internal Server Error",
            2,
        )
        .detect_retryable();
        assert!(env.retryable);
    }

    #[test]
    fn detect_retryable_not_on_client_errors() {
        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "BigQuery API error 401: Unauthorized",
            2,
        )
        .detect_retryable();
        assert!(!env.retryable);

        let env = ErrorEnvelope::new(ErrorCode::ApiError, "BigQuery API error 404: Not Found", 2)
            .detect_retryable();
        assert!(!env.retryable);
    }

    #[test]
    fn detect_retryable_transport_errors() {
        let cases = [
            "error sending request for url (https://bigquery.googleapis.com/...): connection reset",
            "connection refused",
            "operation timed out",
            "dns error: failed to lookup address",
            "connection reset by peer",
            "broken pipe",
        ];
        for msg in cases {
            let env = ErrorEnvelope::new(ErrorCode::InfraError, msg, 2).detect_retryable();
            assert!(env.retryable, "should be retryable: {msg}");
        }
    }

    #[test]
    fn detect_retryable_no_match() {
        let env = ErrorEnvelope::new(
            ErrorCode::InvalidConfig,
            "--project-id or DCX_PROJECT is required",
            1,
        )
        .detect_retryable();
        assert!(!env.retryable);
    }

    #[test]
    fn extract_http_status_parses_codes() {
        assert_eq!(
            extract_http_status("BigQuery API error 503: Service Unavailable"),
            Some(503)
        );
        assert_eq!(
            extract_http_status("CA API error 429 Too Many Requests: rate limited"),
            Some(429)
        );
        assert_eq!(extract_http_status("connection refused"), None);
    }

    #[test]
    fn infra_error_bqx_detects_retryable() {
        let infra = BqxError::InfraError {
            message: "BigQuery API error 503: Service Unavailable".into(),
        };
        let env = ErrorEnvelope::from(&infra);
        assert!(env.retryable);
    }

    #[test]
    fn semantic_exit_code_401_maps_to_auth_error() {
        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "BigQuery API error 401: Unauthorized",
            2,
        )
        .detect_semantic_exit_code();
        assert!(matches!(env.code, ErrorCode::AuthError));
        assert_eq!(env.exit_code, 3);
    }

    #[test]
    fn semantic_exit_code_403_maps_to_auth_error() {
        let env = ErrorEnvelope::new(ErrorCode::ApiError, "Spanner API error 403: Forbidden", 2)
            .detect_semantic_exit_code();
        assert!(matches!(env.code, ErrorCode::AuthError));
        assert_eq!(env.exit_code, 3);
    }

    #[test]
    fn semantic_exit_code_404_maps_to_not_found() {
        let env = ErrorEnvelope::new(ErrorCode::ApiError, "BigQuery API error 404: Not Found", 2)
            .detect_semantic_exit_code();
        assert!(matches!(env.code, ErrorCode::NotFound));
        assert_eq!(env.exit_code, 4);
    }

    #[test]
    fn semantic_exit_code_409_maps_to_conflict() {
        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "Cloud SQL API error 409: Already Exists",
            2,
        )
        .detect_semantic_exit_code();
        assert!(matches!(env.code, ErrorCode::Conflict));
        assert_eq!(env.exit_code, 5);
    }

    #[test]
    fn semantic_exit_code_leaves_500_unchanged() {
        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "BigQuery API error 500: Internal Server Error",
            2,
        )
        .detect_semantic_exit_code();
        assert!(matches!(env.code, ErrorCode::ApiError));
        assert_eq!(env.exit_code, 2);
    }

    #[test]
    fn semantic_exit_code_no_status_unchanged() {
        let env = ErrorEnvelope::new(ErrorCode::InfraError, "connection refused", 2)
            .detect_semantic_exit_code();
        assert!(matches!(env.code, ErrorCode::InfraError));
        assert_eq!(env.exit_code, 2);
    }

    #[test]
    fn semantic_and_retryable_compose() {
        let env = ErrorEnvelope::new(ErrorCode::ApiError, "BigQuery API error 404: Not Found", 2)
            .detect_semantic_exit_code()
            .detect_retryable();
        assert!(matches!(env.code, ErrorCode::NotFound));
        assert_eq!(env.exit_code, 4);
        assert!(!env.retryable);

        let env = ErrorEnvelope::new(
            ErrorCode::ApiError,
            "BigQuery API error 503: Service Unavailable",
            2,
        )
        .detect_semantic_exit_code()
        .detect_retryable();
        assert!(matches!(env.code, ErrorCode::ApiError));
        assert_eq!(env.exit_code, 2);
        assert!(env.retryable);
    }

    #[test]
    fn confirmation_envelope_shape() {
        let env = ConfirmationEnvelope::new(
            vec!["Would create agent 'my-agent'".into()],
            "dcx ca create-agent --name my-agent --tables proj.ds.tbl --yes",
        );
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["status"], "confirmation_required");
        assert_eq!(json["exit_code"], 2);
        assert_eq!(json["changes"][0], "Would create agent 'my-agent'");
        assert!(json["confirm_command"].as_str().unwrap().contains("--yes"));
    }
}
