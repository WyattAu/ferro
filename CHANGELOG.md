# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.1] - 2026-05-30

### Security
- Fixed Cedar EntityUid parse failure falling back to `anonymous` (authorization bypass). Requests with malformed principal/action/resource identifiers are now denied.
- Fixed simple auth middleware granting admin access to disabled accounts. Inactive users with matching admin credentials are now rejected with 401.
- Fixed `AlreadyExists` error mapping from 405 Method Not Allowed to 409 Conflict per HTTP specification.

### Fixed
- `ContentHash::new()` no longer panics on invalid input -- returns `Option<Self>`. All callers updated.
- Audit chain hash now includes `user_agent` and `content_length` fields for complete tamper evidence coverage.
- SQLite metadata timestamp parsing now logs warnings when encountering malformed dates instead of silently defaulting to `Utc::now()`.
- MKCOL on an existing resource returns 405 Method Not Allowed per RFC 4918 Section 9.3.1.
- `LogBuffer::push` now uses `VecDeque` instead of `Vec` for O(1) front removal (was O(n)).
- CI audit workflow: removed duplicate `cargo-deny` install step.

## [2.5.1] - 2026-05-08

### Fixed
- Version alignment: all 20 crates now inherit `version.workspace = true` (was 0.1.0 / 1.0.0 / 2.2.0 mix)
- Import formatting: removed unnecessary braces in `ferro-server::policies`

### Added
- `rustfmt.toml` — codified formatting rules (imports granularity, comment width, etc.)
- `.clippy.toml` — cognitive complexity, struct size, argument thresholds
- `deny.toml` — cargo-deny config with documented ignores for desktop-only transitive advisories

### Changed
- CI audit job: hardened with `deny.toml` baseline for future cargo-deny integration
- Workspace `Cargo.toml`: added `version`, `rust-version` to `[workspace.package]`

## [2.5.0] - 2026-05-01

### Added
- **ferro-client** SDK crate — async WebDAV client with C-FFI for Swift/Kotlin mobile binding
- Chunked upload API (`POST /api/upload/init`, `PUT /api/upload/:id/chunk/:n`, `POST /api/upload/:id/complete`)
- mdBook documentation site (35 pages across 6 sections)
- Comprehensive SECURITY.md with penetration testing guide
- Connection pooling for federation delivery and webhook dispatch (static reqwest clients)
- Permissions-Policy header (camera, microphone, geolocation, payment disabled)
- Cargo.toml publish metadata for all 5 library crates

### Changed
- Hardened Content-Security-Policy headers (blob: in img-src, ws:/wss: in connect-src)
- Hardened Tauri CSP (asset protocol, IPC connections)
- Security headers extracted to dedicated module (`security_headers.rs`)

## [2.4.0] - 2026-04-30

### Added
- CalDAV REPORT: calendar-query with VEVENT/VTODO time-range filtering (RFC 4791)
- CardDAV REPORT: addressbook-query with case-insensitive text-match filtering (RFC 6352)
- CTag change tracking on all calendar/addressbook write mutations
- FUSE offline cache (SQLite metadata + SHA-256 content-addressable blob storage)
- Write-through/read-through caching with offline write queue
- Content deduplication in FUSE cache (same content shares blob)
- Tauri desktop file browser (single-file HTML, 1055 lines, no framework)
  - Directory tree sidebar, file list with sorting
  - Drag-and-drop upload, keyboard shortcuts, context menus
  - Dark theme via prefers-color-scheme
  - Connect dialog with localStorage persistence
- WebDAV integration tests (10 tests against live axum router)
- Publish-ready README.md for all 5 library crates

### Fixed
- XML parser: self-closing tag handling in calendar-query time-range
- XML parser: namespace-prefix-agnostic text-match extraction

## [2.3.0] - 2026-04-29

