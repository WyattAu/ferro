# Ferro Version Tracking

## Current Status
- **Phase:** v3.1.0 Release Candidate
- **Version:** 3.1.0-rc.1
- **Crates:** 43
- **Tests:** 2500+ test functions passed, 0 failed, 0 clippy warnings
- **E2E:** 14 Playwright spec files + DOM snapshot + screenshot infrastructure
- **Fuzzing:** 4 cargo-fuzz harnesses, 2.6M+ iterations, 0 crashes
- **Load Testing:** 1h soak test passed (18,828 requests, 0 failures, P50=6ms, P95=28ms, P99=52ms)
- **Security:** cargo-deny clean, 18/18 internal pen test checks passed
- **Mobile:** Tauri v2 iOS/Android: 12 mobile commands fully implemented (WebDAV sync, offline pinning, thumbnails, push notifications, connectivity monitoring, conflict resolution). Responsive frontend with touch gestures. Config persisted to disk.
- **Status:** v5.0 complete. ALL roadmap items DONE (including mobile). 43 crates, 2500+ tests.
- **Last Updated:** 2026-06-11 (Cycle 15: Full Tauri app implementation -- all stubs replaced, config persistence, real mobile commands, Settings/Recent/Favorites views, all platforms verified)

## Phase Progress
| Phase | Status | Completion |
|-------|--------|------------|
| Phase -1: Context Discovery | Completed | 100% |
| Phase 0: Requirements | Completed | 100% |
| Phase 1: Research (Yellow Papers) | Completed | 100% |
| Phase 2: Architecture (Blue Papers) | Completed | 100% |
| Phase 2.5: Concurrency Analysis | Completed | 100% |
| Phase 3: Security Engineering | Completed | 100% |
| Phase 4: Performance Engineering | Completed | 100% |
| Phase 5: Prototype (Phase 1 MVP) | Completed | 100% |
| Phase 6: CI/CD | Completed | 100% |
| Phase 8: Execution Graph | Completed | 100% |
| Sprint J: End-to-End | Completed | 100% |
| Sprint K: Enterprise Hardening | Completed | 100% |
| Sprint L: Persistence Layer | Completed | 100% |
| Sprint M: Cloud + Polish | Completed | 100% |
| Sprint N: Enterprise | Completed | 100% |
| Sprint O: Make It Real | Completed | 100% |
| Sprint P: Ship It | Completed | 100% |

## Recently Completed

### 2026-05-31: Audit Cycle 4

**Technical Debt Resolution:**
- TD-018: Verified SAFETY doc comments on all 24 unsafe blocks (FFI, libc syscalls, test code)
- TD-019: Verified API docs comprehensive -- 83 sections in rest.md, 20 in admin.md, full coverage
- TD-025: Cedar middleware now passes IP/method/resource as context attributes (was `Context::empty()`)
- TD-026: Eliminated duplicate `is_public_path` in `server::auth::oidc`, consolidated to `common::auth::is_public_auth_path`
- TD-027: Verified TOTP HMAC-SHA1 RFC compliance documented in `crates/auth/src/totp.rs`

**New Features:**
- SAML 2.0 SP (G-08): metadata generation, AuthnRequest redirect binding, SAMLResponse parsing with NameID/attributes/groups, audience+expiry validation, cert fingerprint
- Cedar middleware (TD-025): context JSON with ip/method/resource attributes, `is_authorized` call, Allow/Deny matching
- GraphQL auth: added `CurrentUser` to `GraphQLContext`, `me()` resolver returns real identity

**Security Fixes:**
- RUSTSEC-2026-0002: upgraded `lru` 0.12->0.16
- License deny: added `AGPL-3.0-or-later` to `deny.toml`

**CI/CD Fixes:**
- Desktop CI: `libayatana-appgtk3-dev` -> `libayatana-appindicator3-dev`
- Dockerfile: moved `ARG RUST_VERSION=1.95` inside ui-builder stage

### 2026-05-30: v3.0.0 Release Preparation

**Technical Debt Resolution:**
- TD-017: Fixed poisoned lock recovery in server-activitypub (replaced `unwrap_or_else(|e| e.into_inner())` with proper error handling)
- TD-015: Propagated critical filesystem errors in GDPR data export (4 `let _ =` on fs ops now logged)
- TD-011: Replaced 6 actionable production `expect()` calls (FerroClient::new, CedarAuthorizer, HTTP api_version fallback)
- TD-016: Added SAFETY doc comments to all std::sync::Mutex in async context (4 files documented)
- TD-002: Documented DashMap in-memory storage restart behavior in AppState
- TD-021: Fixed benchmark auto-push to bench-data branch (fail-on-error: false)

