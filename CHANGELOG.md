# Changelog

## [1.0.0-beta.1] - 2026-04-26

### Added — Sprint AT (Mobile Responsive)
- **Responsive file list**: Card layout on mobile, table layout on desktop. Touch-friendly 44x44px targets.
- **Responsive header**: Search collapses to icon, quota hidden on small screens, touch-friendly navigation.
- **Responsive dialogs**: Full-width on mobile, sticky bottom action bar.

### Added — Sprint AR (Command Palette) + Sprint AS (Clipboard)
- **Command palette**: VS Code-style `Ctrl+K` palette with search filtering, keyboard navigation, 13 commands.
- **Keyboard shortcuts**: `Ctrl+N` new folder, `Ctrl+U` upload, `Delete` delete selected, `Ctrl+A` select all, `Ctrl+F` focus search.
- **Clipboard operations**: `Ctrl+C/X/V` to copy/cut/paste files with visual indicator showing count and action type.

### Added — Sprint AQ (Activity Feed)
- **Activity sidebar**: Collapsible panel showing recent file operations with action icons and timestamps.
- **Auto-refresh**: Activity feed updates every 30 seconds.

### Added — Sprint AP (Storage Quota)
- **Quota enforcement**: New `--storage-quota` flag (e.g., `10GB`, `500MB`). Returns 413 when exceeded.
- **Quota API**: `GET /api/quota` returns used/quota/percentage/file count.
- **Quota indicator**: Visual progress bar in header, turns red above 90%.

### Added — Sprint AO (File Move/Copy)
- **Move/Copy API**: `POST /api/files/move` and `POST /api/files/copy` with recursive folder support.
- **Context menu**: Right-click file rows for Copy/Move/Share/Delete operations.

### Added — Sprint AN (Toast Notifications)
- **Toast system**: Success (green), error (red), info (blue), warning (yellow) toasts with auto-dismiss.
- **Operation feedback**: Uploads, deletions, shares, favorites all trigger contextual toasts.

### Added — Sprint AM (Bulk Operations)
- **Multi-select**: Checkboxes on file rows, select-all in header, shift+click for range selection.
- **Bulk delete**: Delete all selected files with a single action.

### Added — Sprint AL (Trash/Recycle Bin)
- **Soft delete**: Files moved to trash instead of permanently deleted. `DELETE /api/trash/{path}`.
- **Restore**: `POST /api/trash/restore` moves file back to original location.
- **Trash page**: `/ui/trash` shows trashed files with restore/purge/empty actions.

### Added — Sprint AK (Favorites + Recent Files)
- **Favorites**: Star/unstar files, favorites view showing only starred files.
- **Recent files**: View last 50 modified files from audit log.
- **Tab switcher**: Files/Favorites/Recent tabs in file browser toolbar.

### Added — Sprint AJ (File Preview)
- **Inline preview**: Click a file to preview it in a modal. Supports images, text (100KB limit), PDF, video, audio.
- **Fallback**: Non-previewable files show download option.

### Added — Sprint AI (Dark Mode)
- **Dark theme**: System preference detection with localStorage persistence. Toggle button in header.
- **Full dark styling**: All components (file browser, dialogs, header, tables) support dark mode.

### Added — Sprint AH (Admin API)
- **System stats**: `GET /api/admin/stats` — version, uptime, file count, total bytes, features.
- **Storage breakdown**: `GET /api/admin/storage` — largest file, recent files, storage backend info.
- **Audit with pagination**: `GET /api/admin/audit?limit=50&offset=0`.

### Added — Sprint AG (Edge Case Tests)
- **25 new integration tests**: Unicode filenames, special characters, path traversal, concurrent PUT, auth edge cases, security headers verification.

### Added — Sprint AF (Web UI Accessibility)
- **WCAG 2.1 AA**: ARIA labels, keyboard navigation, color contrast, focus management, skip-to-content link.
- **Dialog accessibility**: `role="dialog"`, `aria-modal`, focus trap, Escape key close.
- **Table accessibility**: `role="grid"`, `scope="col"`, `aria-label` on action buttons.

### Added — Sprint AE (API Error Standardization)
- **Consistent error format**: All API errors now return `{ "error": "...", "error_code": "...", "details": "..." }`.
- **23 error code constants**: AUTH_REQUIRED, FILE_NOT_FOUND, SHARE_EXPIRED, WASM_INVALID, etc.
- **14 unit tests** for ApiError format and convenience methods.

### Added — Sprint AD (Release Notes)
- **RELEASE_NOTES.md**: Comprehensive release announcement with quick start, features, configuration, downloads.

### Changed
- **Test count**: 303 tests passing, 0 ignored (was 220 passing).

## [1.0.0-beta.1] - 2026-04-23

### Added — Sprint AC (Security Audit Prep)
- **Security headers middleware**: All responses now include `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`, `Permissions-Policy: camera=(), microphone=(), geolocation=()`, and `Content-Security-Policy` with strict defaults. `Strict-Transport-Security` is added only on HTTPS connections.
- **Path traversal protection**: WebDAV handlers now sanitize all paths — rejecting `..` components, null bytes, and normalizing multiple slashes. Prevents directory escape attacks.
- **OWASP Top 10 compliance checklist**: Comprehensive document (`docs/security/owasp-checklist.md`) mapping all 10 OWASP categories to Ferro's implementation status with 66 controls.
- **STRIDE threat model**: Detailed threat analysis (`docs/security/threat-model.md`) covering 7 components (auth, file storage, WASM runtime, OIDC, shares, config, API) with threat actors and asset inventory.
- **Penetration test plan**: 56 test cases (`docs/security/pen-test-plan.md`) across 11 categories (auth, path traversal, WebDAV, WASM, input validation, rate limiting, OIDC, DoS).
- **Test count**: 220 tests passing (was 215). +2 security headers tests, +3 path traversal tests.

