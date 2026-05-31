use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyncFilterError {
    #[error("invalid glob pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },
    #[error("circular include detected for path '{path}'")]
    CircularInclude { path: String },
    #[error("sync profile '{name}' not found")]
    ProfileNotFound { name: String },
}