**New Features (Batch 1 -- Phase 6):**
- Phase 6.5 P0: Streaming uploads -- large file uploads now stream to temp file instead of buffering entire body in memory
  - New `--streaming-upload-threshold` flag (default: 65536 bytes); files above this use streaming path
  - Atomic temp-file-then-move prevents partial uploads on crash
  - Preserves CAS dedup, content-type sniffing, versioning, audit logging
- Phase 6.3: Secure view -- share links can disable download (`allow_download=false`)
  - Serves HTML preview page with CSP blocking download actions
- Phase 6.3: File drop -- share links can accept uploads (`allow_upload=true`)
  - Upload-only share links with drag-and-drop HTML UI
  - POST /s/:token multipart upload handler with filename extraction and audit trail
- Phase 6.3: File locking UI indicator -- web UI polls GET /api/locks every 10 seconds
  - Shows lock icon on file rows and grid cards when files are locked
  - Displays lock owner and expiry time
- Phase 6.4: Data retention policies -- admin API for automated file lifecycle management
  - Create policies by path prefix with max age, max file count, min free bytes
  - Background daemon runs periodically (configurable interval, default 1 hour)
  - Dry-run mode for safe policy testing
  - SQLite persistence of policies (schema v4 migration)
- Phase 6.4: Guest account expiry enforcement -- background daemon auto-purges expired guests
  - Guest auth middleware returns 401 on expired accounts
  - Configurable cleanup interval (default 5 minutes)

**New Features (Batch 2):**
- Admin dashboard API -- user management (list/get/role/delete), storage stats (type breakdown, top files, 7-day growth), filterable audit log with summary
- GDPR data export -- `GET /api/admin/users/:id/export` returns ZIP archive of all user data
- GDPR data erasure -- `DELETE /api/admin/users/:id/data` verified purge with counts
- Comment/annotation system -- threaded replies, resolve status, audit logging (schema v5)
- Thumbnail LRU cache -- disk-backed, SHA-256 naming, `.meta` sidecar, `--thumbnail-cache-size` (100MB default)
- WASM event triggers -- fired on upload/delete/share/lock, glob path matching, admin CRUD (schema v6)
- WORM mode -- path-prefix policies, enforced on PUT/DELETE/MOVE/COPY, `WORM_PROTECTED` 403 error (schema v7)
- Remote WebDAV mount proxy -- basic auth, response caching, connectivity test, admin CRUD (schema v8)
- WebSocket real-time notifications -- broadcast file events (create/update/delete/move/share) to connected clients
- Ranged GET with partial content -- `Range: bytes=` header support, 206 Partial Content, 416 Range Not Satisfiable
- Web UI theming -- configurable logo/color/title/favicon/CSS via admin API and public `GET /api/branding` endpoint
  - Leptos frontend applies branding: document title, CSS custom property, favicon, custom CSS injection
  - Admin CRUD: `GET/PUT/DELETE /api/admin/branding`

**Test Count:** 967 unit/integration tests (+50 from batch 2), 0 failures, 0 clippy warnings

### 2026-05-30: Audit Cycle 3

**Feature Implementations:**
- G-11: ClamAV antivirus scanning via TCP socket to clamd daemon
  - INSTREAM protocol with 4KB chunked streaming (avoids buffering large files in memory)
  - Unix domain socket connection with configurable timeout
  - Max file size enforcement (25 MB default)
  - Response parsing: OK/FOUND/ERROR
  - 4 new unit tests (disabled, too-large, no-daemon, config defaults)
- SMTP email delivery via lettre crate (STARTTLS, rustls)
  - RFC-compliant email builder with plain text + multipart HTML alternative
  - TLS transport with Required mode (STARTTLS mandatory)
  - SMTP AUTH credential support (username/password)
  - Graceful fallback to INFO logging when disabled
  - Serde roundtrip for configuration persistence
  - 4 new unit tests (disabled, no-server-error, config-defaults, serde-roundtrip)

