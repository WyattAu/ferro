//! Shared types, error types, path utilities, storage traits, and `WebDAV` types
//! used across the Ferro server and core crates.

pub mod audit;
pub mod auth;
pub mod chunk;
pub mod conflict;
pub mod error;
pub mod format;
pub mod gdpr;
pub mod metadata;
pub mod mime;
pub mod mobile_error;
pub mod multitenancy;
pub mod notifications;
pub mod path;
pub mod pools;
pub mod scheduling;
pub mod server_context;
pub mod simd;
pub mod storage;
pub mod webdav;
pub mod xml_escape;
pub mod zeroize;

#[cfg(feature = "http")]
pub mod http_client;

#[cfg(test)]
mod path_proptest;

#[cfg(test)]
mod xml_escape_proptest;

#[cfg(test)]
mod metadata_proptest;

#[cfg(test)]
mod format_proptest;

/// Canonical database handle type alias.
///
/// `Arc<Mutex<Connection>>` is used by all crates that need synchronous SQLite access
/// within an async context. Previously defined 19 times across the workspace; now unified here.
#[cfg(feature = "db")]
pub type DbHandle = std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>;

/// Run a blocking database operation on a `DbHandle` via `tokio::task::spawn_blocking`.
///
/// This prevents blocking the async runtime when executing synchronous SQLite queries.
/// All handler code that accesses `DbHandle` should use this function instead of calling
/// `db.lock().unwrap()` directly.
///
/// # Example
/// ```ignore
/// let tasks = ferro_common::db_run(db.clone(), |conn| {
///     let mut stmt = conn.prepare("SELECT * FROM tasks")?;
///     let rows = stmt.query_map([], |row| { ... })?;
///     Ok(rows.collect::<Result<Vec<_>, _>>()?)
/// }).await?;
/// ```
#[cfg(feature = "db")]
pub async fn db_run<F, T>(db: DbHandle, f: F) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T, Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
            Box::new(crate::error::FerroError::Internal(format!("DB lock poisoned: {e}")))
        })?;
        f(&conn)
    })
    .await
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(crate::error::FerroError::Internal(format!(
            "spawn_blocking panicked: {e}"
        )))
    })?
}
