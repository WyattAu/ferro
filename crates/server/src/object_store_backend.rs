//! Re-export of the ObjectStoreStorageEngine from ferro-core.
//! This allows the server binary to construct a real filesystem-backed
//! storage engine via `ferro_server::object_store_backend::ObjectStoreStorageEngine`.

pub use ferro_core::object_store_backend::ObjectStoreStorageEngine;