### Added — Sprint AA (Performance Benchmarks)
- **Criterion benchmark suite**: 5 benchmark files in `crates/server/benches/` covering throughput (upload/download, concurrent 10/50/100 clients), latency (P50/P95/P99 at 1KB/1MB/10MB), WebDAV operations (PROPFIND 10/100/1000 items), WASM dispatch overhead, and storage operations (in-memory CRUD).
- **Benchmark documentation**: `docs/benchmarks.md` with run instructions, benchmark descriptions, expected results, and Nextcloud comparison methodology.

### Added — Sprint AB (E2E Testing)
- **Playwright E2E test suite**: 24 tests across 4 spec files covering file browser operations, authentication flows, file upload/download, and navigation. Configured for chromium with auto-starting ferro-server.

### Added — Sprint Z (Config File + Share Dialog)
- **Config file support**: New `--config` flag loads settings from a TOML file (`ferro.toml`). Auto-discovers config at `./ferro.toml` and `/etc/ferro/ferro.toml`. File values override defaults but not CLI flags.
- **Share dialog**: Web UI file browser now has a "Share" button per file/folder. Opens a dialog to create password-protected, time-limited share links. Copy-to-clipboard support.
- **Audit log pagination**: `GET /api/audit` now supports `?limit=N&offset=M` query parameters for paginated audit log retrieval.

### Added — Sprint Y (Ship-Ready Sprint)
- **Simple HTTP Basic Auth**: New `--admin-user` / `--admin-password` CLI flags (`FERRO_ADMIN_USER` / `FERRO_ADMIN_PASSWORD` env vars) enable built-in authentication without OIDC. Validates credentials on every request, returns 401 with `WWW-Authenticate` header on failure. Public paths (health, config, metrics) bypass auth. Perfect for personal cloud deployments.
- **Docker image with bundled web UI**: Multi-stage Dockerfile now includes a `ui-builder` stage that compiles the Leptos frontend with trunk. The final image contains both the server binary and the built web UI at `/app/ui`. Running `docker compose up` now gives you a working file browser at `http://localhost/ui/`.
- **Caddy reverse proxy**: New `Caddyfile` and Caddy service in `docker-compose.yml` provide automatic HTTPS via Let's Encrypt. Set `DOMAIN=yourdomain.com` to get TLS. Ports 80 (HTTP→HTTPS redirect) and 443 (HTTPS) are exposed.
- **Startup data-loss warning**: When the server starts without `--data-dir`, a prominent warning is printed: all data will be lost on restart. The Dockerfile CMD now always includes `--data-dir /data`.
- **Auth type reporting**: `GET /api/auth/info` now returns an `auth_type` field (`"oidc"`, `"basic"`, or `"none"`) so the web UI can adapt its login flow.

### Changed
- **Test count**: 220 tests passing, 0 ignored (was 209 passing, 0 ignored). +11 new tests across Sprints Z and AC.
- **Dockerfile**: Added `ui-builder` stage (Node 20 + trunk + Rust wasm32 target), copies `dist/` into runtime image.
- **docker-compose.yml**: Added Caddy service, changed ferro to `expose: 8080` (not directly published), added `--static-dir /app/ui` to command.
- **`/api/config` response**: Now reports `auth_enabled: true` when either OIDC or simple auth is configured.

### Security
- Simple auth passwords are stored in memory and compared with constant-time comparison. For production, use environment variables (not command-line arguments) to avoid password exposure in process listings.

## [0.16.0-beta.1] - 2026-04-22

### Added — Sprint X (Observability & DX Sprint)
- **JSON health check**: `GET /.well-known/ferro` now returns structured JSON with `status`, `version`, `uptime_seconds`, and per-subsystem status (storage, auth, search, WASM, metadata, CAS).
- **Request ID middleware**: Every request gets an `X-Request-ID` header. Preserves client-provided IDs or generates UUID v4. Available to handlers via request extensions.
- **Metrics endpoint**: `GET /metrics` returns JSON with `uptime_seconds` and storage stats (file count, total bytes). Scrapable by monitoring systems.
- **Response compression**: All responses are gzip-compressed via `tower-http::CompressionLayer`. Reduces bandwidth for JSON/XML payloads.
- **`--version` flag**: `ferro-server --version` and `ferro-cli --version` now print the version from `CARGO_PKG_VERSION`.

### Changed
- **Request logging**: Now includes `request_id` field in structured log output for log correlation.
- **AppState**: Added `started_at: std::time::Instant` for uptime tracking.
- **`tower-http`**: Added `compression-gzip` feature.

## [0.15.0-beta.1] - 2026-04-22

### Added — Sprint W (Production Hardening Sprint)
- **Graceful shutdown**: Server now handles SIGINT (Ctrl+C) gracefully with `axum::serve().with_graceful_shutdown()`, allowing in-flight requests to complete before shutting down.
- **Structured request logging middleware**: Every request is logged with method, path, HTTP status, duration (ms), and client IP using structured `tracing::info!` fields. Wired as the outermost middleware layer.
- **Request logging module**: New `crates/server/src/request_logging.rs` with `request_logging_middleware()` function.
- **4 new integration tests**: CORS preflight on API endpoints, rate limit middleware presence, workers list endpoint, config version/feature fields.

### Changed
- **`/api/config` response**: Now includes `wasm_enabled` and `wopi_configured` boolean fields for client-side feature detection.
- **Error handling in main.rs**: Replaced `path.strip_prefix("local:").unwrap()` with proper `?` error propagation — invalid storage URLs now return a clear error message instead of panicking.
- **Test count**: 203 tests passing, 0 ignored (was 199 passing, 0 ignored). +4 new integration tests.

## [0.14.0-beta.1] - 2026-04-22

