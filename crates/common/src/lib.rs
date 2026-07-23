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
