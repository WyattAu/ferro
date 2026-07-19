# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Multi-theme system** (14 themes): Solarized Light/Dark, Nord, Tokyo Night, Dracula, High Contrast, Sepia, Forest, Ocean, Custom. All themes use 60+ CSS custom properties.
- **Lock-free storage**: InMemoryStorageEngine, InMemoryCasStore, InMemoryMetadataStore now use DashMap instead of async RwLock. Sub-microsecond latency (get: 270ns, exists: 121ns).
- **ServerState trait** (88 methods): Consolidated state interface for all handlers. Implemented for AppState.
- **FIPS validation**: Runtime self-test for SHA-256, HMAC-SHA-256, HKDF-SHA-256, RNG health.
- **Key hierarchy**: 3-level key management (Master -> KEK -> Data Keys) with wrapping, rotation, and destruction.
- **Circuit breakers, retry, bulkheads**: Architectural resilience patterns in `server-resilience` crate.
- **NIST SP 800-53 mapping**: 27 controls documented across 7 families (AC, AU, CM, IA, MP, SC, SI).
- **CC EAL 3 preparation**: 11 evidence packages covering all assurance families.
- **PWA support**: Service worker, manifest, offline fallback.
- **10 new crates**: server-config, server-state, server-routes, server-api, server-health, server-federation, server-sync-handlers, server-resilience, server-slo, server-fips.
- **Integration tests**: 69 new tests for all new features.
- **Property tests**: 3 new tests for normalize_path and ContentHash.
- **Fuzz targets**: 3 new targets for crypto, FIPS, circuit breaker.
- **CI matrix**: 24 test configurations (12 individual + 12 combinations).

### Changed
- **Type unification**: DbHandle (19->1), ApiError (9->2), AuditEntry (9->1).
- **Router routes**: All prefixed with `/ui/` to match browser URL path.
- **Metadata store**: Converted from async RwLock to DashMap.
- **ThumbnailService**: Consolidated (dead code removed).
- **Frontend**: 929 color replacements, 0 hardcoded colors, 0 static inline styles.
- **A11y**: Toggle buttons labeled, radio inputs labeled, skip-link added.

### Fixed
- **Blank screen bug**: Router routes not matching `/ui/` URL path.
- **MIME type errors**: Service worker not copied to dist/ directory.
- **Memory leak**: `mem::forget(dir)` on TempDir in ferro-core tests.
- **WASM build errors**: MediaSession API, Theme variants, Closure imports.
- **Clippy warnings**: All resolved across entire workspace.

### Removed
- **server-admin** crate (dead code)
- **webhook** crate (unused)
- **server-storage-utils** crate (merged into server-storage-ops)
- **server-webdav** crate (dead code, 3800+ lines)
- **server/src/thumbnails.rs** (dead code)

## [3.2.0] - 2026-07-16

### Added
- **ZIP download** (`POST /api/v1/zip-download`): Download multiple files as a single ZIP archive in one request.
- **File duplication** (`POST /api/v1/duplicate`): Server-side file duplication without re-uploading content.
- **File requests** (`POST/GET/DELETE /api/v1/file-requests`): Upload-only share links with message and expiration metadata. Users can request specific files and uploaders can fulfill without sign-up.
- **QR code sharing** (`GET /api/v1/shares/:token/qr`): Generate QR codes for instant share link access from mobile devices.
- **Group management** (`POST/GET/PUT/DELETE /api/v1/groups`, `POST/DELETE /api/v1/groups/:id/members/:username`): Create and manage user groups with member add/remove operations.
- **Smart collections** (`POST/GET/PUT/DELETE /api/v1/smart-collections`, `GET /api/v1/smart-collections/:id/files`): Rule-based dynamic file groupings with auto-update support based on configurable criteria.
- **Workflow automation** (`POST/GET/PUT/DELETE /api/v1/workflows`, `POST /api/v1/workflows/:id/trigger`): Event-triggered workflows with conditions and actions. Supports manual triggering via API.
- **Video transcoding** (`POST /api/v1/transcode`, `GET /api/v1/transcode/:id/status`, `GET /api/v1/transcode`): Server-side video format conversion via WASM workers with job status polling.
- **Saved searches** (`GET/POST /api/v1/saved-searches`, `GET/PUT/DELETE /api/v1/saved-searches/:id`): Persistent search queries with custom view presets (Default, Compact, Detailed, Media).
- **Admin compliance tools** (`GET /api/v1/admin/compliance/summary`, `GET /api/v1/admin/compliance/data-retention`): WORM, retention policies, antivirus scanning, and DLP enforcement status and management.
- **Remote wipe** (`GET /api/v1/wipe-status`, `POST /api/v1/wipe-confirm`): Device-level data erasure for lost or compromised devices with confirmation flow.

