# Ferro

100% Rust storage orchestrator — a self-hosted alternative to Nextcloud.

[![CI](https://github.com/WyattAu/ferro/actions/workflows/ci.yml/badge.svg)](https://github.com/WyattAu/ferro/actions/workflows/ci.yml)
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
- **Unified Type System** — Single-source-of-truth types (DbHandle, ApiError, AuditEntry) across 57 crates
- **Push Notifications** — FCM (Android) and APNS (iOS) delivery pipeline
- **API Federation** — ActivityPub-based cross-instance federation
- **Compliance Documentation** — SOC 2, ISO 27001, HIPAA, PCI DSS, FedRAMP mapping
- **Web UI** — Modern Leptos-based file browser with drag-and-drop upload
- **Admin CLI** — Full command-line management tool
- **Admin Dashboard** — Leptos-based admin panel with user management, storage stats, audit log
- **GDPR Compliance** — Data export (ZIP) and verified data erasure endpoints
- **Docker Support** — Multi-stage Dockerfile with docker-compose
- **Nix Flakes** — Reproducible development environments
- **14 Themes** — Light, Dark, Midnight, System, Solarized Light, Solarized Dark, Nord, Tokyo Night, Dracula, High Contrast, Sepia, Forest, Ocean, Custom
- **ZIP Download** — Download multiple files as a single ZIP archive
- **Duplicate Files** — Server-side file duplication with a single command
- **Saved Searches** — Persistent search queries with custom view presets (Default, Compact, Detailed, Media)
- **PWA Support** — Progressive Web App with offline capabilities
- **File Requests** — Upload-only share links with message and expiration metadata
- **QR Code Sharing** — Generate QR codes for instant share link access
- **Group Management** — Create and manage user groups with member lists
- **EPUB Preview** — In-browser EPUB reader with navigation and metadata display
- **Background Audio Player** — Persistent audio player with play/pause, volume, seek, and playlist support
- **Slideshow Mode** — Configurable image slideshow with interval presets (3s, 5s, 10s, 30s)
- **Photo Map View** — Geographic map display for geotagged photos
- **Photo Editing** — In-browser image adjustments (brightness, contrast, saturation, crop)
- **Video Transcoding** — Server-side video format conversion via WASM workers
- **Block-Based Editor** — Notion-style block editor with drag-and-drop reordering
- **Graph View** — Force-directed graph visualization of file relationships
- **Dual-Pane Mode** — Side-by-side file browsing for efficient file management
- **Workflow Automation** — Event-triggered workflows with conditions and actions
- **Smart Collections** — Rule-based dynamic file groupings with auto-update support
- **Custom Folder Views** — Saved view configurations with filters, sorting, and grouping
- **Admin Compliance Tools** — WORM, retention policies, antivirus scanning, DLP enforcement
- **Remote Wipe** — Device-level data erasure for lost or compromised devices

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

The Docker image includes the bundled Leptos web UI and a Caddy reverse proxy with automatic HTTPS.

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
| `--validate-config` | — | `false` | Validate configuration file and exit (exit code 0 = valid) |
| `--generate-completions` | — | (none) | Generate shell completion script and exit |
| `--print-man-page` | — | `false` | Print man page to stdout and exit |
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
| `--api-version` | `FERRO_API_VERSION` | `v1` | API version prefix (`v1` = `/api/v1/`) |
| `--max-file-versions` | `FERRO_MAX_FILE_VERSIONS` | `10` | Maximum file versions to keep per file (0 = disabled) |
| `--thumbnail-size` | `FERRO_THUMBNAIL_SIZE` | `256` | Maximum thumbnail dimension in pixels (64–1024) |
| `--thumbnail-cache-size` | `FERRO_THUMBNAIL_CACHE_SIZE` | `104857600` | Maximum thumbnail cache size in bytes (default: 100 MB) |
| `--multi-user` | `FERRO_MULTI_USER` | `false` | Enable multi-user mode with per-user home directories |
| `--migrate-from` | `FERRO_MIGRATE_FROM` | (none) | Migrate data from another storage backend |
| `--dedup-enabled` | — | `false` | Enable perceptual deduplication on upload |
| `--streaming-upload-threshold` | `FERRO_STREAMING_UPLOAD_THRESHOLD` | `65536` | Content-Length threshold (bytes) for streaming uploads |
| `--retention-check-interval` | `FERRO_RETENTION_CHECK_INTERVAL` | `3600` | Retention policy check interval in seconds (0 = disabled) |
| `--guest-cleanup-interval` | `FERRO_GUEST_CLEANUP_INTERVAL` | `300` | Guest account cleanup interval in seconds (0 = disabled) |
| `--federation-secret` | `FERRO_FEDERATION_SECRET` | (none) | Secret for ActivityPub federation HTTP signatures |
| `--federation-trusted-peers` | `FERRO_FEDERATION_TRUSTED_PEERS` | (none) | Comma-separated trusted federation peer URLs |
| `--smtp-host` | `FERRO_SMTP_HOST` | (none) | Email notification SMTP host |
| `--smtp-port` | `FERRO_SMTP_PORT` | (none) | Email notification SMTP port |
| `--smtp-username` | `FERRO_SMTP_USERNAME` | (none) | Email notification SMTP username |
| `--smtp-password` | `FERRO_SMTP_PASSWORD` | (none) | Email notification SMTP password |
| `--email-from` | `FERRO_EMAIL_FROM` | `noreply@ferro.local` | Email notification from address |
| `--email-from-name` | `FERRO_EMAIL_FROM_NAME` | `Ferro` | Email notification from name |
| `--ldap-url` | `FERRO_LDAP_URL` | (none) | LDAP server URL (enables LDAP auth, requires `ldap` feature) |
| `--ldap-bind-dn` | `FERRO_LDAP_BIND_DN` | (none) | LDAP bind DN for service account |
| `--ldap-bind-password` | `FERRO_LDAP_BIND_PASSWORD` | (none) | LDAP service account password |
| `--ldap-user-search-base` | `FERRO_LDAP_USER_SEARCH_BASE` | (empty) | LDAP user search base DN |
| `--sync-nodes` | `FERRO_SYNC_NODES` | (none) | Comma-separated peer node addresses for cross-node sync |
| `--sync-interval` | `FERRO_SYNC_INTERVAL` | `300` | Sync scan interval in seconds (0 = on-demand only) |
| `--sync-mode` | `FERRO_SYNC_MODE` | `bidirectional` | Sync mode: `push`, `pull`, or `bidirectional` |
| `--offline-cache-dir` | `FERRO_OFFLINE_CACHE_DIR` | (none) | Directory for offline content cache (enables offline-first mode) |
| `--offline-queue-size` | `FERRO_OFFLINE_QUEUE_SIZE` | `50000` | Maximum pending offline queue operations |
| `--fcm-server-key` | `FERRO_FCM_SERVER_KEY` | (none) | Firebase Cloud Messaging server key for Android push notifications |
| `--apns-key-path` | `FERRO_APNS_KEY_PATH` | (none) | Path to APNS private key file (.p8) for iOS push notifications |
| `--apns-team-id` | `FERRO_APNS_TEAM_ID` | (none) | Apple Developer Team ID for APNS |

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
- Rust 1.92+ (edition 2024, pinned in `rust-toolchain.toml`)
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

Ferro is built as a Rust workspace with 46 crates:

**Core**

| Crate | Description |
|-------|-------------|
| `ferro-common` | Shared types, StorageEngine trait, error handling |
| `ferro-core` | Storage backends, search engine, WASM runtime, CAS dedup |
| `ferro-crypto` | Cryptographic primitives (SHA, HMAC, password hashing) |
| `ferro-auth` | Authentication (OIDC, simple auth) and Cedar authorization |
| `ferro-dav` | WebDAV protocol implementation (RFC 4918) |
| `ferro-caldav` | CalDAV/CardDAV protocol implementation (RFC 4791/6352) |
| `ferro-scim` | SCIM 2.0 user/group provisioning |

**Server**

| Crate | Description |
|-------|-------------|
| `ferro-server` | Axum web server, WebDAV, REST API, admin endpoints |
| `ferro-server-activitypub` | ActivityPub federation |
| `ferro-server-webrtc` | WebRTC signaling |
| `ferro-server-wopi` | WOPI protocol (Office Online) |
| `ferro-server-versioning` | File versioning and auto-versioning |

**Server Sub-crates**

| Crate | Description |
|-------|-------------|
| `ferro-server-admin` | Server-side admin API handlers |
| `ferro-server-automation` | Server automation and workflow engine |
| `ferro-server-security` | Server security policies and enforcement |
| `ferro-server-sharing` | File and folder sharing logic |
| `ferro-server-webdav` | Server-side WebDAV handler integration |

**Web & Admin**

| Crate | Description |
|-------|-------------|
| `ferro-web` | Leptos WASM web frontend |
| `ferro-admin` | Admin dashboard (Leptos) |

**Desktop & Mobile**

| Crate | Description |
|-------|-------------|
| `ferro-desktop` | Tauri desktop application |
| `ferro-mobile` | Tauri v2 mobile bindings (iOS/Android) |
| `ferro-client` | Rust client SDK with C-FFI |
| `ferro-fuse` | FUSE filesystem mount |
| `ferro-mount-nfs` | NFS mount support |

**Sync & Collab**

| Crate | Description |
|-------|-------------|
| `ferro-crdt` | CRDT-based collaborative data structures |
| `ferro-sync-protocol` | Sync protocol for multi-node replication |
| `ferro-offline` | Offline-first sync and local queue |
| `ferro-selective-sync` | Selective file sync policies |

**Infrastructure**

| Crate | Description |
|-------|-------------|
| `ferro-observability` | Metrics, health checks, Prometheus export |
| `ferro-event-bus` | Internal event bus for decoupled messaging |
| `ferro-rate-limiter` | Per-IP token-bucket rate limiting |
| `ferro-cache` | Caching layer for metadata and content |
| `ferro-health` | Health check and readiness probes |
| `ferro-audit-log` | Audit logging for file operations |
| `ferro-webhook` | Outgoing webhook delivery |
| `ferro-backend-router` | Storage backend routing and selection |
| `ferro-consistent-hash` | Consistent hashing for distributed nodes |
| `ferro-wasm-host` | WASM runtime host for file processing |

**AI & Graph**

| Crate | Description |
|-------|-------------|
| `ferro-ai` | AI integration and smart features |
| `ferro-graphql` | GraphQL API layer |

**Distributed**

| Crate | Description |
|-------|-------------|
| `ferro-distributed` | Distributed storage and consensus |
| `ferro-multi-tenant` | Multi-tenant isolation and management |

**Tools**

| Crate | Description |
|-------|-------------|
| `ferro-cli` | Admin CLI tool |
| `ferro-benchmarks` | Criterion benchmark suite |
| `ferro-migrate` | Data migration utilities |
| `ferro-webdav-handler` | WebDAV XML request/response parsing |

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