### Changed — Sprint V (Release Polish Sprint)
- **Comprehensive .gitignore**: Added patterns for IDE files, OS files, environment secrets, WASM build output, Nix results, Node modules, Docker artifacts, and coverage reports.
- **.editorconfig**: Created project-wide editor configuration (UTF-8, LF line endings, 4-space Rust indent, 2-space YAML/TOML/Dockerfile indent).
- **Documentation updated**: Added `--external-url`, `--wopi-token-secret`, and `--wopi-office-url` to configuration reference, README CLI flags table, and deployment guide OIDC section.
- **CI workflow improvements**: Added `pkg-config libssl-dev` to clippy job, replaced sequential cloud feature tests with parallel matrix strategy, added security audit job with `cargo-audit`.
- **Dockerfile hardening**: Added non-root user (`ferro:ferro`), fixed dependency layer caching with per-crate dummy source files, added `pkg-config libssl-dev` to builder stage, proper `COPY --chown` for binaries.

## [0.13.0-alpha.1] - 2026-04-22

### Changed — Sprint U (Dependency Security Sprint)
- **Wasmtime upgraded 26→44**: Updated `wasmtime` and `wasmtime-wasi` from version 26 to 44, resolving 5 security advisories (RUSTSEC-2025-0046, RUSTSEC-2025-0118, RUSTSEC-2026-0020, RUSTSEC-2026-0021, RUSTSEC-2026-0097). Updated WASI imports from `preview1` to `p1` and pipe imports from `pipe` to `p2::pipe`.
- **rustls-webpki upgraded 0.103.12→0.103.13**: Fixed RUSTSEC-2026-0104 (reachable panic in CRL parsing).

### Fixed — Sprint U
- **WOPI token validation**: Replaced placeholder `validate_access_token` with proper HMAC-SHA256 signature verification and 8-hour expiry checking. Previously the token validation was a TODO stub. Added 7 new unit tests covering valid tokens, expired tokens, tampered signatures, and missing tokens.

### Added — Sprint U
- **SECURITY.md**: Comprehensive security policy with vulnerability reporting instructions, known vulnerability documentation, security features table, and production deployment security checklist.

### Changed
- **Test count**: 199 tests passing, 0 ignored (was 193 passing, 0 ignored). +7 new WOPI validation tests. (1 doctest ignored — pre-existing)
- **Dependency audit**: Only 1 unfixed advisory remains (RUSTSEC-2023-0071, rsa crate, MySQL-only impact). All Tauri/GTK advisories are optional-feature-only.

## [0.12.0-alpha.1] - 2026-04-22

### Fixed — Sprint T (Critical Fixes Sprint)
- **WASM worker runner spawn order**: Moved WASM runtime initialization before `spawn_worker_runner` call in `main.rs`. Previously the runner checked `wasm_runtime.is_some()` before it was set, so the background polling runner never started.
- **Buffer overread in content-type sniffing**: Added `data.len() >= 12` guard before accessing `data[8..12]` for WebP detection. Files between 4-11 bytes would previously panic with index-out-of-bounds.
- **CI clippy --all-features**: Changed CI clippy step from `--all-features` to `--features "s3,gcs,azure"` to avoid compiling the optional `tauri` feature which requires system libraries unavailable in CI.
- **Hardcoded WOPI token secret**: Added `--wopi-token-secret` CLI flag (`FERRO_WOPI_TOKEN_SECRET` env var). The previous hardcoded `"ferro-wopi-token-secret"` allowed anyone who read the source to forge WOPI access tokens. Warns on startup if default is used.
- **OIDC redirect URI hardcoded to localhost**: Added `--external-url` CLI flag (`FERRO_EXTERNAL_URL` env var, default `http://localhost:8080`). OIDC callback URL is now derived from the external URL instead of being hardcoded. Essential for Docker, reverse proxy, and domain-based deployments.
- **CLI policy commands were stubs**: `ferro policy list`, `ferro policy add <file>`, and `ferro policy remove <id>` now call the actual server API endpoints instead of printing "Not yet implemented".
- **Duplicate WASM worker execution**: Added `recently_processed: Arc<DashSet<String>>` to AppState. The inline PUT trigger now records processed file paths, and the polling runner skips them to prevent duplicate execution.
- **WOPI discovery urlsrc configurable**: Added `--wopi-office-url` CLI flag (`FERRO_WOPI_OFFICE_URL` env var). WOPI discovery XML now uses the configured Collabora/OnlyOffice URL for `urlsrc` attributes instead of returning empty strings.

### Changed
- **Test count**: 193 tests passing, 0 ignored (was 190 passing, 0 ignored). +3 new tests.
- **`AppState`**: Added `wopi_token_secret`, `external_url`, `wopi_office_url`, `recently_processed` fields.
- **`ServerConfig`**: Added `external_url`, `wopi_token_secret`, `wopi_office_url` fields.
- **`PkceSession`**: Now stores both `redirect_uri` (user redirect) and `callback_url` (OIDC callback).

### Tests Added
- **3 WOPI token tests**: Custom secret issuance, signature verification, default-vs-custom differentiation.

## [0.11.0-alpha.1] - 2026-04-22

### Changed — Sprint Q (Quality Sprint)
- **Zero clippy warnings**: Fixed all 51 clippy warnings across the workspace. `cargo clippy --all -- -D warnings` now passes cleanly.
- **Default implementations**: Added `Default` impls for `InMemoryStorageEngine`, `LockManager`, `LockToken`, `ShareStore`.
- **Collapsed if statements**: Merged nested `if` blocks into single conditions across webdav.rs, audit.rs, snapshots.rs, cedar.rs.
- **Redundant closures**: Replaced `.map(|x| x.clone())` with `.cloned()`, removed unnecessary closure wrappers.
- **Dead code annotations**: Added `#[allow(dead_code)]` for WASM-only functions with non-wasm stubs.
- **Large Err variants**: Boxed large error types to reduce stack size.
- **Too many arguments**: Refactored functions with >7 parameters.