## [3.1.0] - 2026-07-07

### Added
- **Circuit breaker pattern** (`ferro-circuit-breaker` crate): Three-state (Closed/Open/HalfOpen) circuit breaker for external dependency protection. Wired into Redis, ClamAV, and remote mount operations.
- **OpenTelemetry distributed tracing** (`otel` feature flag): End-to-end trace export to Jaeger via gRPC/OTLP. Configurable endpoint and service name via `--otlp-endpoint` and `--otel-service-name` CLI flags. Instrumented spans on health, auth, file operations, and WebDAV handlers.
- **Deep health probes**: Health endpoint now checks SQLite connectivity, storage backend reachability, and Redis availability with per-subsystem status, duration, and error messages.
- **DAVx5 CalDAV/CardDAV sync-collection**: Implemented sync-collection REPORT method per RFC 6578. Enables bidirectional sync with DAVx5 Android client and other CalDAV clients. Handles sync-token based incremental sync and full resync fallback.
- **REST API integration test suite** (26 tests): Full CRUD coverage for file operations, batch copy/move/delete, WebDAV PROPFIND/MKCOL/PUT/GET/DELETE, auth endpoints, and error handling.
- **Server lifecycle integration tests** (4 tests): Server startup, health check, WebDAV round-trip, auth rejection, and concurrent request deadlock detection.
- **k6 load testing benchmarks** (`benchmarks/k6/`): WebDAV and REST API load test scripts with configurable auth, thresholds, and ramp stages.
- **Productivity crate tests** (70 tests): Task and note CRUD operations, edge cases, error paths, and mock store infrastructure.
- **Compliance crate tests** (117 tests): Retention policies, WORM storage, antivirus scanning, DLP policies with full CRUD coverage.
- **Sharing crate tests** (86 tests): Share creation, expiry, password hashing, constant-time comparison, lockout, and download tracking.
- **Admin API tests** (84 tests): User management, audit logging, GDPR, backup integrity, storage stats.
- **Mutation tests** (26 new): ferro-auth (17 tests covering API key lifecycle, OIDC PKCE, SAML fingerprint, TOTP/HOTP RFC 4226 vectors) and ferro-server-storage-ops (9 tests covering dedup hashing, range headers, quota parsing).

### Changed
- Architecture decomposition: `main.rs` 54 lines (was 1,578), `lib.rs` 227 lines (was 2,340), `state.rs` 412 lines (was 1,822), `webdav.rs` 213 lines (was 1,776).
- Server now rejects CORS wildcard (`*`) when authentication is enabled (security hardening).
- Benchmark regression CI gate tightened from 120% to 105% threshold with `fail-on-alert: true`.
- Code coverage CI threshold raised from 80% to 85%.
- CI quality pipeline expanded: fuzz testing (10s per target), mutation testing (cargo-mutants), security scanning (cargo-machete, semver-checks, Miri, gitleaks).
- 35 unused dependencies removed across 16 crates.
- Workspace coverage: ~45% -> ~58% overall (estimated).
- Total test count: 885 (was 496).

### Fixed
- Circuit breaker deadlock: `parking_lot::Mutex` reentrant lock in `state()` and `record_success()` — wrapped first lock in block scope to drop before re-lock.
- Benchmark CI regression detection: tightened from 120% to 105% alert threshold, set to fail on alert.
- k6 load test scripts: fixed auth header encoding and health endpoint path.

