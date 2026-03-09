use thiserror::Error;

#[derive(Error, Debug)]
pub enum BqxError {
    #[error("Evaluation failed")]
    EvalFailed { exit_code: i32 },
}
