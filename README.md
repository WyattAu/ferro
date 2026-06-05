# Ferro

100% Rust storage orchestrator — a self-hosted alternative to Nextcloud.

[![CI](https://github.com/WyattAu/ferro/actions/workflows/checks.yml/badge.svg)](https://github.com/WyattAu/ferro/actions/workflows/checks.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)

## Overview

Ferro is a high-performance, self-hosted file storage platform built entirely in Rust. 
It provides WebDAV-compatible file access, content-addressable storage with deduplication, 
OIDC authentication, full-text search, WASM-based file processing, and WOPI protocol 
support for online document editing.

## Features

- **WebDAV Server** — Full Class 1/2/3 compliance (PROPFIND, MKCOL, PUT, GET, DELETE, COPY, MOVE, LOCK, UNLOCK, PROPPATCH)
- **Multiple Storage Backends** — In-memory, local filesystem, S3, GCS, Azure Blob Storage
- **Content-Addressable Storage** — SHA-256 deduplication, saves space automatically
- **OIDC Authentication** — PKCE login flow with Keycloak, Auth0, Google, etc.
- **Cedar Authorization** — Fine-grained policy-based access control
- **Full-Text Search** — Tantivy-powered search with auto-indexing
- **WASM Workers** — Run custom file processing pipelines (resize, convert, transform)
- **WOPI Protocol** — Online document editing via Collabora/OnlyOffice
- **Share Links** — Public file sharing with optional passwords and expiration
- **Audit Logging** — Track all file operations
- **Metadata Snapshots** — Point-in-time recovery for ransomware protection
- **Rate Limiting** — Per-IP token-bucket rate limiter
- **Web UI** — Modern Leptos-based file browser with drag-and-drop upload
- **Admin CLI** — Full command-line management tool
- **Docker Support** — Multi-stage Dockerfile with docker-compose
- **Nix Flakes** — Reproducible development environments

## Quick Start

### Binary

```bash
# Download from GitHub Releases
curl -sL https://github.com/WyattAu/ferro/releases/latest/download/ferro-server-linux -o ferro-server
chmod +x ferro-server

# Start the server
./ferro-server --port 8080
```

### Docker

```bash
docker compose up -d
```

The Docker image includes the bundled Leptos web UI and a Caddy reverse proxy with automatic HTTPS. See the [Docker Compose](#docker-compose) section below for details.

### From Source

```bash
git clone https://github.com/WyattAu/ferro.git
cd ferro
cargo build --release --bin ferro-server
./target/release/ferro-server
```

## Configuration

### CLI Flags

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--host` | — | `0.0.0.0` | Bind address |
| `-p, --port` | — | `8080` | Server port |
| `--log-level` | — | `info` | Log level (trace, debug, info, warn, error) |
| `--log-format` | — | `text` | Log format (`text`, `json`) |
| `--storage` | — | `memory` | Storage backend (`memory`, `local:/path`, `s3://bucket`, `gs://bucket`, `az://container`) |
| `--data-dir` | `FERRO_DATA_DIR` | (none) | Persistent data directory (enables SQLite metadata, CAS, snapshots, audit) |
| `--static-dir` | `FERRO_STATIC_DIR` | (none) | Web UI static files directory |
| `--max-body-size` | `FERRO_MAX_BODY_SIZE` | `1073741824` | Max request body size in bytes (default: 1 GB) |
| `--wasm-enabled` | `FERRO_WASM_ENABLED` | `false` | Enable WASM worker runtime |
| `--cas-enabled` | — | `false` | Enable in-memory content-addressable deduplication |
| `--search-index-path` | — | (auto) | Search index directory (defaults to `{data-dir}/search-index`) |
| `--metadata-db` | `FERRO_METADATA_DB` | (none) | PostgreSQL metadata database URL |
| `--oidc-issuer` | `FERRO_OIDC_ISSUER` | (none) | OIDC issuer URL (enables authentication) |
| `--oidc-client-id` | `FERRO_OIDC_CLIENT_ID` | (none) | OIDC client ID |
| `--oidc-audience` | `FERRO_OIDC_AUDIENCE` | `ferro` | OIDC audience claim |
| `--oidc-jwks-uri` | `FERRO_OIDC_JWKS_URI` | (none) | JWKS URI (overrides auto-discovery) |
| `--cedar-policy-file` | `FERRO_CEDAR_POLICY_FILE` | (none) | Path to Cedar policy file |
| `--admin-user` | `FERRO_ADMIN_USER` | (none) | Admin username for simple HTTP Basic Auth |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | (none) | Admin password for simple HTTP Basic Auth |
| `--config` | `FERRO_CONFIG` | (none) | Path to configuration file (TOML format) |
| `--external-url` | `FERRO_EXTERNAL_URL` | `http://localhost:8080` | External base URL for OIDC callbacks |
| `--wopi-token-secret` | `FERRO_WOPI_TOKEN_SECRET` | (none) | HMAC secret for WOPI tokens |
| `--wopi-office-url` | `FERRO_WOPI_OFFICE_URL` | (none) | Collabora/OnlyOffice server URL for WOPI |
| `--storage-quota` | — | (none) | Per-user storage quota in bytes |
| `--trash-ttl` | — | `604800` | Trash auto-purge TTL in seconds (default: 7 days) |
| `--graceful-shutdown-timeout` | — | `30` | Graceful shutdown timeout in seconds |
| `--maintenance-mode` | — | `false` | Enable maintenance mode (503 for all requests) |
| `--cors-allowed-origins` | — | (none) | CORS allowed origins (comma-separated, `*` for all) |
| `--api-version` | — | `1` | API version prefix (`1` = `/api/v1/`) |
| `--max-file-versions` | — | `0` | Maximum file versions to keep (0 = unlimited) |
| `--thumbnail-size` | — | `256` | Thumbnail size in pixels |
| `--multi-user` | — | `false` | Enable multi-user mode |
| `--migrate-from` | — | (none) | Migrate data from another storage backend |
| `--federation-secret` | `FERRO_FEDERATION_SECRET` | (none) | Secret for ActivityPub federation HTTP signatures |

### Storage Backends

#### In-Memory (default)
```bash
ferro-server
```

#### Local Filesystem
```bash
ferro-server --storage local:/path/to/files
```

#### Amazon S3
```bash
cargo build --release --features s3 --bin ferro-server
./target/release/ferro-server --storage s3://my-bucket
# Requires: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION
```

#### Google Cloud Storage
```bash
cargo build --release --features gcs --bin ferro-server
./target/release/ferro-server --storage gs://my-bucket
# Requires: GOOGLE_APPLICATION_CREDENTIALS
```

#### Azure Blob Storage
```bash
cargo build --release --features azure --bin ferro-server
./target/release/ferro-server --storage az://my-container
# Requires: AZURE_STORAGE_ACCOUNT_NAME, AZURE_STORAGE_ACCOUNT_KEY
```

### Persistent Data

Use `--data-dir` to enable SQLite-backed persistence for metadata, CAS deduplication, snapshots, and audit logs in a single database:

```bash
ferro-server --data-dir /var/lib/ferro
```

This creates `/var/lib/ferro/ferro.db` with WAL mode for concurrent access. CAS deduplication is automatically enabled when `--data-dir` is set.

> **Note:** A startup warning is displayed when `--data-dir` is not set — all data will be lost on restart.

### Configuration File

Ferro can load settings from a `ferro.toml` file:

```bash
ferro-server --config /path/to/ferro.toml
```

Auto-discovery searches the current directory, then `/etc/ferro/ferro.toml`. CLI flags and environment variables override file values.

Example `ferro.toml`:

```toml
host = "0.0.0.0"
port = 8080
storage = "local:/data/files"
data_dir = "/var/lib/ferro"
admin_user = "admin"
admin_password = "changeme"
log_level = "info"
```

### Simple Auth

For quick setups without an OIDC provider, use HTTP Basic Auth:

```bash
ferro-server --admin-user admin --admin-password secret
```

Or via environment variables:

```bash
FERRO_ADMIN_USER=admin FERRO_ADMIN_PASSWORD=secret ferro-server
```

When simple auth is enabled, the `/api/auth/info` endpoint returns `"auth_type": "basic"`.

### OIDC Authentication

```bash
ferro-server \
  --oidc-issuer https://keycloak.example.com/realms/myrealm \
  --oidc-client-id ferro
```

When OIDC is enabled, Cedar authorization is automatically activated with a default permissive policy. Supply `--cedar-policy-file` to load custom policies.

### Docker Compose

The bundled Docker image includes the Leptos web UI and a Caddy reverse proxy with automatic HTTPS.

```yaml
services:
  ferro:
    build: .
    expose:
      - "8080"
    volumes:
      - ferro-data:/data
    environment:
      - FERRO_DATA_DIR=/data
      - FERRO_STATIC_DIR=/app/ui
      - FERRO_ADMIN_USER=admin
      - FERRO_ADMIN_PASSWORD=changeme
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:8080/.well-known/ferro"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

  caddy:
    image: caddy:2-alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
    environment:
      - DOMAIN=localhost
    depends_on:
      - ferro
    restart: unless-stopped

volumes:
  ferro-data:
    driver: local
  caddy-data:
  caddy-config:
```

## WebDAV Access

Ferro provides a fully compliant WebDAV server:

```bash
# Mount with rclone
rclone mount http://localhost:8080 /mnt/ferro --vfs-cache-mode full

# Or any WebDAV client
curl -X PROPFIND http://localhost:8080/ -H "Depth: 1"
```

See [docs/webdav.md](docs/webdav.md) for detailed client setup instructions.

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/.well-known/ferro` | GET | Health check |
| `/api/config` | GET | Server configuration and capabilities |
| `/api/auth/info` | GET | Current user info (includes `auth_type`: `none`, `basic`, or `oidc`) |
| `/api/auth/login` | GET | OIDC login URL (PKCE) |
| `/api/auth/callback` | GET | OIDC callback |
| `/api/search` | GET | Full-text search (`?q=...&limit=N`) |
| `/api/workers` | GET/POST | WASM worker management |
| `/api/workers/upload` | POST | Upload WASM module |
| `/api/workers/modules` | GET | List uploaded WASM modules |
| `/api/workers/modules/{filename}` | DELETE | Delete WASM module |
| `/api/policies` | GET/POST/DELETE | Cedar policy management |
| `/api/upload-url` | GET | Generate pre-signed upload URL |
| `/api/download-url` | GET | Generate pre-signed download URL |
| `/api/shares` | GET/POST | Share link management |
| `/api/shares/:token` | DELETE | Delete share link |
| `/s/:token` | GET | Public share download |
| `/api/audit` | GET | Audit log |
| `/api/storage/stats` | GET | Storage statistics |
| `/api/snapshots` | GET/POST | Snapshot management |
| `/api/snapshots/:id` | DELETE | Delete snapshot |
| `/api/snapshots/:id/restore` | POST | Restore snapshot |
| `/wopi/files/*path` | GET/POST | WOPI protocol (CheckFileInfo, GetFile, PutFile, Lock, Unlock) |
| `/wopi/files/{path}/token` | POST | Issue WOPI access token |
| `/hosting/discovery` | GET | WOPI discovery document |
| `/ui/*` | GET | Web UI (requires `--static-dir`) |
| `/*path` | WebDAV | WebDAV collection (all methods) |

See [docs/api.md](docs/api.md) for detailed request/response documentation.

## Development

### Prerequisites
- Rust 1.92+ (edition 2024)
- OpenSSL (for PostgreSQL support)

### Build
```bash
cargo build --all
cargo test --all
```

### Feature Flags

| Flag | Description |
|------|-------------|
| `s3` | Amazon S3 storage backend |
| `gcs` | Google Cloud Storage backend |
| `azure` | Azure Blob Storage backend |

```bash
cargo build --features s3,gcs,azure
```

### Nix
```bash
nix develop           # Full dev environment
nix develop .#web     # WASM build environment
nix develop .#desktop # Tauri desktop environment
```

### Web Frontend
```bash
cd crates/web
trunk build --release
```

## Ecosystem Packages

Ferro is a collection of independently deployable packages:

| Package | Binary | Purpose |
|---|---|---|
| ferro-server | `ferro-server` | Core storage server with WebDAV, API, and metrics endpoints |
| ferro-web | static files (nginx) | User-facing file browser |
| ferro-admin | static files (nginx) | Admin panel for system management |
| ferro-cli | `ferro-cli` | Admin CLI tool |
| ferro-fuse | `ferro-fuse` | FUSE filesystem mount |
| ferro-observability | endpoints in ferro-server | `/metrics` and `/healthz` endpoints for monitoring |

### Deploy the Full Ecosystem

```bash
cd deploy
docker compose -f docker-compose.ecosystem.yml up -d
```

See [deploy/README-ecosystem.md](deploy/README-ecosystem.md) for all deployment options, environment variables, and port mappings.

## Architecture

Ferro is built as a Rust workspace with 43 crates:

| Crate | Description |
|-------|-------------|
| `ferro-common` | Shared types, StorageEngine trait, error handling |
| `ferro-core` | Storage backends, search engine, WASM runtime, CAS dedup |
| `ferro-server` | Axum web server, WebDAV, REST API, admin endpoints |
| `ferro-dav` | WebDAV protocol implementation (RFC 4918) |
| `ferro-webdav-handler` | WebDAV XML request/response parsing |
| `ferro-auth` | Authentication (OIDC, simple auth) and Cedar authorization |
| `ferro-crypto` | Cryptographic primitives (SHA, HMAC, password hashing) |
| `ferro-observability` | Metrics, health checks, Prometheus export |
| `ferro-web` | Leptos WASM web frontend |
| `ferro-cli` | Admin CLI tool |
| `ferro-desktop` | Tauri desktop application |
| `ferro-fuse` | FUSE filesystem mount |
| `ferro-client` | Rust client SDK with C-FFI |
| `ferro-admin` | Admin dashboard (Leptos) |
| `ferro-graphql` | GraphQL API layer |
| `ferro-server-versioning` | File versioning and auto-versioning |
| `ferro-server-activitypub` | ActivityPub federation |
| `ferro-server-webrtc` | WebRTC signaling |
| `ferro-server-wopi` | WOPI protocol (Office Online) |
| `ferro-benchmarks` | Criterion benchmark suite |

## Documentation

Full documentation is deployed at [wyattau.github.io/ferro](https://wyattau.github.io/ferro):

- [Introduction](https://wyattau.github.io/ferro/introduction.html)
- [Quick Start](https://wyattau.github.io/ferro/quickstart.html)
- [Configuration Reference](https://wyattau.github.io/ferro/configuration.html)
- [Deployment Guide](https://wyattau.github.io/ferro/deployment/docker.html)
- [API Reference](https://wyattau.github.io/ferro/api/rest.html)
- [Security](https://wyattau.github.io/ferro/security.html)

## License

AGPL-3.0-or-later