### Fixed (Cycle 16 - WASM Frontend & Static Serving)
- Fixed static file serving: WebDAV catch-all route at `/` was intercepting requests before static files could be served. Now skips WebDAV catch-all when `--static-dir` is set.
- SPA middleware correctly handles `/` (serve index.html), `/ui/*` (static files with SPA fallback), and falls through to API/WebDAV for other paths.
- Rebuilt Leptos WASM frontend with `trunk build` (16MB debug binary).
- Added `e2e/quick-capture.js` Playwright script for viewport traversal.
- Added `scripts/traverse.sh` for one-command server + traversal execution.

### Verified (Cycle 16 - Playwright DOM/Screenshot Traversal)
- Desktop (1280x720): 165 elements, 17 buttons, 3 inputs, 0 console errors. Full file browser UI.
- Mobile (390x844): 165 elements, same UI with touch-friendly layout.
- Tablet (768x1024): 165 elements, same UI with tablet layout.
- All API endpoints exercised: /api/config, /api/branding, /api/quota, /api/preferences, /api/favorites, /api/locks, /api/recent.
- 0 accessibility issues (no unlabeled buttons, no images without alt).

### Changed (Cycle 15 - Full Tauri App Implementation)
- All 12 mobile commands fully implemented with real WebDAV/REST API calls (previously all stubs):
  mobile_get_file_thumbnail (image preview), mobile_get_storage_stats (real quota),
  mobile_start/stop_background_sync (tokio task with 5-min interval), mobile_get_offline_files
  (cache scan), mobile_pin/unpin_file_offline (download + manifest), mobile_get_sync_status
  (state tracking), mobile_resolve_conflict (KeepLocal/KeepRemote/KeepBoth), mobile_share_file
  (server share API), mobile_monitor_connectivity (HEAD polling), mobile_register_push_notifications
  (server registration).
- Config persistence: DesktopConfig saved to `~/.config/ferro/desktop.json` (Linux),
  `~/Library/Application Support/ferro/desktop.json` (macOS), `%APPDATA%\ferro\desktop.json` (Windows).
  Loaded on startup if no CLI args provided.
- IosFilesProvider and AndroidSAFProvider now use real HEAD/PROPFIND requests instead of empty data.
- Frontend: localStorage restore validates connection before navigating (prevents broken UI on server down).
- Frontend: Added Settings view (server URL, auth token, mount point, sync interval, theme toggle).
- Frontend: Added Recent files view (sorted by mtime) and Favorites view (localStorage + context menu).
- Frontend: loadTree() changed from Depth:infinity to lazy loading (Depth:1 + expand on click).
- test_connection now parses actual server name from PROPFIND XML.
- get_server_url returns real stored URL instead of hardcoded localhost.
- cmd_get_mount_progress now returns real progress from RcloneManager.
- Sync commands (start/stop/pause/resume/sync_now/get_status) registered in invoke_handler.
- Fixed app.share() compile error for android+tauri feature combination.
- Removed dead duplicate commands from tauri_commands.rs (kept only sync commands).
- Fixed CSS variable references, console.log typo, mobile nav button wiring.

### Added (Cycle 14 - Mobile iOS/Android via Tauri v2)
- Tauri v2 iOS/Android mobile bundle config in `tauri.conf.json` (iOS minimum 14.0, Android minSdkVersion 24).
- `crates/desktop/capabilities/mobile.json`: Mobile-specific permissions capability.
- `crates/desktop/src/mobile_commands.rs`: 12 mobile-specific Tauri commands (thumbnail, storage stats, background sync, offline pinning, conflict resolution, push notifications, connectivity monitoring, sharing). 23 tests.
- Mobile-responsive frontend: CSS media queries, touch events (long-press, swipe, pull-to-refresh), mobile bottom navigation bar, hamburger menu, 44px touch targets.
- `crates/desktop/MOBILE.md`: Build documentation for iOS and Android targets.
- `scripts/build-mobile.sh`: One-command build script for `android` and `ios` targets.
- `crates/desktop/src/lib.rs`: `run_mobile()` entry point with mobile-specific Tauri plugin setup.

