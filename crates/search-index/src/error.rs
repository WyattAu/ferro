use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum SearchError {
    #[error("document not found: {0}")]
    DocumentNotFound(String),
    #[error("document already exists: {0}")]
    DocumentAlreadyExists(String),
    #[error("document ID cannot be empty")]
    EmptyDocumentId,
    #[error("query cannot be empty")]
    EmptyQuery,
    #[error("unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("index error: {0}")]
    IndexError(String),
}