### Added — Sprint R (Robustness Sprint)
- **Auth endpoint tests**: 5 tests verifying login/callback return proper error codes when OIDC is not configured, auth info returns anonymous user.
- **WASM upload edge case tests**: 4 tests for empty modules dir listing, path traversal rejection, valid magic byte storage, no-runtime upload.
- **Rate limiter boundary tests**: 3 tests for independent IP tracking, window expiry reset, exact boundary enforcement.
- **CORS middleware tests**: 2 tests for preflight headers and same-origin passthrough.
- **Config endpoint tests**: 2 tests for anonymous config response and auth_enabled flag.

### Changed
- **Test count**: 190 tests passing, 0 ignored (was 178 passing, 0 ignored). +12 new tests.
- **`ferro-server` lib tests**: 79 passing (was 67).

## [0.10.0-alpha.1] - 2026-04-22

### Added — Sprint P (Ship It Sprint)
- **GitHub Actions CI**: Comprehensive CI pipeline with rustfmt check, clippy lint, test suite (with PostgreSQL service), cloud feature tests (s3, gcs, azure), release binary build, and Docker image build.
- **GitHub Actions Release**: Multi-target release workflow (linux-gnu, macos, linux-musl) triggered on version tags, with automatic GitHub Release creation and binary uploads.
- **Dependabot**: Weekly dependency monitoring for Cargo and GitHub Actions.
- **README.md**: Complete project documentation with overview, features, quick start, configuration table, storage backend examples, OIDC setup, Docker Compose, API endpoints, development instructions, and architecture overview.
- **docs/deployment.md**: Production deployment guide covering Docker, reverse proxy (nginx/Caddy), TLS termination, database setup, backup strategy, scaling, monitoring, and security hardening.
- **docs/configuration.md**: Complete configuration reference for all 16 CLI flags, environment variables, storage backend URLs, persistence modes, feature flags, Cedar policy format, and rate limiting.
- **docs/api.md**: Full API reference for all 26 endpoint groups with request/response examples.
- **docs/webdav.md**: WebDAV client connection guide for rclone, Cyberduck, Windows Explorer, macOS Finder, Nautilus, curl, and Office integration.

## [0.9.0-alpha.1] - 2026-04-22

### Added — Sprint O (Make It Real Sprint)
- **Web UI auth flow**: Login page with OIDC provider redirect, auth callback page that exchanges authorization codes for tokens, token storage in localStorage, and reactive auth state management via Leptos context.
- **Auth-aware API client**: All WASM API functions (list_files, upload_file, delete_file, create_directory, search_files, download_file, fetch_json) now include `Authorization: Bearer <token>` header when authenticated.
- **Dynamic header auth UI**: Header component now shows user name/email + "Sign out" button when authenticated, "Sign in" link when auth enabled, and hides auth UI when auth is disabled.
- **Auth routes**: Added `/auth/callback` and `/auth/login` routes to the Leptos router for the OIDC flow.
- **WASM worker upload**: `POST /api/workers/upload` endpoint accepts multipart .wasm file uploads with magic byte validation (0x00 0x61 0x73 0x6D), UUID-based filenames, and path traversal protection.
- **WASM module management**: `GET /api/workers/modules` lists uploaded modules with metadata (filename, path, size). `DELETE /api/workers/modules/{filename}` removes uploaded modules.
- **Event-driven worker dispatch**: Workers now fire immediately after a successful PUT (file upload) instead of waiting for the 30-second polling cycle. Matching workers are triggered asynchronously.
- **Workers directory**: `--data-dir` now creates a `workers/` subdirectory for uploaded WASM modules.
- **Real cloud presigned URLs**: `S3PresignedUrlGenerator`, `GcsPresignedUrlGenerator`, and `AzurePresignedUrlGenerator` delegate to the `object_store::signer::Signer` trait for real provider-signed URLs. Wired automatically when using cloud backends.
- **AppState.workers_dir**: New field for the WASM modules storage directory.

### Changed
- **Test count**: 178 tests passing, 0 ignored (was 170 passing, 0 ignored). +8 new tests.
- **PresignedUrlGenerator trait** is now `async_trait` to support async Signer methods.
- **Cloud backend creation** in main.rs now also instantiates the matching cloud presigned URL generator.
- **`axum` multipart feature** enabled in ferro-server for WASM upload endpoint.

### Tests Added
- **3 unit tests**: WASM magic byte validation, filename traversal protection, filename extension validation.
- **5 integration tests**: Upload module, list modules, delete module, upload non-wasm rejection, upload without runtime.

## [0.8.0-alpha.1] - 2026-04-22

### Added — Sprint N (Enterprise Sprint)
- **OIDC PKCE login flow**: `GET /api/auth/login` now builds a full authorization URL with PKCE code_verifier, code_challenge (S256), and CSRF state parameter. The code_verifier is stored in a server-side cache for callback verification.
- **OIDC callback endpoint**: `GET /api/auth/callback` exchanges the authorization code for tokens via the provider's token endpoint, validates the ID token, and returns access_token + user info to the frontend.
- **PKCE session management**: `OidcValidator` stores and consumes PKCE sessions with automatic cleanup of expired sessions (>10 minutes).
- **WOPI token issuance**: `POST /wopi/files/{path}/token` issues time-limited HMAC-SHA256 signed access tokens for Office Online integration. Tokens valid for 8 hours.
- **WASM runtime initialization**: `--wasm-enabled` flag (`FERRO_WASM_ENABLED` env var) initializes the WASM worker runtime at server startup. Previously, the 600+ lines of WASM code were dead code with no CLI activation path.

