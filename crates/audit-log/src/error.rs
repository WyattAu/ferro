use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("chain verification failed at entry {index}: {reason}")]
    ChainBroken { index: usize, reason: String },
    #[error("export error: {0}")]
    Export(String),
    #[error("entry not found: {0}")]
    NotFound(String),
    #[error("invalid filter: {0}")]
    InvalidFilter(String),
    #[error("lock poisoned")]
    LockPoisoned,
}
