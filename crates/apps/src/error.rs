/// Errors produced by the courses_apps I/O crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("DynamoDB error: {0}")]
    Dynamo(String),

    #[error("CloudWatch error: {0}")]
    CloudWatch(String),

    #[error("AWS config error: {0}")]
    Config(String),

    #[error(transparent)]
    Payload(#[from] courses_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