### Added (Cycle 13 - Production Readiness)
- `ferro-migrate` crate: Nextcloud-to-Ferro migration tool with WebDAV file streaming, SQLite DB reader for users/shares/tags/favorites, progress tracking, and CLI subcommand `ferro migrate nextcloud`.
- `deploy/docker-compose.production.yml`: 7-service production stack (Ferro + PostgreSQL + Redis + Caddy + Prometheus + Grafana + Alertmanager) with auto-provisioned dashboards and health checks.
- `deploy/Caddyfile`: Reverse proxy with TLS, compression, security headers, WebSocket support.
- `deploy/.env.example`: All configurable variables for production deployment.
- `crates/server/tests/soak_test.rs`: Configurable-duration soak test with 50 concurrent users, mixed workload (PUT/GET/PROPFIND/DELETE/MOVE/COPY), latency percentiles, and JSON results output.
- `scripts/soak-test.sh`: One-command soak test runner (default 1h, configurable to 24h).
- `crates/server/tests/webdav_litmus.rs`: WebDAV RFC 4918 compliance test suite (22 tests across Class 1/2/3).
- `crates/server/tests/multi_user.rs`: Multi-user scenario tests (24 tests) covering sharing, concurrent edits, permission enforcement, stress testing.
- `crates/server/tests/disaster_recovery.rs`: Backup/restore disaster recovery drill (13 tests) with full cycle verification.
- `crates/server/tests/collab_integration.rs`: CRDT collab E2E tests (6 tests) covering two-client relay, concurrent convergence, state persistence.
- Enhanced `rclone_e2e.rs` with 9 new tests: sync, move, check, large files, special characters, concurrent operations.
- Selective sync wired into server: 5 API endpoints (GET/POST/PUT/DELETE /sync/profiles, POST /sync/filter-preview) and client methods.
- Plugin marketplace admin frontend: Leptos 0.8 component with search/filter, install/uninstall/enable/disable, detail modal. Server API stubs with mock plugins.
- Search relevance tuning: configurable boost factors (file name 3x, path 2x, recency 1.2x), normalized 0-100 scores, highlights, match locations, admin API for tuning and reindexing.
- Collab editor E2E: server-side CRDT document state per room, periodic persistence, frontend reconnection with exponential backoff, offline buffer.

### Added (Cycle 12.2 - Server Decomposition)
- `ferro-server-webdav` crate: WebDAV handler, locking, MOVE/COPY, range GET extracted from server (14 tests).
- `ferro-server-security` crate: Security module, ClamAV, ransomware detection, encryption, E2EE, API keys, TOTP, WebAuthn (58 tests).
- `ferro-server-sharing` crate: Shares, public links, guests, comments, tags, favorites, federation sync (50 tests).
- `ferro-server-admin` crate: Admin API, user management, branding, tenant rate limiting, GDPR, LDAP, backup, metrics, quota (8 tests).
- `ferro-server-automation` crate: Event triggers, webhooks, push notifications, retention, WORM, batch ops, OCR (13 tests).
- `GET /health` endpoint wired to ferro-health HealthChecker with memory probe.
- ferro-health HealthChecker added to AppState.

### Changed (Cycle 12.2)
- Server crate decomposed from 115 files into 5 focused sub-crates + server core.
- Crate count: 38 -> 42 (net +4 after removing 4 dead crates, adding 5 new sub-crates, removing 3 integration-only crates).
- Webhook/audit-log/backend-router crates removed from server deps (server inline implementations are more complete).
- ROADMAP: TD-046, TD-047, TD-048 marked DONE.

