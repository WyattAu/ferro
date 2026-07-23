# Ferro

Self-hosted storage orchestrator. Rust. WebDAV. Content-addressable. WASM-extendable.

[![CI](https://github.com/WyattAu/ferro/actions/workflows/ci.yml/badge.svg)](https://github.com/WyattAu/ferro/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)

## Architecture

73-crate Rust workspace (edition 2024, MSRV 1.92, toolchain 1.95.0). Axum web framework. SQLite WAL for persistence. Content-addressable storage with SHA-256 deduplication.

```
ferro-server       Axum binary — WebDAV, REST, CalDAV, CardDAV, WOPI
ferro-core         Storage backends (memory, local, S3, GCS, Azure)
ferro-common       Shared types, traits, error handling
ferro-crypto       SHA-256, HMAC, password hashing, age encryption
ferro-auth         OIDC (PKCE), Cedar authorization, API keys, WebAuthn
ferro-dav          WebDAV RFC 4918 — PROPFIND, MKCOL, PUT, LOCK, COPY, MOVE
ferro-caldav       CalDAV RFC 4791, CardDAV RFC 6352
ferro-web          Leptos WASM frontend — file browser, search, sharing
ferro-desktop      Tauri v2 desktop client (Linux, macOS, Windows)
ferro-cli          Admin CLI — user management, config, health
ferro-distributed  Erasure coding, Raft consensus, geo-replication
```

Full crate index: [CONTRIBUTING.md](CONTRIBUTING.md)

## Features

| Category | Capabilities |
|----------|-------------|
| Storage | Content-addressable (SHA-256 dedup), snapshots, versioning, trash with TTL |
| Access | WebDAV Class 1/2/3, REST API v1, CalDAV, CardDAV, WOPI (Collabora/OnlyOffice) |
| Auth | OIDC (PKCE), Cedar policies, API keys, TOTP/HOTP, WebAuthn, LDAP |
| Search | Tantivy full-text with auto-indexing, saved queries, smart collections |
| Processing | WASM workers — resize, convert, transcode, OCR, watermark |
| Sharing | Public links (password/expiry), QR codes, file requests, federated sharing |
| Collaboration | CRDT real-time editing, chat, comments, tags |
| Compliance | WORM, retention policies, DLP, antivirus scanning, audit logging |
| Sync | Bidirectional sync, offline-first, selective sync profiles |
| Observability | Prometheus metrics, health probes, structured logging |
| Desktop | Tauri v2 — native file browser, FUSE mount, keyboard shortcuts |
| Mobile | Tauri v2 — iOS/Android with push notifications (FCM/APNS) |
| Federation | ActivityPub cross-instance federation |
| Multi-tenant | Tenant isolation, per-user quotas, guest accounts |

## Quick Start

### Binary

```bash
curl -sL https://github.com/WyattAu/ferro/releases/latest/download/ferro-server-linux -o ferro-server
chmod +x ferro-server
./ferro-server --port 8080 --data-dir /var/lib/ferro
```

### Docker

```bash
docker compose up -d
```

Bundled image includes Leptos web UI + Caddy reverse proxy with automatic HTTPS.

### From Source

```bash
git clone https://github.com/WyattAu/ferro.git && cd ferro
cargo build --release --bin ferro-server
./target/release/ferro-server --data-dir ./data
```

## Configuration

Priority: CLI flags > environment variables > `ferro.toml` > defaults.

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--port` | — | `8080` | Server port |
| `--storage` | — | `memory` | Backend: `memory`, `local:/path`, `s3://bucket`, `gs://bucket`, `az://container` |
| `--data-dir` | `FERRO_DATA_DIR` | — | Persistence directory (SQLite WAL, CAS, snapshots, audit) |
| `--oidc-issuer` | `FERRO_OIDC_ISSUER` | — | OIDC issuer URL (enables auth) |
| `--oidc-client-id` | `FERRO_OIDC_CLIENT_ID` | — | OIDC client ID |
| `--admin-user` | `FERRO_ADMIN_USER` | — | HTTP Basic Auth username |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | — | HTTP Basic Auth password |
| `--config` | `FERRO_CONFIG` | — | TOML config file path |

Full reference: [docs/configuration.html](https://wyattau.github.io/ferro/configuration.html)

### Storage Backends

| Backend | Build | Env Vars |
|---------|-------|----------|
| Local filesystem | default | — |
| Amazon S3 | `--features s3` | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` |
| Google Cloud Storage | `--features gcs` | `GOOGLE_APPLICATION_CREDENTIALS` |
| Azure Blob Storage | `--features azure` | `AZURE_STORAGE_ACCOUNT_NAME`, `AZURE_STORAGE_ACCOUNT_KEY` |

## API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/.well-known/ferro` | GET | Health check |
| `/api/v1/search?q=...` | GET | Full-text search |
| `/api/v1/shares` | GET/POST | Share link management |
| `/api/v1/snapshots` | GET/POST | Snapshot management |
| `/api/v1/audit` | GET | Audit log |
| `/wopi/files/*` | GET/POST | WOPI protocol |
| `/*path` | WebDAV | WebDAV collection |

Full reference: [docs/api/rest.html](https://wyattau.github.io/ferro/api/rest.html)

## Development

Prerequisites: Rust 1.92+, OpenSSL (for PostgreSQL support).

```bash
cargo build --all              # Build all crates
cargo test --workspace         # Run all tests
cargo fmt --all -- --check     # Format check
cargo clippy --workspace --all-targets -- -D warnings  # Lint
```

### Feature Flags

| Flag | Crate | Description |
|------|-------|-------------|
| `s3` | ferro-core | Amazon S3 backend |
| `gcs` | ferro-core | Google Cloud Storage backend |
| `azure` | ferro-core | Azure Blob Storage backend |
| `pg` | ferro-core | PostgreSQL metadata store |
| `redis` | ferro-server-infra | Redis distributed locks |
| `ldap` | ferro-auth | LDAP authentication |
| `screenshot` | ferro-desktop | Native screenshot capture |

### Nix

```bash
nix develop           # Full dev environment
nix develop .#web     # WASM build environment
nix develop .#desktop # Tauri desktop environment
```

## Deployment

### Docker Compose (Production)

```yaml
services:
  ferro:
    build: .
    volumes:
      - ferro-data:/data
    environment:
      - FERRO_DATA_DIR=/data
      - FERRO_OIDC_ISSUER=https://keycloak.example.com/realms/ferro
      - FERRO_OIDC_CLIENT_ID=ferro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:8080/.well-known/ferro"]
      interval: 30s
      timeout: 5s
      retries: 3

  caddy:
    image: caddy:2-alpine
    ports: ["80:80", "443:443"]
    volumes: ["./Caddyfile:/etc/caddy/Caddyfile:ro"]
    depends_on: [ferro]

volumes:
  ferro-data:
```

### Kubernetes

```bash
kubectl apply -f deploy/k8s/
```

### Ecosystem

```bash
cd deploy && docker compose -f docker-compose.ecosystem.yml up -d
```

Packages: ferro-server, ferro-web, ferro-admin, ferro-cli, ferro-fuse, ferro-observability.

## CI/CD

13 GitHub Actions workflows:

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| ci.yml | push/PR | fmt, clippy, tests (6 feature combos), PostgreSQL, security scan |
| quality.yml | push/PR | machete, semver-checks, miri, fuzz, mutation testing |
| security.yml | daily + push | cargo-deny, sanitizers (ASan/TSan/Miri), Trivy |
| extended-checks.yml | push/PR | Playwright E2E, code coverage (85% minimum) |
| release.yml | tags | Cross-platform build, SBOM, Docker multi-arch, Cosign signing |
| desktop.yml | push/PR | Tauri builds (Linux, macOS, Windows, Android) |

## Documentation

Deployed at [wyattau.github.io/ferro](https://wyattau.github.io/ferro):

- [Introduction](https://wyattau.github.io/ferro/introduction.html)
- [Quick Start](https://wyattau.github.io/ferro/quickstart.html)
- [Configuration](https://wyattau.github.io/ferro/configuration.html)
- [Deployment](https://wyattau.github.io/ferro/deployment/docker.html)
- [API Reference](https://wyattau.github.io/ferro/api/rest.html)
- [Security](https://wyattau.github.io/ferro/security.html)

## License

AGPL-3.0-or-later
