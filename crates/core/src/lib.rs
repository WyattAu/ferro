//! Core storage, metadata, search, WASM worker, and persistence abstractions.

pub mod cas;
pub mod fs_util;
pub mod metadata;
pub mod presigned;
pub mod storage;

#[cfg(feature = "sqlite")]
pub mod persistence;

#[cfg(feature = "sqlite")]
pub mod sqlx_metadata;

#[cfg(feature = "postgres")]
pub use sqlx_metadata::PgMetadataStore;

#[cfg(feature = "search")]
pub mod search;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub mod wasm_abi;

#[cfg(feature = "object_store")]
pub mod object_store_backend;

#[cfg(feature = "object_store")]
pub use object_store_backend::ObjectStoreStorageEngine;

#[cfg(feature = "object_store")]
pub use object_store_backend::MULTIPART_THRESHOLD;

#[cfg(feature = "object_store")]
pub use object_store_backend::MULTIPART_CHUNK_SIZE;