### Removed (Cycle 12.2)
- `ferro-mobile-contract` crate: zero workspace consumers.
- `ferro-grpc` crate: zero workspace consumers.
- `ferro-webhook` crate: server inline webhooks.rs is more complete.
- `ferro-audit-log` crate: server inline audit.rs covers needs.
- `ferro-backend-router` crate: incompatible with current storage architecture.
- `crates/server/tests/search_workflow.rs`: referenced deleted search-index crate.

### Fixed (Audit Cycle 12)
- Replaced 9 critical production `.unwrap()` calls with proper error handling across server, auth, distributed, sync-protocol crates.
- Added error logging for 6 silently swallowed errors (event-bus replay, server indexer, fuse offline cache, server audit/snapshots).
- Deleted duplicate `OfferStore` implementation (server/src/webrtc/offers.rs), now imports from ferro-server-webrtc crate.
- Extracted shared `hash_content()` function in offline crate to avoid duplication.
- Added SAFETY documentation comments to health/src/probe.rs unsafe blocks.

### Changed (Audit Cycle 12)
- CI/CD: Added timeout-minutes to all 7 workflows (30 min regular, 60 min build jobs).
- CI/CD: Release smoke test no longer has continue-on-error; failing tests block the release.
- CI/CD: Bench workflow now uses --locked for reproducible builds.
- CI/CD: Dependabot auto-merge now waits for CI checks to pass before merging.
- CI/CD: Release workflow permissions moved from top-level to job-level (least-privilege).
- CI/CD: Fixed firecracker Dockerfile from chmod 777 to chmod 755.

### Added (Cycle 12.1)
- CRDT collaboration WebSocket relay at `/ws/collab/{document_id}` with per-document rooms, participant tracking, and presence broadcast (9 tests).
- EventBus integrated into server: AppState holds EventBus, webhook/notification handlers subscribe to file events, post-operation dispatch publishes to bus.
- Backup/restore API: `POST /api/admin/backup`, `GET /api/admin/backup/latest`, `GET /api/admin/backup/download`, `POST /api/admin/backup/restore` with SQLite WAL checkpoint, CAS blob listing, SHA-256 manifest, and zip archive support (20 tests).
- Landing page 404.html with matching Spatial Materialism design.
- Web UI shared utilities: consolidated percent_encode/percent_decode/urlencoding_decode into `crates/web/src/utils.rs`.

### Changed (Cycle 12.1)
- Crate count reduced from 39 to 38 (removed search-index, config-manager, storage-adapter).
- Removed orphaned `search_workflow.rs` integration test (referenced deleted search-index crate).
- ROADMAP: selective-sync marked as "Planned" (crate exists but not wired into server).

### Added (Audit Cycle 12)
- FocusTrap component added to 5 dialogs (web: file_preview, keyboard_shortcuts, setup_wizard; admin: modal, new FocusTrap component).
- Touch targets (min 44x44px) applied to 12 buttons across web and admin frontends.
- Form label associations (for/id) added to 8 form inputs across web frontend.
- ARIA attributes (aria-label, aria-labelledby, aria-describedby) added to 6 components.
- Skip-to-content link, mobile hamburger menu, and prefers-reduced-motion added to landing page.
- Semantic `<main>` element and focus-visible indicators added to landing page.
- deploy/Dockerfile.web and deploy/Dockerfile.admin created for ecosystem deployment.

### Removed (Audit Cycle 12)
- Internal planning documents removed from user-facing docs (gui-refactor-roadmap, ui-honest-assessment, ui-improvement-roadmap, deferred-items-analysis).