## Sprint Progress
| Sprint | Description | Status |
|--------|-------------|--------|
| Sprint A | StorageEngine trait, PROPFIND depth guard, conditional GET, PROPPATCH, Content-Type sniffing, rclone E2E | Completed |
| Sprint B | AppState builder, auth middleware, API endpoints, Cedar/OIDC config, SQLx migration | Completed |
| Sprint C | Background content indexer, search API, WASM worker API, search auto-creation | Completed |
| Sprint D | Leptos web frontend: file browser, upload/download, drag-drop, search bar, breadcrumbs | Completed |
| Sprint E | Tauri desktop scaffold, DesktopState commands, auto-mount, graceful shutdown | Completed |
| Sprint F | Metadata + CAS dedup + Presigned URLs + Share links + Audit logging + Snapshots + WOPI + User isolation + WASM event loop + Admin dashboard + Tauri build script + Virtual scrolling | Completed |
| Sprint G | Bug fixes: doubled-path URLs, audit IP/UA capture, share password enforcement, CORS middleware, content-type persistence, 7 new integration tests | Completed |
| Sprint H | Hardening: snapshot restore fix, rclone progress parsing, Tauri commands, SQLite tests, OIDC tests, virtual scrolling, 22 new tests | Completed |
| Sprint I | Nix flake: 6 devShells, PostgreSQL 16, trunk 0.21, wasm-bindgen-cli, process-compose, helper scripts | Completed |
| Sprint J | End-to-end: trunk WASM build, static file serving, PROPFIND root fix, COPY/MOVE Destination URI fix, lock token fix, E2E test un-ignored, release build | Completed |
| Sprint K | Enterprise: Cedar middleware wiring, WOPI Discovery + token validation, auto-indexing on PUT/DELETE, WASM spawn_blocking + timeout + I/O sandboxing, CLI PROPFIND rewrite with quick-xml, 23 new tests | Completed |
| Sprint L | Persistence: unified SQLite (metadata + CAS + snapshots + audit), --data-dir flag, body size limits, Dockerfile + docker-compose, 9 new tests | Completed |
| Sprint M | Cloud: S3/GCS/Azure feature flags, ServerPresignedUrlGenerator, LOCK refresh, concurrent access tests, rate limiting middleware, 13 new tests | Completed |
| Sprint N | Enterprise: OIDC PKCE login/callback, WOPI token issuance, WASM --wasm-enabled flag, Cedar context, 5 new tests | Completed |
| Sprint O | Real features: web UI auth flow (login page + callback + token storage), WASM worker upload + event-driven dispatch, real cloud presigned URLs (S3/GCS/Azure), 8 new tests | Completed |
| Sprint P | Ship it: GitHub Actions CI/CD, release workflow, Dependabot, comprehensive user documentation (README, deployment, configuration, API, WebDAV guides) | Completed |
| Sprint Q | Quality: fixed all 51 clippy warnings (Default impls, collapsed ifs, redundant closures, dead code, large Err variants), 0 warnings remaining | Completed |
| Sprint R | Robustness: 12 new integration tests (auth endpoints, WASM upload edge cases, rate limiter boundaries, CORS middleware, config endpoint), 190 total tests | Completed |
| Sprint T | Critical fixes: WASM runner spawn order, content-type buffer overread, CI --all-features, WOPI secret configurable, OIDC redirect URI, CLI policy commands, duplicate worker dedup, WOPI urlsrc configurable | Completed |
| Sprint U | Dependency security: wasmtime 26→44 (5 advisories fixed), rustls-webpki 0.103.13 (CRL panic fixed), WOPI token validation with HMAC+expiry, SECURITY.md, 7 new tests | Completed |
| Sprint V | Release polish: .gitignore/.editorconfig, docs updated (new CLI flags), CI matrix strategy + audit job, Dockerfile hardening (non-root user, layer caching) | Completed |
| Sprint W | Production hardening: graceful shutdown (SIGINT), structured request logging middleware, unwrap→proper errors, version info in /api/config, 1 new test | Completed |
| Sprint X | Observability: JSON health check with subsystem status, request ID middleware (X-Request-ID), gzip compression, /metrics endpoint, --version flag | Completed |
| Sprint Y | Ship-ready: simple auth (--admin-user/--admin-password), Docker with bundled web UI, Caddy HTTPS reverse proxy, data-dir startup warning, 6 new auth tests | Completed |
| Sprint Z | Config file (ferro.toml, --config flag, auto-discovery), audit pagination, web UI share dialog | Completed |
| Sprint AA | Performance benchmarks: criterion suite (throughput, latency, WebDAV ops, WASM dispatch, storage ops) | Completed |
| Sprint AB | E2E testing: Playwright suite (24 tests across file browser, auth, upload/download, navigation) | Completed |
| Sprint AC | Security audit prep: OWASP checklist, STRIDE threat model, pen-test plan, security headers middleware, path traversal protection, 5 new tests | Completed |
| Sprint AD | Release notes: comprehensive RELEASE_NOTES.md for v1.0.0-beta.1 | Completed |
| Sprint AE | API error standardization: consistent JSON error format with error codes, 23 error code constants | Completed |
| Sprint AF | Web UI accessibility: WCAG 2.1 AA (ARIA labels, keyboard nav, color contrast, focus management, skip nav) | Completed |
| Sprint AG | Edge case integration tests: 25 new tests (special chars, unicode, path traversal, auth edge cases) | Completed |
| Sprint AH | Admin API: /api/admin/stats, /api/admin/storage, /api/admin/audit endpoints | Completed |
| Sprint AI | Dark mode: system preference detection, localStorage persistence, full dark theme across all components | Completed |
| Sprint AJ | File preview: inline preview for images, text, PDF, video, audio with 100KB text limit | Completed |
| Sprint AK | Favorites + Recent: star/unstar files, favorites view, recent files view from audit log | Completed |
| Sprint AL | Trash/recycle bin: soft delete, restore, purge, empty trash, trash page | Completed |
| Sprint AM | Bulk operations: multi-select with checkboxes, shift+click range, batch delete | Completed |
| Sprint AN | Toast notifications: success/error/info/warning toasts, auto-dismiss, accessible | Completed |
| Sprint AO | File move/copy: server endpoints, context menu, recursive folder support | Completed |
| Sprint AP | Storage quota: --storage-quota flag, usage tracking, 413 on exceed, quota indicator in header | Completed |
| Sprint AQ | Activity feed: recent operations sidebar, auto-refresh, action icons | Completed |
| Sprint AR | Command palette: Ctrl+K trigger, search filtering, keyboard navigation, 13 commands | Completed |
| Sprint AS | Clipboard operations: Ctrl+C/X/V copy/cut/paste files, visual clipboard indicator | Completed |
| Sprint AT | Mobile responsive: card layout, touch targets, responsive toolbar/dialogs/header | Completed |