### Changed
- **Test count**: 170 tests passing, 0 ignored (was 167 passing, 0 ignored). +3 new tests.
- **`/api/auth/callback` added to public paths**: OIDC callback endpoint bypasses authentication.
- **Cedar context support**: `is_authorized()` now builds Cedar `Context` from request attributes (infrastructure for attribute-based policies).

## [0.7.0-alpha.1] - 2026-04-22

### Added — Sprint M (Cloud + Polish Sprint)
- **S3 backend**: `--storage s3://bucket` with `cargo run --features s3`. Reads `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` from environment.
- **GCS backend**: `--storage gs://bucket` with `cargo run --features gcs`. Reads `GOOGLE_APPLICATION_CREDENTIALS` from environment.
- **Azure backend**: `--storage az://container` with `cargo run --features azure`. Reads `AZURE_STORAGE_ACCOUNT_NAME`, `AZURE_STORAGE_ACCOUNT_KEY` from environment.
- **Feature flags**: `s3`, `gcs`, `azure` feature flags on both `ferro-core` and `ferro-server`. Error messages hint at available backends based on enabled features.
- **ServerPresignedUrlGenerator**: New presigned URL generator that constructs URLs from a configurable base URL. Suitable for local/memory backends.
- **LOCK refresh (RFC 4918 §9.10.2)**: Sending a LOCK request with an `If` header containing a lock token now refreshes the lock's timeout instead of creating a new lock. Falls back to new lock acquisition if the token is expired.
- **Concurrent access tests**: 3 tests — concurrent PUTs to different files, concurrent GETs to same file, LOCK refresh via If header.
- **Rate limiting middleware**: Per-IP sliding window rate limiter (10,000 requests per 60 seconds). Returns `429 Too Many Requests` with `Retry-After: 60` header when exceeded.
- **`/api/auth/callback` route**: New route for OIDC callback handling.

### Changed
- **Test count**: 167 tests passing, 0 ignored (was 157 passing, 0 ignored). +10 new tests.
- **`presigned.rs` refactored**: Added `ServerPresignedUrlGenerator` alongside `NoOpPresignedUrlGenerator`. All presigned URL generators now implement the same `PresignedUrlGenerator` trait.
- **Presigned URL tests**: 5 tests (was 2) — noop put/get, server URL generation, trailing slash handling, nested paths.
- **Cloud backend error messages**: Dynamic hints based on which feature flags are compiled in.

## [0.6.0-alpha.1] - 2026-04-22

### Added — Sprint L (Persistence Layer Sprint)
- **Unified SQLite persistence (`SqlitePersistence`)**: Single SQLite database provides persistent storage for metadata, CAS content, snapshots, and audit log. One `--data-dir` flag enables everything.
- **`--data-dir` CLI flag** (`FERRO_DATA_DIR` env var): When set, creates a SQLite database at `$DATA_DIR/ferro.db` with WAL mode for concurrent access. Enables persistent metadata (via shared pool with `SqliteMetadataStore`) and SQLite-backed CAS deduplication.
- **Persistent CAS store**: `SqlitePersistence` implements `CasStore` trait with `INSERT OR IGNORE` for automatic deduplication. Content blobs stored in `cas_content` table with hash, content BLOB, size, and creation timestamp.
- **Persistent snapshots**: `SqlitePersistence` implements `SnapshotStore` trait. Snapshot entries serialized as JSON in `snapshots` table. Create/get/list/delete operations with summary type (no full entries in list).
- **Persistent audit log**: `SqlitePersistence` implements `AuditLogStore` trait. Audit entries stored in `audit_log` table with auto-increment ID. Recent entries query with configurable limit (capped at 10k).
- **`SqliteMetadataStore::from_pool()`**: New constructor allowing `SqliteMetadataStore` to share an existing `SqlitePool` with `SqlitePersistence` instead of creating a duplicate connection.
- **Request body size limits**: `--max-body-size` flag (`FERRO_MAX_BODY_SIZE` env var, default 1 GB). Requests exceeding the limit return `413 Payload Too Large` with size details.
- **`AppState.max_body_size`**: New field enforced in the WebDAV handler.
- **`/api/config` enhancements**: Now reports `metadata_persistent` and `cas_enabled` capabilities.
- **Dockerfile**: Multi-stage build (Rust builder → Debian slim runtime). Includes curl for health checks, creates `/data` volume directory.
- **docker-compose.yml**: Pre-configured Ferro service with volume mount at `/data`, health check, and restart policy.

### Changed
- **Test count**: 157 tests passing, 0 ignored (was 148 passing, 0 ignored). +9 new tests.
- **`main.rs` persistence wiring**: When `--data-dir` is set, creates `SqlitePersistence`, shares its pool with `SqliteMetadataStore`, and enables CAS dedup automatically. Falls back gracefully to in-memory stores on failure.
- **`serde_json` added** to `ferro-core` dependencies (for snapshot entry serialization).
- **`serde_json` added** to workspace dependencies.

### Tests Added
- **7 persistence tests**: CAS round-trip, CAS dedup, snapshot create/get/delete, audit log insert/recent, shared database across all traits.
- **2 body size limit tests**: rejected when over limit (413), accepted when under limit (201).

## [0.5.1-alpha.1] - 2026-04-21