### Added (Audit Cycle 6 - Feature Expansion)
- `ferro-crdt` crate: RGA (Replicated Growable Array) text CRDT for real-time collaborative editing. 16 tests.
- `ferro-sync-delta` crate: Content-defined chunking (Buzhash rolling hash), block-level diff computation, sync protocol messages. 9 tests.
- `ferro-e2ee` crate: X25519 key management, AES-256-GCM file encryption (chunked), key envelope sharing, streaming encryption. 14 tests.
- `ferro-mount-nfs` crate: Unified `MountBackend` trait for NFS/SMB/WebDAV, NFS and SMB skeleton backends, in-memory mock implementation. 7 tests.
- `ferro-multi-tenant` crate: Organization and tenant management, quota enforcement, resource isolation, cross-tenant access control. 36 tests.
- `ferro-distributed` crate: XOR erasure coding interface, geo-replication log and coordinator, Raft consensus node (leader election, log replication, term management), membership store with failure detector. 23 tests.
- `ferro-ai` crate: Embedding model trait with mock implementation, semantic search index with cosine similarity, auto-tagging with configurable rules. 23 tests.
- `ferro-plugin-marketplace` crate: Plugin registry (register/install/uninstall/enable/disable), plugin repository trait, version compatibility checks, review system. 15 tests.
- `ferro-selective-sync` crate: Sync profiles with glob-based include/exclude rules, path filter with multi-profile support, conflict detection for concurrent edits. 22 tests.
- `ferro-mobile-contract` crate: REST API contract definitions for iOS File Provider and Android SAF integration, sync checkpoint protocol, push notification payload types. 5 tests.
- Fixed `deny.toml` license format for cargo-deny v0.18+ compatibility (added `0BSD`, `OpenSSL`).

### Fixed (Audit Cycle 5)
- Web UI WASM build failure: `web-sys` `dyn_into` return type mismatch in `app.rs` favicon/style branding code. Changed `and_then` chain to `.ok().and_then()` pattern.

### Security
- WebAuthn API endpoints now emit `tracing::warn!` on every call, clearly marking them as stubs that perform no cryptographic verification. Module doc comments updated with WARNING annotations.
- GDPR `list_user_files()` and `create_zip_archive()` documented as placeholders returning empty results.
- Admin user creation failure during password change now properly logged instead of silently swallowed.

### Fixed
- Production `unreachable!()` in versioning route handler replaced with `StatusCode::METHOD_NOT_ALLOWED` response.
- HTTP client build in remote mount proxy now uses `expect()` with descriptive message instead of bare `unwrap()`.
- WORM and retention policy JSON serialization uses `unwrap_or_else()` with error logging instead of `unwrap()`.
- Event trigger loader runtime creation uses `expect()` instead of `unwrap()`.

### Added
- 46 new unit tests for `ferro-common` (error status code mapping, ContentHash validation/compute/etag, FileMetadata, WebDAV LockDepth/LockToken/LockInfo, public auth path validation, Claims, AuthDecision).
- 10 new unit tests for `ferro-crypto` (SHA-256/512 known vectors, HMAC RFC 4231 test vector, empty inputs, constant-time edge cases, password hash uniqueness, token encoding).
- `prefers-reduced-motion` CSS media query to disable animations for users who prefer reduced motion.
- MSRV (Minimum Supported Rust Version) CI check job for Rust 1.92.
- Web UI `index.html` now includes `<meta name="description">` for SEO and sets `maximum-scale=1` for accessibility.

### Changed
- Web UI viewport meta changed from `maximum-scale=5` to `maximum-scale=1` for proper accessibility zoom behavior.

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

[Unreleased]: https://github.com/WyattAu/ferro/compare/v3.2.0...HEAD
[3.2.0]: https://github.com/WyattAu/ferro/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/WyattAu/ferro/compare/v3.0.0...v3.1.0
[2.5.1]: https://github.com/WyattAu/ferro/compare/v2.5.0...v2.5.1
[2.5.0]: https://github.com/WyattAu/ferro/compare/v2.4.0...v2.5.0
[2.4.0]: https://github.com/WyattAu/ferro/compare/v2.3.0...v2.4.0
[2.3.0]: https://github.com/WyattAu/ferro/compare/v2.2.0...v2.3.0
[2.2.0]: https://github.com/WyattAu/ferro/compare/v2.1.0...v2.2.0
[2.1.0]: https://github.com/WyattAu/ferro/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/WyattAu/ferro/compare/v1.1.0...v2.0.0
[1.1.0]: https://github.com/WyattAu/ferro/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/WyattAu/ferro/releases/tag/v1.0.0
