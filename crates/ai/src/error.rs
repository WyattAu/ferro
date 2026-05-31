use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("embedding generation failed: {reason}")]
    EmbeddingFailed { reason: String },
    #[error("model '{model_name}' is not loaded")]
    ModelNotLoaded { model_name: String },
    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },
    #[error("tag conflict for '{tag}': existing value '{existing_value}' differs from new value '{new_value}'")]
    TagConflict {
        tag: String,
        existing_value: String,
        new_value: String,
    },
    #[error("semantic index is full (capacity: {capacity})")]
    IndexFull { capacity: usize },
}