### Added — Sprint K (Enterprise Hardening Sprint)
- **Cedar authorization middleware**: Cedar policies are now enforced at the middleware level on every request. HTTP methods are mapped to Cedar actions (`read`, `write`, `delete`, `list`, `admin`). Default permissive policy allows all operations; administrators can restrict via `POST /api/policies`. Public paths (health, auth, config, policies, shares) bypass authorization.
- **WOPI Discovery endpoint**: `GET /hosting/discovery` returns an XML discovery document listing supported WOPI operations (view, edit) for common file types (odt, ods, odp, docx, xlsx, pptx, txt, pdf).
- **WOPI access_token validation**: WOPI GET and POST endpoints now require `access_token` query parameter (GET) or `X-WOPI-AccessToken` header (POST). Missing or empty tokens return 401.
- **Auto-indexing on PUT/DELETE**: Files are automatically indexed into Tantivy search when uploaded (PUT) and removed from the index when deleted. Text files (text/*, JSON, XML, YAML, JS) have their content indexed; binary files get metadata-only indexing.
- **WASM worker spawn_blocking**: WASM module compilation and execution now runs on Tokio's blocking thread pool instead of starving the async runtime.
- **WASM worker timeout enforcement**: `max_time_ms` is now enforced via `tokio::time::timeout` around the blocking execution task.
- **WASM input passing**: Worker input bytes are loaded into WASM linear memory and passed as `(input_ptr, input_len)` to the exported function (instead of hardcoded zeros).
- **WASM output capture**: WASM stdout is captured into an in-memory `MemoryOutputPipe` buffer and returned in `WorkerResult.output` (instead of inheriting host stdout).
- **CLI share commands**: `ferro share list`, `ferro share create <path>`, `ferro share delete <token>`.
- **CLI snapshot commands**: `ferro snapshot list`, `ferro snapshot create`, `ferro snapshot delete <id>`, `ferro snapshot restore <id>`.
- **CLI PROPFIND parser rewrite**: Replaced manual string-search XML parser (which had infinite loop bugs) with `quick-xml` event-based parser. Correctly handles namespace-prefixed tags, self-closing elements, and nested collections.

### Fixed — Sprint K (Enterprise Hardening Sprint)
- **CLI `file list` was broken**: `list_files()` was discarding the PROPFIND response body (`let _ = body; Ok(vec![])`). Now calls `parse_propfind_response()`.
- **CLI `whoami` was fake**: Returned hardcoded anonymous user without calling the server. Now calls `/api/auth/info` and parses the real response.
- **CLI `head_file` missed collections**: Didn't detect collections via `httpd/unix-directory` MIME type. Now correctly sets `is_collection` flag.
- **CLI default Content-Type**: Removed blanket `application/octet-stream` default header that caused 415 errors on JSON POST endpoints (shares, snapshots). Content-Type is now only set for `put_file()`.
- **CLI share/snapshot commands sent no body**: `create_share` and `create_snapshot` now send proper JSON request bodies.

### Changed
- **Test count**: 148 tests passing, 0 ignored (was 125 passing, 0 ignored). +23 new tests.
- **`CedarAuthorizer` now implements `Clone`** (via `Arc<RwLock<PolicySet>>`).
- **Default Cedar policy** expanded to 5 actions: `read`, `write`, `delete`, `list`, `admin`.
- **`quick-xml` dependency added** to `ferro-cli` (with `serialize` feature).
- **Cedar middleware runs after** auth middleware in the layer stack (auth → Cedar → CORS).
- **5 Cedar unit tests**: default policy, restrictive policy, HTTP method mapping, exempt paths, decision format.
- **4 Cedar integration tests**: permissive allows all, restrictive denies write, exempt paths bypass, no-Cedar passthrough.
- **5 WOPI integration tests**: discovery XML, token validation, CheckFileInfo, GetFile contents, file not found.
- **5 CLI tests**: nested collections, empty collection, client construction, URL trimming, UserInfo serialization.

## [0.5.0-alpha.1] - 2026-04-21

### Added — Sprint J (End-to-End Sprint)
- **Static file serving**: Server now serves web frontend via `--static-dir` (or `FERRO_STATIC_DIR` env var). Assets are served at `/ui/` with SPA fallback to `index.html` for client-side routing.
- **Trunk WASM build**: `ferro-wasm-build` helper script builds the Leptos frontend to `crates/web/dist/` with `public_url = "/ui/"` for correct asset paths.
- **`build_router_with_static()`**: New function in `ferro-server` that accepts an optional static directory, nesting `ServeDir` under `/ui` with `ServeFile` fallback.

### Fixed — Sprint J (End-to-End Sprint)
- **PROPFIND root 404**: `PROPFIND /` with depth > 0 returned 404 when no explicit root collection existed. Now synthesizes a synthetic root collection metadata and lists children correctly.
- **COPY/MOVE Destination URI parsing**: WebDAV COPY and MOVE handlers now correctly parse the `Destination` header as a full URI (RFC 4918 §10.4), stripping scheme and authority before path normalization. Previously `normalize_path("http://host/path")` produced garbage paths.
- **Lock token comparison**: `release_lock()` and `refresh_lock()` now compare against `as_str()` (`urn:uuid:...`) instead of `as_opaque()` (raw UUID), matching the format used in the `Lock-Token` and `If` headers.
- **Integration test lock token extraction**: Updated `test_lock_protects_resource` to use the full `urn:uuid:` token format instead of the opaque UUID.

### Changed
- **E2E test un-ignored**: `test_real_server_e2e` (rclone E2E) is no longer `#[ignore]` and runs as part of the normal test suite.
- **Test count**: 125 tests passing, 0 ignored (was 124 passing, 1 ignored).
- **Release binaries**: `ferro-server` (31MB) and `ferro-cli` (6.7MB) built with `--release`.
- **Router base path**: Leptos `Router` uses `base="/ui"` for correct client-side routing when served under `/ui/`.
- **Cleaned up unused imports**: Removed `StorageEngine` import from `worker_runner.rs`, `mut` from `cors_middleware`, `mut` from `search.rs`.
- **`tower-http` fs feature**: Added to `Cargo.toml` for `ServeDir`/`ServeFile` support.
- **`url` crate dependency**: Added to `ferro-server` for URI parsing in COPY/MOVE Destination headers.

## [0.4.1-alpha.1] - 2026-04-20

### Added — Sprint I (Infrastructure Sprint)
- **Nix flake rewrite** with 6 purpose-built devShells:
  - `default` — Full dev environment: Rust, trunk, sqlx-cli, rclone, SQLite, PostgreSQL 16, wasm-bindgen-cli, binaryen
  - `minimal` — Rust + OpenSSL only (fastest to enter)
  - `desktop` — Full Tauri system deps: WebKitGTK4.1, GTK4, libadwaita, X11/Wayland libs
  - `web` — WASM toolchain: trunk 0.21, binaryen 126, wasm-bindgen-cli, wasm-pack, Node.js
  - `services` — PostgreSQL 16 + process-compose + overmind for service orchestration
  - `ci` — Minimal + test tools for CI pipelines
- **Helper scripts** (available in all relevant shells):
  - `ferro-pg-start` — Initialize and start PostgreSQL 16 in `.pgdata/` with trust auth
  - `ferro-pg-stop` — Stop PostgreSQL gracefully
  - `ferro-pg-reset` — Stop and delete PostgreSQL data directory
  - `ferro-test-integration` — Start PostgreSQL, create test database, run full test suite
  - `ferro-wasm-build` — Build WASM target + bundle with trunk for production
- **Process compose config** for running PostgreSQL as a managed service
- **Nix formatter** (`nixfmt`) configured via `formatter` output
- **flake.lock** regenerated with pinned nixpkgs (2026-04-14), rust-overlay (2026-04-19)

### Changed
- Rust toolchain now includes `rustfmt` and `clippy` extensions
- Added `cargo-edit`, `cargo-audit`, `cargo-watch` to core tools
- PostgreSQL 16 with plpgsql_check available in all shells that include DB services

## [0.4.0-alpha.1] - 2026-04-20

### Fixed — Sprint H (Hardening Sprint)
- **Snapshot restore logic**: Previously only re-wrote files that already existed (no-op). Now correctly distinguishes between intact files, recreated collections, and files with missing content (deleted since snapshot). Returns `files_intact`, `collections_created`, and `missing_content` counts.

### Added — Sprint H (Hardening Sprint)
- **rclone stdout/stderr parsing**: Background tasks read rclone process output, parse `Transferred:` progress lines for speed/errors, detect `ERROR:`/`Fatal`/`Failed` lines, and track current file being copied. Exposed via `MountProgress` struct with `bytes_transferred`, `speed_bytes_per_sec`, `errors`, `current_file`, `last_error`, and `status` fields.
- **Tauri command wrappers**: `crates/desktop/src/tauri_commands.rs` with 8 commands (`cmd_mount`, `cmd_unmount`, `cmd_get_mount_status`, `cmd_get_config`, `cmd_save_config`, `cmd_get_mount_progress`, `cmd_open_path`, `cmd_show_notification`, `cmd_default_mount_point`). Uses `#[cfg_attr(feature = "tauri", tauri::command)]` pattern for compilation without tauri dependency. Includes standalone functions for CLI mode.
- **SQLite metadata store tests**: 7 tests covering put/get, upsert, delete, exists, list with prefix filtering, collection metadata, and nonexistent get. Fixed list query to correctly filter deeply-nested paths.
- **OIDC JWT validation tests**: 7 tests covering valid token decoding, expired token rejection, no-expiry tokens, invalid JWT formats, public path detection, OIDC config creation with/without custom JWKS URI.
- **22 new integration tests**: Content-Type sniffing verification, PROPPATCH set property, storage stats endpoint, health check, conditional GET with If-None-Match (304 Not Modified), DELETE nonexistent (404), auth info anonymous mode.
- **Infinite scroll in file browser**: Replaced "Show more" pagination button with scroll-based infinite loading. Automatically loads 50 more entries when user scrolls within 200px of the list bottom. Shows loading spinner during load.
- **Optional `tauri` feature** for `ferro-desktop` crate.

### Changed
- Test count increased from 102 to 124 passing (22 new tests)
- `ferro-core` tests increased from 39 to 46 (7 SQLite metadata tests)
- `ferro-server` tests increased from 48 to 66 (7 OIDC + 7 server integration + 7 API tests)

## [0.3.1-alpha.1] - 2026-04-20

### Fixed — Sprint G (Bug Fix Sprint)
- **Doubled-path bug in WASM API client**: `list_files()`, `upload_file()`, `delete_file()`, and `create_directory()` were constructing URLs as `format!("{}{}", path, path)` instead of `path.to_string()`, causing all WebDAV operations from the browser to hit invalid URLs
- **Audit logging now captures client IP and user agent**: `build_audit_entry()` accepts optional `client_ip` and `user_agent` parameters; the WebDAV handler extracts `X-Forwarded-For`/`X-Real-IP` and `User-Agent` headers before logging
- **Share link password enforcement**: `serve_share()` now requires a `?password=` query parameter when the share has a password set; returns 401 with `requires_password: true` for missing passwords and 401 for wrong passwords
- **CORS middleware added**: Conditional CORS layer that only activates for cross-origin requests (those with `Origin` header); handles preflight OPTIONS requests; same-origin WebDAV OPTIONS requests pass through untouched preserving DAV/Allow headers
- **Content-type sniffing documented**: Added clarifying comment that sniffed content-type only persists when a metadata_store is configured; without one, in-memory backend retains default mime_type

### Added — Sprint G (Bug Fix Sprint)
- 7 new integration tests: audit logging capture, share link CRUD, share link password protection, snapshot create/list/delete, CORS preflight handling, same-origin OPTIONS passthrough, user path isolation via X-Ferro-User header

### Changed
- Test count increased from 95 to 102 passing (7 new integration tests)
- Removed unused `tower_http::cors::CorsLayer` and `tower_http::trace::TraceLayer` imports from main.rs
- Cleaned up pre-existing compiler warnings (unused mut in search.rs)

### Added — Sprint F (Feature Expansion)
- **Metadata Store wiring**: PostgreSQL/SQLite metadata stores wired into AppState via `--metadata-db` flag, enabling persistent file metadata across restarts
- **CAS deduplication**: Content-addressable storage wired into PUT handler via `--cas-enabled` flag, skips storage for duplicate content using SHA-256 hashing
- **Pre-signed URLs**: `/api/upload-url` and `/api/download-url` endpoints for direct-to-cloud transfers with configurable TTL
- **Share links**: `/api/shares` CRUD (create, list, delete) + public `/s/{token}` download endpoint with expiration and download limits
- **Audit logging**: `/api/audit` endpoint with structured logging, stores last 100 entries in-memory with timestamp, user, action, and path fields
- **Metadata snapshots**: `/api/snapshots` create/list/restore/delete endpoints for ransomware protection and point-in-time recovery
- **WOPI protocol**: `/wopi/files/{path}` endpoints for Collabora/OnlyOffice integration — CheckFileInfo, GetFile, PutFile, Lock, Unlock
- **Per-user path isolation**: Authenticated users get `/users/{sub}/` prefix for storage isolation, scoped by OIDC subject claim
- **WASM worker event loop**: Background runner detects file changes and triggers matching WASM workers every 30s via configurable polling interval
- **Admin dashboard**: Leptos admin page with storage stats, share links, and audit log cards
- **Virtualized scrolling**: File browser paginates at 50 entries with "Show more" button
- **Desktop Tauri preparation**: `tauri.conf.json` with window/bundle/system tray config, `build.rs` placeholder (gated behind commented build-dependency)

### Changed
- Test count increased to 95 passing

## [0.2.0-alpha.1] - 2026-04-19

### Added — Sprint C (Intelligence)
- Background content indexer that periodically scans storage and indexes into Tantivy
- `SearchEngine::commit()` method for flushing index writes
- Search index auto-creation fallback (creates index if `open()` fails)
- `spawn_indexer()` function for background indexing with configurable interval
- Search API wired to real Tantivy engine via `/api/search`
- WASM worker API wired to real Wasmtime runtime via `/api/workers`

### Added — Sprint D (Web Frontend)
- Full Leptos file browser with breadcrumb navigation
- Drag-and-drop file upload support
- File upload dialog with multi-file selection
- Create directory dialog
- File download via blob URL generation
- Delete file action with confirmation
- Search bar with dropdown results in header
- Auth status indicator in header
- Inline SVG icons (no icon library dependency)
- Proper empty/loading/error states
- Collection-first sorting in file listings
- `FileList` web-sys feature for drag-drop support
- `SearchResultEntry` and `SearchResponse` types in API module
- `urlencoding` helper for URL-safe query strings
- `download_file()` WASM API function

### Added — Sprint E (Desktop)
- Tauri configuration (`tauri.conf.json`) with window, bundle, system tray, and allowlist settings
- `DesktopState` with `mount_drive`, `unmount_drive`, `get_mount_status`, `get_config`, `save_config`
- `MountStatusResponse`, `ConfigResponse`, `SaveConfigRequest` API types
- Auto-mount on startup (`--auto-mount` CLI flag)
- Graceful shutdown with Ctrl+C cleanup (unmounts on exit)
- 3 new desktop state tests

### Changed
- CLI `search` and `mkdir` commands added
- CLI client URLs fixed (removed `/webdav` prefix, routes now match server directly)
- CLI client added `create_directory()`, `search()`, `get_server_config()` methods
- Server search fallback: tries `open()` first, then `new()` if index doesn't exist
- Background indexer spawns on server startup when search is enabled
- `SearchEngine.schema` field marked `#[allow(dead_code)]`

## [0.1.0-alpha.1] - 2026-04-18

### Added
- Complete R&D lifecycle documentation (.specs/)
- 3 Yellow Papers (WebDAV, CAS Storage, OIDC/Cedar Auth)
- 3 Blue Papers (Storage Engine, WebDAV Handler, Auth Middleware)
- 5 interface contract TOML files
- 3 Lean4 proof sketches
- STRIDE threat model (22 threats)
- Concurrency analysis (thread safety, deadlock analysis)
- Performance requirements and benchmark suite
- CI/CD pipeline configuration
- Master execution plan (21 tasks across 5 phases)

### Implemented
- **ferro-common**: Shared types (FerroError, FileMetadata, ContentHash, WebDAV types, path utilities, auth types, StorageEngine trait)
- **ferro-core**: Storage engine (InMemoryStorageEngine, ObjectStoreStorageEngine, CasStore, MetadataStore, SearchEngine, WasmWorkerRuntime)
- **ferro-server**: Axum server with full WebDAV support (PROPFIND, MKCOL, PUT, GET, DELETE, COPY, MOVE, LOCK, UNLOCK), auth middleware, API endpoints, in-memory LockManager
- **ferro-web**: Leptos web frontend scaffold with API client, file browser, header components
- **ferro-cli**: Admin CLI with file operations, server health check, search
- **ferro-desktop**: Desktop client with rclone management, mount service, tray actions

### Test Coverage
- 91 tests passing across workspace (1 ignored rclone E2E test)
- 5 path utility tests (ferro-common)
- 39 core storage tests (CAS, metadata, storage, search, WASM workers, object store)
- 41 server tests (WebDAV integration, lock management, storage, XML parsing, API)
- 6 desktop tests (config, rclone, mount state)

### Architecture Decisions
- ADR-001: Use object_store for storage abstraction
- ADR-002: Streaming XML for WebDAV responses
- ADR-003: In-memory lock manager with DashMap
- ADR-004: Cedar as authorization engine
- ADR-005: OIDC with PKCE for all clients
- ADR-006: StorageEngine trait in ferro-common (Option A)
