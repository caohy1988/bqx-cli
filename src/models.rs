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
