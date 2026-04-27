# Ferro v1.0.0-beta.2 — Production-Hardened Release

## What is Ferro?

Ferro is a self-hosted file storage orchestrator written in 100% Rust. Think Nextcloud, but fast, lightweight, and without the PHP. It speaks WebDAV natively, so it works with every file manager on every operating system.

## What's New in beta.2

This release focuses on **production readiness** — every HIGH, MEDIUM, LOW, and INFO severity finding from a comprehensive security/quality audit has been resolved.

### Security Hardening
- **bcrypt password hashing** (cost=12) — replaces weak SHA-256 with static salt
- **Constant-time password comparison** via `subtle::ConstantTimeEq` in auth
- **WOPI token enforcement** — rejects token issuance with default secret (500 error)
- **Bounded in-memory collections** — audit log (10k), trash (1k), shares (10k), webhooks (100), favorites (10k), users (10k), recently-processed (10k)

### Multi-Instance Support
- **PostgreSQL backend** (`--features pg`) — shares, favorites, preferences in PG
- **Redis distributed locks** (`--features redis`) — Lua atomic scripts, TTL
- **Redis rate limiter** — INCR+EXPIRE sliding window
- **LDAP authentication** (`--features ldap`) — auto-provision, 5s/10s timeouts
- **Trait abstractions** — `LockManagerTrait`, `ShareStoreTrait`, `UserStoreTrait` for pluggable backends

### Kubernetes & Infrastructure
- **K8s manifests** — 14 files: namespace, deployment (security context, probes, PDB), service, ingress, configmap, secret, PVC, 4 NetworkPolicies
- **Helm chart** — full values.yaml (replicas, resources, persistence, auth, CORS, ingress, serviceMonitor)
- **Terraform module** — deploys via Helm with configurable variables
- **Liveness/readiness probes** — `/healthz`, `/readyz` with storage reachability check
- **Docker compose overlays** — `docker-compose.pg.yml`, `docker-compose.redis.yml`

### Observability
- **Prometheus metrics** — `/metrics/prometheus` (text/plain, gauge/counter format)
- **JSON structured logging** — `--log-format json|text`
- **Backup/restore API** — point-in-time, idempotent, disk-backed manifests
- **Webhook notifications** — HMAC-SHA256 signed, async delivery, 3x retry
- **Request counter** — per-endpoint Prometheus counter

### Multi-User & Versioning
- **User management** — CRUD API, roles (Admin/User/ReadOnly), per-user quotas, home directories
- **File versioning** — auto-version on PUT overwrite, max N versions (configurable), list/download/delete API
- **Lock cleanup** — background task sweeps expired locks every 60 seconds

### Performance
- **Lock-free rate limiter** — DashMap replaces RwLock<HashMap>, no global lock per request
- **Efficient audit log** — VecDeque replaces Vec for O(k) front-drain on eviction
- **Graceful shutdown** — SIGTERM+SIGINT with configurable timeout

### Quality
- **363 tests** — 0 failures across all feature combinations
- **0 clippy warnings** — `-D warnings` across 4 feature configurations
- **Zero undocumented public items** — `cargo doc` reports 0 warnings
- **Zero unsafe blocks** — in production code (OIDC test helpers only)
- **Zero production panics** — all `.unwrap()` replaced with `expect()` or error handling
- **Release build** — compiles in ~5 minutes, ~39MB server binary

## Quick Start

### Docker (single-node)
```bash
git clone https://github.com/WyattAu/ferro.git
cd ferro
docker compose up -d
# Open http://localhost:8080/ui/
```

### Docker (PostgreSQL + Redis)
```bash
docker compose -f docker-compose.yml -f docker-compose.pg.yml -f docker-compose.redis.yml up -d
```

### Kubernetes (Helm)
```bash
cd deploy/helm/ferro
helm install ferro . --set auth.adminPassword=changeme
```

### From Source
```bash
cargo build --release -p ferro-server -p ferro-cli
./target/release/ferro-server --data-dir /path/to/data --static-dir crates/web/dist
```

## Features

### Core
- **WebDAV Class 1/2/3** — Full RFC 4918 compliance with LOCK/UNLOCK, depth:infinity
- **Multiple storage backends** — Local filesystem, S3, GCS, Azure Blob (feature flags)
- **Content-addressable dedup** — Same file stored once, referenced by SHA-256 hash
- **Full-text search** — Tantivy-powered, auto-indexed on upload
- **WASM worker runtime** — Run custom logic on file events, sandboxed with fuel + memory limits
- **File versioning** — Auto-version on overwrite, configurable max versions