### Added
- FUSE filesystem mount (crates/fuse/) — real read/write/create/mkdir/unlink/rmdir/rename
- Inode table and file handle table with access mode tracking
- WebDAV sync-token delta sync (AtomicU64 sync_clock)
- CalDAV/CardDAV persistence (rusqlite bundled, 4 SQLite tables)
- Composable TOML config layering (recursive include with cycle detection)
- GitHub Actions release CI (6 binary targets + Docker push + GitHub Release)

### Changed
- Modular crate architecture: feature-gated APIs for ferro-core, ferro-dav, ferro-crypto
- Zero circular dependencies, zero doc warnings

## [2.2.0] - 2026-04-28

### Added
- SQLite persistence (rusqlite bundled, 15 tables, WAL mode, write-through caching)
- 11 stores persisted: users, shares, favorites, webhooks, trash, tags, sync, federation
- Docker Compose (4 variants: base, PostgreSQL, Redis, full)
- Podman rootless deployment (security hardened, systemd integration)
- Firecracker microVM launcher (2 vCPU, 512MB)
- K3s single manifest (Traefik ingress, PVC)
- Terraform K3s provider module
- HTTP signature enforcement on federation inbox

## [2.1.0] - 2026-04-27

### Added
- Zero production unwrap() (10 remaining all safe: 6 WASM browser, 2 guarded, 1 Tauri, 1 string)
- HTTP Signature verification (draft-cavage-http-signatures-12)
- ActivityPub HTTP delivery to follower inboxes (fire-and-forget)
- Batch file operations (`POST /api/batch/copy|move`)
- File tagging API (TagStore, 50 tags/file)
- Request idempotency store (TTL-based, 100K cap)
- Storage health monitoring (`GET /api/health/storage`)
- WebSocket real-time notifications (7 event types)
- Configurable CORS (`--cors-origins`)

### Fixed
- DashMap retain deadlock in federation store eviction

## [2.0.0] - 2026-04-26

### Added
- ActivityPub federation (9 activity types, ActivityStore)
- E2E encryption (age crate, passphrase-based)
- GraphQL API (async-graphql v7)
- CRDT real-time sync (VectorClock, SyncStore, SSE events, delta sync)
- WebRTC signaling
- Tauri desktop system tray
- FIPS crypto abstraction (ferro-crypto crate)

## [1.1.0] - 2026-04-25

### Added
- File streaming (range requests)
- CalDAV/CardDAV (ferro-dav crate, iCal/vCard parsers)
- AGPL-3.0-or-later license
- Playwright E2E CI
- Thumbnails (image + PDF)
- Trash auto-purge
- Quota enforcement
- S3 multipart upload
- File diff view
- Browser notifications

## [1.0.0] - 2026-04-24

### Added
- Initial release
- WebDAV server with full PROPFIND/GET/PUT/DELETE/MKCOL/COPY/MOVE
- REST API for files, users, shares, favorites, preferences, locks, versions, trash
- Full-text search (tantivy)
- WASM plugin runtime (wasmtime)
- Object store backends (S3, GCS, Azure Blob)
- Leptos web UI (CSR, dark mode, command palette, grid view, search filters, file preview)
- Authentication (simple token + bcrypt + OIDC)
- Authorization (Cedar policy engine)
- Rate limiting (token bucket)
- JSON logging
- Health probes (liveness + readiness)
- Backup/restore
- Webhooks
- Multi-user with roles (admin/editor/viewer)

[2.5.1]: https://github.com/WyattAu/ferro/compare/v2.5.0...v2.5.1
[2.5.0]: https://github.com/WyattAu/ferro/compare/v2.4.0...v2.5.0
[2.4.0]: https://github.com/WyattAu/ferro/compare/v2.3.0...v2.4.0
[2.3.0]: https://github.com/WyattAu/ferro/compare/v2.2.0...v2.3.0
[2.2.0]: https://github.com/WyattAu/ferro/compare/v2.1.0...v2.2.0
[2.1.0]: https://github.com/WyattAu/ferro/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/WyattAu/ferro/compare/v1.1.0...v2.0.0
[1.1.0]: https://github.com/WyattAu/ferro/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/WyattAu/ferro/releases/tag/v1.0.0
