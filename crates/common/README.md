# ferro-common

[![crates.io](https://img.shields.io/crates/v/ferro-common.svg)](https://crates.io/crates/ferro-common)
[![docs.rs](https://docs.rs/ferro-common/badge.svg)](https://docs.rs/ferro-common)
[![license](https://img.shields.io/badge/license-AGPL-3.0-blue.svg)](LICENSE)

Foundation types and traits shared across the Ferro ecosystem. This crate defines the core `StorageEngine` trait, file metadata types, error handling, WebDAV protocol types, authentication primitives, and path utilities used by all other Ferro crates.

## Key Types

- **`StorageEngine`** — async trait defining the storage backend interface (`head`, `get`, `get_stream`, `put`, `delete`, `list`, `copy`, `move_path`, `exists`, `create_collection`, `list_all`, `put_multipart`)
- **`FileMetadata`** — metadata for files and collections (path, content hash, size, MIME type, timestamps, owner, ETag)
- **`ContentHash`** — SHA-256 content hash with computation, streaming, and ETag parsing
- **`StorageReader`** — async reader wrapper for streaming file content
- **`FerroError`** — unified error type with HTTP status code mapping
- **`Claims`**, **`AuthDecision`**, **`AuthRequest`** — authentication and authorization types
- **`LockToken`**, **`LockInfo`**, **`LockDepth`**, **`LockScope`** — WebDAV locking types
- **`MultiStatusResponse`**, **`MultiStatusItem`**, **`WebDavProperty`** — WebDAV multistatus response types
- Path utilities: `normalize_path`, `parent_path`, `base_name`, `is_collection_path`, `validate_path`, `join_path`

## Usage

Implement a custom storage backend by implementing `StorageEngine`:

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

## Examples

### Computing a content hash

```rust
use ferro_common::metadata::ContentHash;

let hash = ContentHash::compute(b"hello world");
println!("SHA-256: {}", hash.as_hex());

let hash = ContentHash::from_etag("\"abc123\"");
```

### Path normalization

```rust
use ferro_common::path::{normalize_path, join_path, base_name};

assert_eq!(normalize_path("/foo/../bar"), "/bar");
assert_eq!(join_path("/docs", "file.txt"), "/docs/file.txt");
assert_eq!(base_name("/docs/file.txt"), "file.txt");
```

### Error handling

```rust
use ferro_common::error::FerroError;

let err = FerroError::NotFound("/missing.txt".into());
assert_eq!(err.status_code(), 404);
```

## License

Licensed under AGPL-3.0-or-later.