### Security
- **bcrypt password hashing** — Cost factor 12 with per-user salts
- **HTTP Basic Auth** — Simple `--admin-user`/`--admin-password` for personal deployments
- **OIDC authentication** — PKCE flow with Keycloak, Auth0, Google, etc.
- **LDAP authentication** — Active Directory / OpenLDAP with auto-provision
- **Cedar authorization** — Policy-based access control (enterprise)
- **Security headers** — CSP, X-Frame-Options, HSTS, nosniff
- **Path traversal protection** — All file paths sanitized
- **Rate limiting** — Lock-free per-IP, configurable window
- **WOPI protocol** — Collabora/WPS Office integration with HMAC-signed tokens
- **Constant-time comparisons** — Password and secret comparisons timing-safe

### Web UI
- **File browser** — List/grid views, upload, download, create folders, drag-and-drop
- **Share links** — Password-protected (constant-time), time-limited public URLs
- **Multi-user** — Admin panel, user management, role-based access
- **File preview** — Inline preview for images, text, PDF, video, audio
- **Search** — Type/sort/mime/date/size filters, debounced input
- **Dark mode** — System preference detection, manual toggle, localStorage
- **Command palette** — Ctrl+K with 13 commands, keyboard navigation
- **Onboarding tour** — 6-step first-run walkthrough
- **Responsive** — Desktop and mobile layouts

### Operations
- **Kubernetes ready** — Helm chart, K8s manifests, Terraform module
- **Docker support** — Multi-stage build with bundled web UI
- **Prometheus metrics** — `/metrics/prometheus` endpoint
- **JSON logging** — `--log-format json` for structured log aggregation
- **Backup/restore** — Point-in-time backup API with disk manifests
- **Webhook notifications** — HMAC-SHA256 signed, async with retry
- **Graceful shutdown** — SIGTERM/SIGINT with configurable drain timeout
- **Health probes** — Liveness, readiness, startup probes for orchestration
- **Config file** — `ferro.toml` for persistent configuration
- **Audit logging** — All operations logged, bounded in-memory + optional SQLite

## Configuration

All options via CLI flags, environment variables, or `ferro.toml`:

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--host` | `FERRO_HOST` | `0.0.0.0` | Bind address |
| `--port` | `FERRO_PORT` | `8080` | Bind port |
| `--data-dir` | `FERRO_DATA_DIR` | none | Persistent storage directory |
| `--static-dir` | `FERRO_STATIC_DIR` | none | Web UI static files |
| `--admin-user` | `FERRO_ADMIN_USER` | none | Admin username (enables auth) |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | none | Admin password |
| `--log-level` | `FERRO_LOG_LEVEL` | `info` | Log level |
| `--log-format` | `FERRO_LOG_FORMAT` | `text` | Log format (text/json) |
| `--max-body-size` | `FERRO_MAX_BODY_SIZE` | `1GB` | Max upload size |
| `--config` | `FERRO_CONFIG` | none | Config file path |
| `--storage` | `FERRO_STORAGE` | `memory` | Storage backend |
| `--external-url` | `FERRO_EXTERNAL_URL` | none | Public URL (for OIDC/shares) |
| `--graceful-shutdown-timeout` | `FERRO_GRACEFUL_SHUTDOWN_TIMEOUT` | `30s` | Shutdown drain timeout |
| `--max-file-versions` | `FERRO_MAX_FILE_VERSIONS` | `10` | Max versions per file |
| `--database-url` | `FERRO_DATABASE_URL` | none | PostgreSQL URL (pg feature) |
| `--redis-url` | `FERRO_REDIS_URL` | none | Redis URL (redis feature) |

## Feature Flags

| Flag | Dependencies | Description |
|------|-------------|-------------|
| `s3` | aws-config, object_store | Amazon S3 backend |
| `gcs` | aws-config, object_store | Google Cloud Storage backend |
| `azure` | object_store | Azure Blob Storage backend |
| `pg` | sqlx, postgres | PostgreSQL for shares/favorites/preferences |
| `redis` | redis | Distributed locks + rate limiting |
| `ldap` | ldap3 | LDAP/Active Directory authentication |

## WebDAV Clients

Ferro works with any WebDAV client:
- **macOS**: Finder → Connect to Server → `http://localhost:8080/`
- **Windows**: Map Network Drive → `http://localhost:8080/`
- **Linux**: Nautilus/GIO → `dav://localhost:8080/`
- **rclone**: `rclone lsf :webdav,url=http://localhost:8080/`
- **Cyberduck**: WebDAV connection at `http://localhost:8080/`

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting. This release has:
- 0 known vulnerabilities in Ferro's own code
- 4 documented transitive advisories (no fix available upstream)
- OWASP Top 10 compliance checklist in `docs/security/`
- STRIDE threat model in `docs/security/threat-model.md`
- Full quality audit in `.reports/final_quality_audit.md`

## Contributing

Bug reports, feature requests, and pull requests welcome. See the existing code style (363 tests, 0 clippy warnings) for contribution guidelines.

## License

AGPL-3.0-or-later
