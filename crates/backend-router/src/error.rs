use thiserror::Error;

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("no policy named '{0}' found")]
    PolicyNotFound(String),
    #[error("policy '{0}' already exists")]
    PolicyAlreadyExists(String),
    #[error("invalid glob pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },
    #[error("routing produced no decision for path '{0}'")]
    NoDecision(String),
}
