use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventBusError {
    #[error("handler '{name}' failed for event '{event_type}': {reason}")]
    HandlerFailed {
        name: String,
        event_type: String,
        reason: String,
    },
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("deserialization error: {0}")]
    Deserialization(String),
    #[error("no handler found for event type '{0}'")]
    NoHandler(String),
    #[error("interceptor error: {0}")]
    InterceptorError(String),
}

impl EventBusError {
    pub fn handler_failed(name: &str, event_type: &str, reason: impl std::fmt::Display) -> Self {
        Self::HandlerFailed {
            name: name.to_string(),
            event_type: event_type.to_string(),
            reason: reason.to_string(),
        }
    }
}