## Feature Status
| Feature | Status |
|--------|--------|
| WebDAV (full Class 1/2/3) | Working |
| Storage backends (memory, local) | Working |
| **Storage backends (S3, GCS, Azure)** | **Working (feature flags: s3, gcs, azure)** |
| Content-addressable dedup (CAS) | Wired (in-memory or SQLite) |
| PostgreSQL metadata | Wired (optional) |
| SQLite metadata | Wired (--data-dir or --metadata-db) |
| Unified SQLite persistence | Wired (--data-dir for metadata + CAS + snapshots + audit) |
| Request body size limits | Working (--max-body-size, default 1GB) |
| **Docker support** | **Dockerfile + docker-compose.yml + Caddy HTTPS + bundled web UI** |
| Tantivy full-text search | Working + auto-indexed |
| WASM worker runtime | Working + sandboxed (--wasm-enabled) |
| **OIDC authentication** | **Working (PKCE login flow + callback + token validation)** |
| **Simple HTTP Basic Auth** | **Working (--admin-user/--admin-password for personal use)** |
| **OIDC callback** | **Working (/api/auth/callback)** |
| Cedar authorization | Enforced at middleware level |
| Share links | Working |
| Pre-signed URLs | Working (ServerPresignedUrlGenerator) |
| Audit logging | Working |
| Metadata snapshots | Working |
| WOPI protocol | Working + Discovery + **token issuance** |
| **LOCK refresh** | **Working (RFC 4918 §9.10.2 via If header)** |
| **Rate limiting** | **Working (10k req/min per IP, 429 response)** |
| Per-user isolation | Working |
| Leptos web UI | Built + served at /ui/ + **auth-connected (login, callback, token storage, user display)** |
| Tauri desktop | Config ready |
| Admin CLI | Working + share/snapshot commands |
| Static file serving | Working (--static-dir) |
| **WASM worker upload** | **Working (multipart upload, module listing, deletion)** |
| **Event-driven dispatch** | **Working (workers fire immediately on PUT)** |
| **Real cloud presigned URLs** | **Working (S3/GCS/Azure via object_store Signer trait)** |
| **Graceful shutdown** | **Working (SIGINT handler, connection draining)** |
| **Structured request logging** | **Working (method, path, status, duration, client IP, request ID)** |
| **Health check** | **Working (JSON with subsystem status, version, uptime)** |
| **Metrics endpoint** | **Working (JSON metrics: uptime, storage stats)** |
| **Request ID** | **Working (X-Request-ID header, preserved from client or UUID v4)** |
| **Response compression** | **Working (gzip via tower-http)** |
| **--version flag** | **Working (server + CLI binaries)** |
| **User documentation** | **README + deployment + configuration + API + WebDAV guides** |
| **Dark mode** | **Working (system preference + manual toggle)** |
| **File preview** | **Working (images, text, PDF, video, audio)** |
| **Favorites** | **Working (star/unstar, favorites view)** |
| **Recent files** | **Working (from audit log)** |
| **Trash/recycle bin** | **Working (soft delete, restore, purge)** |
| **Bulk operations** | **Working (multi-select, batch delete)** |
| **Toast notifications** | **Working (success/error/info/warning)** |
| **File move/copy** | **Working (recursive, API + UI)** |
| **Storage quota** | **Working (--storage-quota, 413 on exceed)** |
| **Activity feed** | **Working (auto-refresh sidebar)** |
| **Command palette** | **Working (Ctrl+K, 13 commands)** |
| **Clipboard operations** | **Working (Ctrl+C/X/V, visual indicator)** |
| **Mobile responsive** | **Working (card layout, touch targets)** |
| **WCAG 2.1 AA** | **Working (ARIA, keyboard nav, focus management)** |
| **Streaming uploads** | **Working (temp-file-then-move, --streaming-upload-threshold)** |
| **Secure view shares** | **Working (allow_download=false, CSP preview)** |
| **File drop shares** | **Working (allow_upload=true, drag-and-drop HTML)** |
| **Data retention policies** | **Working (path prefix, max age/count/free bytes, daemon)** |
| **Guest account expiry** | **Working (401 on expired, auto-cleanup daemon)** |
| **Admin dashboard API** | **Working (user mgmt, storage stats, audit log)** |
| **GDPR export/erasure** | **Working (ZIP export, verified purge)** |
| **Comments/annotations** | **Working (threaded replies, resolve status)** |
| **Thumbnail LRU cache** | **Working (disk-backed, --thumbnail-cache-size)** |
| **WASM event triggers** | **Working (glob matching, admin CRUD)** |
| **WORM mode** | **Working (path-prefix policies, 403 enforcement)** |
| **Remote WebDAV mount** | **Working (proxy, caching, admin CRUD)** |
| **WebSocket notifications** | **Working (file events broadcast)** |
| **Ranged GET** | **Working (206 Partial Content, 416, Accept-Ranges)** |
| **Web UI theming** | **Working (logo/color/title/favicon/custom CSS)** |
| **SMTP email delivery** | **Working (lettre, STARTTLS, rustls, AUTH)** |
| **ClamAV antivirus scanning** | **Working (clamd TCP INSTREAM, chunked streaming)** |

