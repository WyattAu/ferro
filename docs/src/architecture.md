# Architecture

## Crate Structure

Ferro is built as a Rust workspace with 10 crates:

```
ferro/
+-- crates/
|   +-- common/       # Shared types, StorageEngine trait, error handling
|   +-- core/         # Storage backends, search engine, WASM runtime
|   +-- server/       # Axum web server, WebDAV, REST API, all HTTP handlers
|   +-- dav/          # CalDAV/CardDAV parsers, store traits, Axum handlers
|   +-- crypto/       # Cryptographic primitives (hashing, HMAC, bcrypt)
|   +-- client/       # Async WebDAV client SDK with optional C-FFI
|   +-- fuse/         # FUSE3 filesystem mount (Linux only)
|   +-- web/          # Leptos WASM web frontend
|   +-- cli/          # Admin CLI tool
|   +-- desktop/      # Tauri desktop application
+-- deploy/           # Deployment configurations (Docker, K8s, Terraform, etc.)
```

| Crate | Description |
|-------|-------------|
| `ferro-common` | Foundation types: `StorageEngine` trait, `FileMetadata`, `FerroError`, path utilities, WebDAV types |
| `ferro-core` | Production storage backends (SQLite, PostgreSQL, S3, GCS, Azure), Tantivy search, Wasmtime WASM runtime |
| `ferro-server` | Axum web server with all HTTP handlers: WebDAV, REST, GraphQL, WebSocket, CalDAV, CardDAV, WOPI, Federation |
| `ferro-dav` | iCalendar (RFC 5545) and vCard (RFC 6350) parsers, CalDAV/CardDAV store traits and handlers |
| `ferro-crypto` | `CryptoProvider` trait with Ring-based implementation: SHA-256/512, HMAC, bcrypt, secure random |
| `ferro-client` | Async WebDAV client with optional C-FFI bindings for mobile platforms (Swift/Kotlin) |
| `ferro-fuse` | FUSE3 filesystem mount translating POSIX operations to WebDAV HTTP requests |
| `ferro-web` | Leptos WASM web frontend for file browsing and upload |
| `ferro-cli` | Admin CLI tool for server management |
| `ferro-desktop` | Tauri desktop application with file browser and FUSE integration |

## Request Flow

```
HTTP Request
    |
    v
+------------------+
| Compression Layer|  (gzip, brotli)
+--------+---------+
         |
+--------+---------+
| Security Headers |  (HSTS, CSP, X-Content-Type-Options, X-Frame-Options)
+--------+---------+
         |
+--------+---------+
| Request Logging  |  (X-Request-ID, request counter)
+--------+---------+
         |
+--------+---------+
| Request ID       |  (assigns unique X-Request-ID header)
+--------+---------+
         |
+--------+---------+
| CORS Layer       |  (configurable origins, preflight handling)
+--------+---------+
         |
+--------+---------+
| Simple Auth      |  (HTTP Basic Auth if configured)
+--------+---------+
         |
+--------+---------+
| OIDC Auth        |  (PKCE flow if configured)
+--------+---------+
         |
+--------+---------+
| Cedar AuthZ      |  (policy-based authorization if configured)
+--------+---------+
         |
+--------+---------+
| Rate Limiter     |  (10,000 req/min per IP)
+--------+---------+
         |
+--------+---------+
|   Router         |  (Axum)
|  +------------+  |
|  | Handler(s) |  |  (WebDAV, REST, GraphQL, CalDAV, etc.)
|  +------------+  |
|  |  AppState   |  |
|  +------------+  |
+------------------+
         |
+--------+---------+
|  Storage Engine  |  (In-Memory, Local FS, S3, GCS, Azure)
+------------------+
```

The middleware stack processes requests in order. Each layer can short-circuit the request (e.g., rate limiter returns 429, auth returns 401).

## Storage Abstraction

All storage operations go through the `StorageEngine` trait defined in `ferro-common`:

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

This allows swapping backends without changing any server code. The `ObjectStoreStorageEngine` in `ferro-core` wraps the `object_store` crate to support S3, GCS, and Azure via a single implementation.

## Feature Flags

| Flag | Crate | Description |
|------|-------|-------------|
| `s3` | server, core | Amazon S3 storage backend |
| `gcs` | server, core | Google Cloud Storage backend |
| `azure` | server, core | Azure Blob Storage backend |
| `sqlite` | core | SQLite metadata store (default) |
| `search` | core | Tantivy full-text search (default) |
| `wasm` | core | Wasmtime WASM worker runtime (default) |
| `object_store` | core | object_store backend (default) |
| `postgres` | core, server | PostgreSQL metadata and state |
| `redis` | server | Redis distributed locking and rate limiting |
| `ldap` | server | LDAP authentication |
| `handlers` | dav | Axum handlers for CalDAV/CardDAV (default) |
| `persistence` | dav | SQLite persistence for calendar/address book stores |
| `ffi` | client | C-compatible FFI bindings for mobile |
| `ring` | crypto | Ring-based CryptoProvider (default) |
| `fips` | crypto | FIPS-approved mode (implies `ring`) |

```bash
# Build with all storage backends
cargo build --features s3,gcs,azure

# Build with PostgreSQL and Redis
cargo build --features pg,redis
```

## AppState

The central state object shared across all handlers:

| Field | Type | Description |
|-------|------|-------------|
| `storage` | `Arc<dyn StorageEngine>` | Storage backend |
| `metadata_store` | `Option<Arc<SqliteMetadataStore>>` | Persistent metadata |
| `search` | `Option<Arc<SearchEngine>>` | Full-text search engine |
| `wasm_runtime` | `Option<Arc<WasmWorkerRuntime>>` | WASM worker runtime |
| `cas_store` | `Option<Arc<CasStore>>` | Content-addressable store |
| `lock_manager` | `Arc<dyn LockManagerTrait>` | WebDAV lock manager |
| `share_store` | `Arc<ShareStore>` | Share link store |
| `audit_log` | `Arc<AuditLog>` | Audit log |
| `snapshot_store` | `Arc<SnapshotStore>` | Metadata snapshots |
| `ws_manager` | `Arc<WsManager>` | WebSocket manager |
| `activity_store` | `Arc<ActivityStore>` | Federation activity store |
| `cedar` | `Option<Arc<CedarAuthorizer>>` | Cedar policy engine |
| `oidc` | `Option<Arc<OidcValidator>>` | OIDC validator |
| `external_url` | `String` | Server's external URL |
| `federation_secret` | `String` | Federation HMAC secret |
