# ferro-common

Foundation types and traits shared across the Ferro ecosystem. Defines the core `StorageEngine` trait, file metadata types, error handling, WebDAV protocol types, authentication primitives, and path utilities.

## Key Types

| Type | Description |
|------|-------------|
| `StorageEngine` | Async trait for storage backend interface |
| `FileMetadata` | Metadata for files and collections |
| `ContentHash` | SHA-256 content hash with ETag parsing |
| `StorageReader` | Async reader wrapper for streaming |
| `FerroError` | Unified error type with HTTP status mapping |
| `Claims` | Authentication claims |
| `AuthDecision` | Authorization decision |
| `LockToken` / `LockInfo` | WebDAV locking types |
| `MultiStatusResponse` | WebDAV multistatus response |

### StorageEngine Trait

The central abstraction -- all storage backends implement this trait:

```rust
pub trait StorageEngine: Send + Sync {
    async fn head(&self, path: &str) -> Result<FileMetadata>;
    async fn get(&self, path: &str) -> Result<Bytes>;
    async fn get_stream(&self, path: &str) -> Result<StorageReader>;
    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn list(&self, path: &str) -> Result<Vec<FileMetadata>>;
    async fn copy(&self, from: &str, to: &str) -> Result<()>;
    async fn move_path(&self, from: &str, to: &str) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata>;
    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>>;
    async fn put_multipart(&self, path: &str, parts: Vec<Bytes>, owner: &str) -> Result<FileMetadata>;
}
```

### Path Utilities

```rust
use ferro_common::path::{normalize_path, join_path, base_name};

assert_eq!(normalize_path("/foo/../bar"), "/bar");
assert_eq!(join_path("/docs", "file.txt"), "/docs/file.txt");
assert_eq!(base_name("/docs/file.txt"), "file.txt");
```

## Feature Flags

This crate has no feature flags -- it is always included as a dependency.

## Minimal Usage

Implement a custom storage backend:

```rust
use async_trait::async_trait;
use bytes::Bytes;
use ferro_common::storage::StorageEngine;
use ferro_common::metadata::FileMetadata;
use ferro_common::error::Result;

struct MyBackend;

#[async_trait]
impl StorageEngine for MyBackend {
    async fn head(&self, path: &str) -> Result<FileMetadata> { todo!() }
    async fn get(&self, path: &str) -> Result<Bytes> { todo!() }
    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> { todo!() }
    async fn delete(&self, path: &str) -> Result<()> { todo!() }
    async fn list(&self, path: &str) -> Result<Vec<FileMetadata>> { todo!() }
    async fn copy(&self, from: &str, to: &str) -> Result<()> { todo!() }
    async fn move_path(&self, from: &str, to: &str) -> Result<()> { todo!() }
    async fn exists(&self, path: &str) -> Result<bool> { todo!() }
    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> { todo!() }
    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>> { todo!() }
}
```
