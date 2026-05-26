# Ferro v2.0.0 -- Federation, Encryption, Real-Time Sync

> **Historical document.** This release note covers v2.0.0 (2026-04-26).
> Current version: **v2.5.1** (2026-05-08). See [CHANGELOG.md](./CHANGELOG.md) for v2.1.0+ changes.

## What is Ferro?

Ferro is a self-hosted file storage orchestrator written in 100% Rust. Think Nextcloud, but fast, lightweight, and without the PHP. It speaks WebDAV natively, so it works with every file manager on every operating system.

## What's New in v2.0.0

This is a major release adding **ActivityPub federation**, **end-to-end encryption**, **real-time sync**, **GraphQL API**, **CalDAV/CardDAV**, **WebRTC P2P signaling**, and a **Tauri desktop app**.

### ActivityPub Federation
- **Full inbox/outbox processing** — Create, Update, Delete, Announce, Follow, Accept, Reject, Like, Undo
- **Follower management** — Auto-accept Follow activities, track followers/following
- **Federated sharing** — `POST /api/fed/share` delivers shares to followers
- **WebFinger discovery** — `/.well-known/webfinger` for actor resolution
- **Actor profiles** — `GET /fed/actor/:user` with preferredUsername, inbox, outbox

### End-to-End Encryption
- **age-based encryption** — Passphrase-based, ASCII-armored output
- **Encrypt/decrypt files** — `POST /api/files/encrypt`, `POST /api/files/decrypt`
- **Detection** — Automatic recognition of age-encrypted content

### Real-Time Sync (CRDTs)
- **Vector clocks** — Monotonic ordering with merge support
- **Operation log** — Append-only DashMap store, bounded 100k operations
- **SSE events** — `GET /api/sync/events` for real-time change notifications
- **Delta sync** — `GET /api/sync/delta?since=N` for incremental updates
- **Auto-recording** — WebDAV PUT/DELETE/MOVE/MKCOL operations recorded automatically

### GraphQL API
- **Full schema** — Query: files, file, shares, me, health, audit_log, versions
- **Mutations** — create_folder, delete_file
- **GraphiQL playground** — Interactive IDE at `/api/graphql`

### CalDAV & CardDAV
- **iCalendar parser** — RFC 5545 compliant (VEVENT, VTODO, VTIMEZONE)
- **vCard parser** — RFC 6350 compliant (FN, N, TEL, EMAIL, ADR, PHOTO)
- **CalDAV endpoints** — `/dav/cal/` with PROPFIND, REPORT, GET, PUT, DELETE, MKCALENDAR
- **CardDAV endpoints** — `/dav/card/` with PROPFIND, REPORT, GET, PUT, DELETE

### WebRTC P2P Signaling
- **Offer/answer exchange** — `POST /api/webrtc/offer/create`, `/submit-answer`
- **ICE candidate relay** — `/add-ice`, `/poll-answer`
- **TTL-based cleanup** — Offers expire after 5 minutes

### Tauri Desktop App
- **System tray** — Show/hide, quit from tray menu
- **4 plugins** — shell, dialog, fs, notification
- **Dual mode** — CLI (default) or GUI (via `tauri` feature flag)
- **Mobile scaffold** — `MobileConfig` for Android/iOS (via `mobile` feature flag)

### FIPS Crypto Abstraction
- **`ferro-crypto` crate** — `CryptoProvider` trait for swappable backends
- **Ring provider** — SHA-256/512, HMAC-SHA256, random bytes, password hashing
- **`fips` feature flag** — Informational FIPS mode flag

### File Streaming & Multipart Upload
- **True streaming** — `get_stream()` returns `AsyncRead` for zero-copy download
- **S3 multipart** — `put_multipart()` for files >10MB on object storage backends
- **WebDAV streaming** — GET and share download use `Body::from_stream()`

### Thumbnails, Trash, Quota
- **Image thumbnails** — JPEG, PNG, GIF, WebP via `image` crate with disk caching
- **PDF thumbnails** — Page count + metadata extraction, styled SVG
- **Trash auto-purge** — Background hourly task, configurable TTL (default 30d)
- **Quota enforcement** — Pre-check on PUT via Content-Length, 413 on overflow

### File Diff
- **LCS-based diff** — `GET /api/files/{path}/diff?from=v1&to=v2`
- **10k line cap** — Binary detection, unified diff format

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
- **File streaming** — True AsyncRead streaming for large files

### Federation & Sync
- **ActivityPub** — Full inbox/outbox with 9 activity types, auto-follow, federated shares
- **Real-time sync** — CRDT vector clocks, SSE events, delta sync API
- **WebRTC signaling** — P2P offer/answer/ICE exchange server

### Security
- **E2E encryption** — age-based file encryption with passphrase
- **FIPS crypto** — Swappable `CryptoProvider` trait (ring default)
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

### APIs & Protocols
- **WebDAV** — RFC 4918 Class 1/2/3
- **ActivityPub** — Full actor + inbox + outbox
- **CalDAV** — Calendar access (RFC 4791)
- **CardDAV** — Address book access (RFC 6352)
- **GraphQL** — Full schema with playground
- **REST API** — Users, shares, locks, audit, versions, encryption, sync

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

### Desktop & Mobile
- **Tauri desktop app** — System tray, native dialogs, notifications
- **Mobile scaffold** — Android/iOS config module (via `mobile` feature flag)

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
| `--thumbnail-size` | `FERRO_THUMBNAIL_SIZE` | `256` | Thumbnail dimension (64-1024) |
| `--trash-ttl` | `FERRO_TRASH_TTL` | `30d` | Trash auto-purge TTL |
| `--storage-quota` | `FERRO_STORAGE_QUOTA` | none | Storage quota (e.g., 10GB) |
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
| `tauri` | tauri, tauri-plugin-* | Desktop GUI mode |
| `mobile` | tauri, tauri-plugin-http | Mobile app config |
| `fips` | ring, bcrypt | FIPS crypto mode (informational) |

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

## Quality

- **882 tests** -- 0 failures across all feature combinations (counts as of v2.5.1)
- **0 clippy warnings** -- `-D warnings` across all feature configurations
- **Zero undocumented public items** -- `cargo doc` reports 0 warnings
- **Zero unsafe blocks** -- in production code (FFI in client crate only)
- **0 `todo!()` / `unimplemented!()`** -- no stubs in production code
- **20 crates** -- common, core, server, web, cli, desktop, dav, crypto, auth, admin, observability, graphql, client, fuse, benchmarks, webdav-handler, server-versioning, server-activitypub, server-webrtc, server-wopi
- **4 fuzz harnesses** -- cargo-fuzz with 2.6M+ iterations, 0 crashes
- **25 property tests** -- proptest for storage, path, lock, and XML parsers
- **49 E2E tests** -- Playwright across 7 spec files (chromium, firefox, webkit)
- **3 load tests** -- k6 concurrent upload, large directory, soak test

## Contributing

Bug reports, feature requests, and pull requests welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

AGPL-3.0-or-later
