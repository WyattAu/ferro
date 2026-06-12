# Getting Started

## What is Ferro?

Ferro is a high-performance, self-hosted file storage platform built entirely in Rust. It provides WebDAV-compatible file access, content-addressable storage with automatic deduplication, OIDC authentication, full-text search, WASM-based file processing, and WOPI protocol support for online document editing. Ferro is designed as a storage orchestrator -- it sits between your files and the storage backends (local filesystem, S3, GCS, Azure Blob), providing a unified API for accessing, searching, sharing, and processing files regardless of where they are stored.

Ferro includes a full protocol stack: WebDAV (Class 1/2/3), CalDAV, CardDAV, ActivityPub federation, and a modern Leptos-based web UI. It supports end-to-end encryption via age, CRDT-based collaborative editing, FUSE mounts for native filesystem access, and a Tauri desktop application. With 43 modular crates, fine-grained Cedar authorization, and a flexible plugin architecture, Ferro scales from a single-user NAS replacement to a multi-tenant file platform serving thousands of concurrent users.

## Quick Install

### Docker (recommended)

```bash
docker compose up -d
```

The server is available at `http://localhost:8080`. The Docker image (`ghcr.io/wyattau/ferro:latest`) includes the bundled Leptos web UI and a Caddy reverse proxy with automatic HTTPS.

### Manual Build

```bash
git clone https://github.com/WyattAu/ferro.git
cd ferro
cargo build --release --bin ferro-server
./target/release/ferro-server --port 8080
```

### CLI Install

```bash
# Download pre-built binary
curl -sL https://github.com/WyattAu/ferro/releases/latest/download/ferro-server-linux -o ferro-server
chmod +x ferro-server
./ferro-server --port 8080
```

## First Steps

### Upload a file

```bash
curl -X PUT http://localhost:8080/hello.txt \
  -H "Content-Type: text/plain" \
  -d "Hello, Ferro!"
```

### Download a file

```bash
curl http://localhost:8080/hello.txt
```

### Create a folder

```bash
curl -X MKCOL http://localhost:8080/documents/
```

### List files

```bash
curl -X PROPFIND http://localhost:8080/ \
  -H "Depth: 1" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0" encoding="utf-8"?>
       <D:propfind xmlns:D="DAV:">
         <D:prop>
           <D:resourcetype/>
           <D:getcontentlength/>
           <D:getlastmodified/>
         </D:prop>
       </D:propfind>'
```

### Connect with a WebDAV client

- **rclone**: Configure with `vendor: Other`, URL: `http://localhost:8080`
- **macOS Finder**: Go > Connect to Server > `http://localhost:8080/`
- **Windows Explorer**: Map network drive > `http://localhost:8080/`
- **Linux (Nautilus)**: Other Locations > `dav://localhost:8080/`

### Health check

```bash
curl http://localhost:8080/.well-known/ferro
```

## Key Features

- [**WebDAV Server**](./api/webdav.md) -- Full Class 1/2/3 compliance with locking and sync-token support
- [**Multiple Storage Backends**](./architecture.md#storage-abstraction) -- In-memory, local filesystem, S3, GCS, Azure Blob Storage
- [**Content-Addressable Storage**](./architecture.md#storage-architecture) -- SHA-256 deduplication saves space automatically
- [**OIDC Authentication**](./security.md) -- PKCE login flow with Keycloak, Auth0, Google, or any OIDC provider
- [**Cedar Authorization**](./security.md) -- Fine-grained policy-based access control
- [**Full-Text Search**](./api/rest.md#search) -- Tantivy-powered search with auto-indexing
- [**WASM Workers**](./architecture.md) -- Custom file processing pipelines (resize, convert, transform)
- [**ActivityPub Federation**](./api/federation.md) -- Share files across Ferro instances using the fediverse
- [**End-to-End Encryption**](./guides/encryption.md) -- age-based file encryption (X25519, ChaCha20-Poly1305)
- [**FUSE Mount**](./guides/fuse-mount.md) -- Access remote files as a local directory on Linux

## System Requirements

### Minimum

| Resource | Requirement |
|----------|-------------|
| CPU | 1 core |
| RAM | 128 MB |
| Disk | 50 MB (binary) |
| OS | Linux, macOS, Windows |

### Recommended (production)

| Resource | Requirement |
|----------|-------------|
| CPU | 2+ cores |
| RAM | 512 MB |
| Disk | Depends on storage backend |
| OS | Linux (kernel 5.4+) |

### Runtime Dependencies

| Dependency | Required | Purpose |
|------------|----------|---------|
| OpenSSL | If using PostgreSQL | TLS for database connections |
| FUSE kernel module | For ferro-fuse | Filesystem mount support |

## Next Steps

- [Configuration](./configuration.md) -- Customize your server (CLI flags, environment variables, TOML config)
- [Architecture](./architecture-overview.md) -- Understand how Ferro works under the hood
- [Deployment](./deployment/SUMMARY.md) -- Docker, Kubernetes, and bare metal deployment guides
- [API Reference](./api-reference.md) -- Full REST, GraphQL, WebSocket, and WebDAV API docs
- [Security](./security.md) -- Authentication, encryption, and security features
- [Guides](./guides/SUMMARY.md) -- Desktop app, FUSE mount, CalDAV clients, and more
- [Contributing](../../CONTRIBUTING.md) -- Help improve Ferro
