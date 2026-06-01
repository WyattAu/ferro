use thiserror::Error;

#[derive(Debug, Error)]
pub enum HealthError {
    #[error("probe '{name}' not found")]
    ProbeNotFound { name: String },

    #[error("probe '{name}' already registered")]
    ProbeAlreadyRegistered { name: String },

    #[error("check failed for probe '{name}': {message}")]
    CheckFailed { name: String, message: String },
}