## Crates Status
| Crate | Tests | Status |
|-------|-------|--------|
| ferro-common | 8 passing | Implemented |
| ferro-core | 75 passing | Implemented |
| ferro-server | 506 lib + integration + property | Implemented |
| ferro-web | 36 passing (non-WASM stubs + API tests) | Implemented |
| ferro-cli | 8 passing | Implemented |
| ferro-desktop | 39 passing | Implemented |
| ferro-dav | 55 passing | Implemented |
| ferro-auth | 50 passing | Implemented |
| ferro-client | 11 passing | Implemented |
| ferro-crypto | 8 passing | Implemented |
| ferro-fuse | 22 passing | Implemented |
| ferro-graphql | 19 passing | Implemented |
| ferro-observability | 18 passing | Implemented |
| ferro-admin | 34 passing | Implemented |
| ferro-server-versioning | 17 passing | Implemented |
| ferro-server-wopi | 14 passing | Implemented |
| ferro-server-activitypub | 29 passing | Implemented |
| ferro-server-webrtc | 2 passing | Implemented |
| ferro-webdav-handler | 10 passing | Implemented |
| ferro-benchmarks | 18 benchmark functions | Implemented |

## Total Tests: 2184+ passed, 0 failed
## E2E Tests: 23 Playwright (11 spec files, 3 browsers)
## Property Tests: 4 (proptest)
## Fuzzing: 4 harnesses, 2.6M+ iterations, 0 crashes
## Load Testing: 69 req/s (20 VUs, 30s, 0% failure)
## Security Review: 18/18 internal checks passed
## Clippy: 0 warnings (with all features: s3,gcs,azure,pg,redis,ldap)
## Security: cargo-deny clean, advisories/bans/licenses/sources ok
## Error Level: None
## Rollback Checkpoint: main@90e5755 (pre-batch-2 polish, 2026-05-30)
