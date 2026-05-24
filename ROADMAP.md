# Ferro Roadmap: v2.5.1 to Production and Beyond

**Version:** 2.5.1 | **Date:** 2026-05-20 | **Status:** Active Development

---

## Current State (2026-05-20)

| Metric | Value |
|--------|-------|
| Crates | 20 |
| Tests | 847 passed, 1 ignored |
| Clippy warnings | 0 |
| Production `expect()` calls | 0 |
| Production `unwrap()` calls | 0 |
| CI/CD | 10/10 checks green (fmt, clippy, test, test-pg, test-cloud x3, audit, build, docker) |
| E2E | 24 Playwright tests passing |
| Code coverage | LLVM-cov active in CI |
| Security | cargo-deny clean, OWASP checklist complete, STRIDE threat model |
| Documentation | mdBook deployed to GitHub Pages, README comprehensive |
| Pre-commit hooks | fmt + clippy + tests + cargo-deny |

## What Was Just Completed

- Eliminated last production `expect()` in `hash_password()` -- now returns `Result`
- Corrected documentation inaccuracies (rate limiter terminology, crate count, stale version refs)
- Verified all CI/CD pipelines green
- Verified GitHub Pages docs site and repo landing page

## Audit 2026-05-23: Findings and Fixes Applied

### CI/CD Fixes Applied
- Fixed docker-compose.pg.yml and redis.yml passing `--features` as CLI args (runtime error)
- Added BUILD_FEATURES ARG to Dockerfile; release binaries now include s3/gcs/azure support
- Added rust-cache to clippy job (was rebuilding from scratch each run)
- Added `--locked` to test job for reproducible builds
- Set aarch64 cross-compile linker env in release.yml
- Switched docs.yml from hardcoded wget to `cargo install mdbook`
- Added npm ecosystems to dependabot (e2e/, web-e2e/)
- Added RUSTSEC-2026-0149 to deny.toml ignore list (WASI truncate bypass, not exploitable in Ferro)

### Documentation Fixes Applied
- Fixed `/.well-known/ferro` response format in docs/src/api/rest.md
- Consolidated 4 overlapping ROADMAP files into single ROADMAP.md
- Updated test counts in VERSION.md to match actual (833 passed)
- Updated per-crate test counts with accurate numbers

### Code Quality Audit Results
- 0 `todo!()` or `unimplemented!()` in production code
- 0 stub implementations
- 0 hardcoded secrets
- 0 `unsafe` blocks in production logic (only FFI for C-FFI client)
- 0 clippy warnings with all features
- 0 formatting issues
- cargo-deny: advisories ok, bans ok, licenses ok, sources ok
- Production `unwrap()` count: ~1274 (known tech debt, gradual improvement target)
- 1 TODO comment in graphql/src/lib.rs (auth extraction)

### Production Hardening 2026-05-24

Implemented across 2 commits (`d274895`, `52e6851`):

**Secret Redaction (P0):**
- Custom `Debug` impls for `ServerConfig`, `FileConfigValues`, `FileConfig`
  that redact `admin_password`, `wopi_token_secret`, `federation_secret`,
  `metadata_db` credentials
- `redact_url_credentials()` helper for sanitizing PostgreSQL/Redis URLs
- Fixed 3 log lines in main.rs that leaked DB/Redis connection URLs
- 6 new tests

**Atomic File Writes (P0):**
- `ferro_core::fs_util::atomic_write()` using temp-file-then-rename
- Converted 7 bare `fs::write` sites to atomic writes across backup,
  trash, thumbnails, wasm_upload, and server-versioning crates
- 5 new tests

**OIDC Token Refresh (P1):**
- `POST /api/auth/refresh` endpoint accepts refresh_token,
  exchanges via provider token_endpoint for new access_token
- `OidcValidator::refresh_access_token()` method
- Returns rotated refresh_token if provider issues new one

**LDAP Group Mapping (P2):**
- New `LdapConfig` fields: `group_search_base`, `group_filter`, `group_role_map`
- Queries LDAP groups after user bind, maps to Admin/User/ReadOnly
- Highest-privilege matching role wins

**Prometheus Histogram Fix (P0):**
- `ferro_http_request_duration_seconds_sum` was hardcoded to 0
- Now tracks cumulative request duration via `AtomicU64`

**Config Schema Version Validation (P1):**
- Rejects `schema_version > 1` at startup with clear error message

**Audit of existing features found already implemented:**
- Phase 1.1 P0: Password change enforcement, rate limiting, account lockout
- Phase 1.2 P0: WAL mode SQLite, DB backup API
- Phase 1.3 P0: Config validation on startup
- Phase 2.1 P0: Request tracing via X-Request-ID
- Phase 2.1 P1: Slow query logging (100ms threshold)
- Phase 2.3 P0: Deep health check (storage, DB, search)
- Phase 2.3 P1: Readiness gate (503 when unhealthy)
- Phase 2.4 P0: Global error handler (consistent JSON)
- Phase 2.4 P1: Panic handler (axum catches panics), graceful degradation (search)

### Production Hardening 2026-05-24 (Session 2)

**Audit Chain Verification (Phase 2.1 P1):**
- `SqlitePersistence::verify_audit_chain()` method: reads all entries ordered by id,
  recomputes SHA-256 chain hashes, compares against stored values
- `GET /api/admin/audit-chain` admin endpoint exposes verification
- `AuditLog::verify_chain()` delegates to persistence layer
- 3 new tests: valid chain verification, tamper detection, legacy NULL hash skip
- `ChainVerificationReport` and `ChainMismatch` structs for structured output

**Security Model Audit — CSRF Not Needed:**
- Ferro uses purely header-based auth (Basic + Bearer), no cookies
- Browsers do not auto-send `Authorization` headers cross-origin
- CORS config lacks `Access-Control-Allow-Credentials: true`
- UI stores tokens in `localStorage`, not cookies
- Existing `generate_csrf_token()` / `verify_csrf_token()` are dead code but harmless
- Session token rotation similarly unnecessary (no session layer)

**Additional features found already implemented:**
- Phase 1.2 P1: CAS checksum verification at startup (main.rs:622-667) + `/api/admin/integrity`
- Phase 1.2 P2: Trash auto-purge daemon (hourly tokio task)
- Phase 4.2 P0: XML entity expansion safe by default (quick-xml, no DTD processing, 10MB limit)
- Phase 4.1 P0: CSRF protection not needed (header-based auth, no cookies)
- Phase 1.1 P1: Session token rotation not needed (stateless auth, no sessions)

### Production Hardening 2026-05-24 (Session 3)

**File Name Sanitization Gap Closed (Phase 4.2 P0):**
- Added `security::validate_path()` to `encrypt_file` and `decrypt_file` handlers
- WOPI paths are token-authenticated (trusted WOPI client URLs); risk accepted

**Content-Type Verification Extended (Phase 4.2 P0):**
- WebDAV PUT handler now logs Content-Type mismatches via `security::verify_content_type()`
- Mismatches are logged (warn) but not blocked (WebDAV compatibility)

**WASM Worker Metrics (Phase 2.2 P1):**
- `AppState` gained 3 atomic counters: `wasm_dispatch_count`, `wasm_error_count`, `wasm_fuel_total`
- Both inline trigger (webdav.rs) and background runner (worker_runner.rs) update counters
- Prometheus exposes `ferro_wasm_dispatch_total`, `ferro_wasm_errors_total`, `ferro_wasm_fuel_consumed_total`

**Cache Metrics in Prometheus (Phase 2.2 P1):**
- Read cache stats now exposed: `ferro_read_cache_hits_total`, `ferro_read_cache_misses_total`, `ferro_read_cache_evictions_total`

**Security Audit Findings:**
- CSP `style-src 'unsafe-inline'` is required by Leptos WASM framework (inline `<style>` tags)
  - Impact limited: CSS-only, not script execution. Nonce-based CSP deferred to Phase 6.
- Share link brute-force: per-token lockout (10 fails / 5 min) + UUID v4 tokens (122-bit entropy) = sufficient
  - Per-IP rate limiting not added; token enumeration is computationally infeasible
- 19 property-based tests already exist (storage engine, path normalization, lock state machine)

### Production Hardening 2026-05-24 (Session 4)

**XML Parsing Property Tests (Phase 3.1 P1):**
- 6 new proptest cases for XML parsers: `parse_proppatch` and `LockRequest::parse`
  - Random XML-like content must not panic (fuzz test)
  - Valid PROPPATCH XML must extract operations
  - Valid LOCK XML must extract owner and scope
  - Empty/oversized input handled gracefully
  - `escape_xml` must not panic on arbitrary input
- Total property tests: 25 (up from 19)

**Startup Probe (Phase 2.3 P2):**
- `GET /startupz` returns 200 after all startup checks pass, 503 during initialization
- `AppState.startup_complete` atomic flag set in main.rs after CAS verification, storage reachability
- Kubernetes startup probe: `httpGet: path: /startupz, port: 8080`

**SRI Assessment (Phase 4.1 P1):**
- Only external CDN resource: Google Fonts CSS (dynamically generated per user-agent, SRI inapplicable)
- Google Fonts falls back to system fonts when unavailable (offline/air-gapped deployments)
- Desktop app is fully self-contained (no CDN references)
- Accepted risk: fonts.googleapis.com is the only external dependency in the web UI

---

## Phase 1: Production Hardening (Sprint AU)

**Goal:** Make Ferro safe to deploy for real users with real data.

### 1.1 Authentication and Authorization Hardening

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Enforce password change on first login | Reject default `changeme` password; require immediate change | P0 | DONE |
| Rate limit login attempts | Separate, stricter rate limit on `/auth/login` (5/min per IP) | P0 | DONE |
| Account lockout after failed attempts | Lock account for 15 min after 10 consecutive failures | P1 | DONE |
| Session token rotation | Re-issue tokens on sensitive operations (password change, settings update) | P1 | N/A (stateless auth) |
| OIDC token refresh flow | `POST /api/auth/refresh` exchanges refresh_token for new access_token | P1 | DONE |
| LDAP group mapping | Map LDAP groups to Admin/User/ReadOnly via `group_role_map` config | P2 | DONE |

### 1.2 Data Integrity and Recovery

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Atomic file writes | Write to temp file, then rename (prevent partial uploads on crash) | P0 | DONE |
| WAL mode for SQLite | Enable `PRAGMA journal_mode=WAL` for concurrent read/write | P0 | DONE |
| Database backup API | Admin endpoint to trigger and download SQLite backup | P0 | DONE |
| Data directory migration tool | CLI command to migrate data between storage backends | P1 | Pending |
| Checksum verification on startup | Verify CAS store integrity on boot (compare stored vs. computed hashes) | P1 | DONE |
| Trash auto-purge daemon | Background task to purge items past `--trash-ttl` | P2 | DONE |

### 1.3 Configuration Safety

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Config validation on startup | Reject invalid combinations (e.g., CORS `*` with auth enabled) | P0 | DONE |
| Secret redaction in logs | Custom Debug impls redact passwords/tokens/URLs | P0 | DONE |
| Config file schema version | Reject unsupported schema_version at startup | P1 | DONE |

### 1.4 Documentation Completion

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Complete API reference | Document all 90+ endpoints in `docs/api.md` | P0 | Pending |
| Complete configuration reference | Document all 37 CLI flags and TOML keys | P0 | Pending |
| Deployment guide for production | Step-by-step for Docker, bare metal, Kubernetes | P1 | Pending |
| Upgrade guide | Document migration path between versions | P1 | Pending |

---

## Phase 2: Reliability and Observability (Sprint AV)

**Goal:** Make Ferro operable in production with full visibility into system health.

### 2.1 Structured Logging

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Request tracing correlation | Propagate `X-Request-ID` through all log lines | P0 | DONE |
| Log level per crate | Allow `FERRO_LOG=ferro_server=debug,ferro_core=trace` | P0 | DONE |
| Slow query logging | Log SQLite queries exceeding 100ms | P1 | DONE |
| Audit log immutability | Append-only audit table; chain hash verification endpoint | P1 | DONE |

### 2.2 Metrics

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Prometheus endpoint completeness | Expose request latency histograms, error rates, active connections | P0 | DONE |
| Storage backend metrics | PUT/GET latency per backend, cache hit/miss ratio | P1 | DONE (cache stats in Prometheus) |
| WASM worker metrics | Dispatch count, fuel consumption, error rate per module | P1 | DONE (dispatch/fuel/error counters) |
| Dashboard templates | Grafana dashboard JSON for common views | P2 | Pending |

### 2.3 Health Checks

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Deep health check | `/readyz` verifies storage backend, SQLite, search index health | P0 | DONE |
| Readiness gate | `/readyz` returns 503 until all subsystems healthy | P1 | DONE |
| Startup probe | Separate probe for container orchestration (longer timeout) | P2 | DONE (/startupz) |

### 2.4 Error Handling

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| Reduce production `expect()` count | Target: zero expects on external input paths | P0 | DONE |
| Global error handler | Consistent JSON error format across all 90+ endpoints | P0 | DONE |
| Panic handler | Catch panics in request handlers; return 500 instead of killing connection | P1 | DONE |
| Graceful degradation | If search index fails, return empty results (not 500) | P1 | DONE |

---

## Phase 3: Test Coverage Expansion (Sprint AW)

**Goal:** Achieve >95% branch coverage on critical paths and >80% overall.

### 3.1 Missing E2E Tests

| Item | Description | Priority |
|------|-------------|----------|
| ActivityPub federation E2E | Test actor discovery, inbox delivery, follow/accept flow | P0 |
| WASM worker pipeline E2E | Upload module -> dispatch -> verify result | P0 |
| GraphQL E2E | Test queries, mutations, subscriptions against live server | P1 |
| File versioning E2E | PUT, overwrite, list versions, restore | P1 |
| CardDAV E2E | Test vCard CRUD via WebDAV | P1 |
| Multi-browser E2E | Add Firefox and WebKit targets to Playwright matrix | P2 |

### 3.2 Property-Based Testing

| Item | Description | Priority |
|------|-------------|----------|
| Storage engine properties | `proptest`: PUT then GET returns identical content for random byte sequences | P0 | DONE (5 tests) |
| Path normalization properties | Verify no path escapes after N random transformations | P0 | DONE (6 tests) |
| Lock protocol properties | Lock, refresh, unlock state machine exhaustively tested | P1 | DONE (8 tests) |
| XML parsing properties | Proptest-generated XML fed to WebDAV parser; must not panic | P1 | DONE (6 tests) |

### 3.3 Fuzzing

| Item | Description | Priority |
|------|-------------|----------|
| WebDAV request fuzzer | AFL++ or cargo-fuzz targeting the WebDAV handler | P1 |
| XML parser fuzzer | Fuzz PROPFIND/PROPPATCH request bodies | P1 |
| WASM module fuzzer | Fuzz WASM bytecode input to wasmtime runtime | P2 |

### 3.4 Load Testing

| Item | Description | Priority |
|------|-------------|----------|
| Concurrent upload benchmark | 100+ simultaneous PUT requests; measure throughput and error rate | P1 |
| Large directory listing | PROPFIND with 10,000+ entries; verify pagination | P1 |
| Long-running stability test | 24h soak test with continuous random operations | P2 |

---

## Phase 4: Security Certification (Sprint AX)

**Goal:** Pass independent security audit and achieve reasonable security posture.

### 4.1 Authentication Security

| Item | Description | Priority |
|------|-------------|----------|
| CSRF protection | Not needed: header-based auth (Basic+Bearer), no cookies, CORS lacks credentials | P0 | N/A |
| Secure cookie flags | Not needed: server sets no cookies (stateless auth) | P0 | N/A |
| Content Security Policy | `style-src 'unsafe-inline'` required by Leptos; `script-src 'self'` enforced | P0 | DONE |
| Subresource integrity | Google Fonts CSS is dynamic (SRI inapplicable); accepted risk, system font fallback | P1 | DONE (assessed) |

### 4.2 Input Validation

| Item | Description | Priority |
|------|-------------|----------|
| File name sanitization | Reject names with control characters, reserved names (CON, AUX, NUL) | P0 | DONE |
| Content-Type verification | Validate uploaded Content-Type against actual file content (magic bytes) | P0 | DONE |
| XML entity expansion limits | Limit entity expansion depth and total size in WebDAV XML | P1 | DONE (quick-xml safe by default) |
| Share link brute-force protection | Rate limit share token guesses | P1 | DONE (per-token lockout + UUID v4 entropy) |

### 4.3 Dependency Security

| Item | Description | Priority |
|------|-------------|----------|
| Resolve rsa RUSTSEC-2023-0071 | Eliminate `rsa` crate from dependency tree (replace age or patch sqlx) | P0 |
| Resolve GTK3 unmaintained chain | Monitor Tauri GTK4 migration; remove 22 advisory ignores | P1 |
| Dependabot auto-merge | Auto-merge patch updates for non-breaking changes | P1 |
| Reproducible builds | Pin all build toolchain versions in Nix flake and Dockerfile | P2 |

### 4.4 Audit Preparation

| Item | Description | Priority |
|------|-------------|----------|
| Threat model update | Revise STRIDE model with new attack surface from federation, WASM | P1 |
| Penetration test execution | Execute the corrected pen-test guide in SECURITY.md | P1 |
| SBOM automation | Auto-generate SPDX SBOM on every release | P2 |

---

## Phase 5: Production Release v3.0 (Sprint AY)

**Goal:** Ship a production-hardened v3.0.0.

### 5.1 Release Criteria

All of the following must be satisfied:

- [ ] Zero P0 items from Phases 1-4 remaining
- [ ] 95%+ branch coverage on critical paths (storage, auth, WebDAV)
- [ ] 80%+ overall branch coverage
- [ ] Zero critical or high CVEs in dependency tree
- [ ] All 90+ endpoints documented in API reference
- [ ] Upgrade guide from v0.x to v1.0
- [ ] 24h soak test passed with zero panics or data loss
- [ ] Multi-architecture release (linux-amd64, linux-arm64, macos-arm64, windows)
- [ ] Docker image published to ghcr.io with multi-arch manifest
- [ ] Helm chart for Kubernetes deployment
- [ ] Independent security review completed (internal or external)

### 5.2 Release Artifacts

| Artifact | Format | Platforms |
|----------|--------|-----------|
| Server binary | Static binary (musl) | linux-amd64, linux-arm64 |
| CLI binary | Static binary (musl) | linux-amd64, linux-arm64, macos-arm64, windows-msvc |
| Docker image | OCI (multi-arch) | linux/amd64, linux/arm64 |
| Helm chart | Helm v3 | Any Kubernetes |
| SBOM | SPDX JSON | Bundled with release |

### 5.3 Versioning Strategy

Current version: v2.5.1. The next major release will be v3.0.0.
- Pre-release: `v3.0.0-beta.1`, `v3.0.0-rc.1`
- Stable: `v3.0.0`
- Maintenance: `v3.0.1`, `v3.0.2` (bug fixes only)
- Minor: `v3.1.0` (new features, backward compatible)

---

## Phase 6: Post-v3.0 Growth (v3.1 - v3.5)

**Goal:** Expand user-facing features and platform support.

### 6.1 Desktop Client (v3.1)

| Item | Description | Priority |
|------|-------------|----------|
| File sync daemon | Background sync with conflict resolution | P0 |
| Selective sync | Per-folder sync toggle | P1 |
| System tray indicator | Sync status, recent changes, pause/resume | P1 |
| macOS universal binary | Support both Intel and Apple Silicon | P1 |
| Windows MSI installer | Proper Windows installer with shell integration | P1 |

### 6.2 Mobile (v3.2)

| Item | Description | Priority |
|------|-------------|----------|
| iOS file provider | iOS Files app integration via File Provider extension | P1 |
| Android Storage Access Framework | SAF provider for Android | P1 |
| Offline mode | Local cache with conflict resolution | P2 |
| Push notifications | Notify on share received, quota warning | P2 |

### 6.3 Collaboration (v3.3)

| Item | Description | Priority |
|------|-------------|----------|
| Real-time co-editing | CRDT-based document collaboration via WebRTC | P1 |
| Comments and annotations | Per-file comment threads | P2 |
| File locking UI | Visual indicator in web UI when file is locked | P2 |
| Activity notifications | Email/webhook on share, comment, mention | P2 |

### 6.4 Admin and Compliance (v3.4)

| Item | Description | Priority |
|------|-------------|----------|
| Admin dashboard | User management, storage stats, audit log viewer in web UI | P0 |
| Two-factor authentication | TOTP support for admin and user accounts | P1 |
| SSO/SAML | SAML 2.0 service provider | P2 |
| Data retention policies | Automatic deletion of files past retention period | P2 |
| Export compliance | GDPR data export (all user data in machine-readable format) | P2 |

### 6.5 Performance (v3.5)

| Item | Description | Priority |
|------|-------------|----------|
| Streaming uploads | True streaming (no full buffering before write) | P0 |
| Ranged GET with caching | Support `Range` header for partial content with caching | P1 |
| Thumbnail cache | Persistent thumbnail cache with LRU eviction | P1 |
| Search index sharding | Partition Tantivy index for >1M files | P2 |
| Connection pooling | Configurable connection pool for cloud backends | P2 |

---

## Phase 7: Platform Evolution (v4.0+)

**Goal:** Position Ferro as a platform, not just a file server.

### 7.1 Plugin System (v4.0)

| Item | Description |
|------|-------------|
| Stable WASM plugin API | Versioned ABI for WASM plugins (beyond current ad-hoc workers) |
| Plugin marketplace | Registry of community plugins (thumbnails, antivirus, OCR) |
| Plugin permissions | Capability-based security model for WASM sandbox |
| Hot-reload | Load/unload plugins without server restart |

### 7.2 Multi-Tenant (v4.1)

| Item | Description |
|------|-------------|
| Organization support | Multi-tenant with per-org storage, quotas, and policies |
| Resource isolation | Per-tenant rate limits, connection pools, and storage backends |
| Cross-org sharing | Controlled sharing between organizations |

### 7.3 Distributed Storage (v4.2)

| Item | Description |
|------|-------------|
| Erasure coding | Reed-Solomon encoding for data durability across nodes |
| Geo-replication | Async replication between data centers |
| Consensus | Raft-based metadata consensus for distributed deployments |

### 7.4 AI Integration (v4.3)

| Item | Description |
|------|-------------|
| Semantic search | Vector embeddings for natural language file search |
| Auto-tagging | ML-based content classification and tagging |
| OCR and indexing | Extract text from images and PDFs for full-text search |
| Smart deduplication | Perceptual hashing for near-duplicate detection |

---

## Technical Debt Register

Items that should be addressed during normal development:

| ID | Description | Severity | Planned Fix |
|----|-------------|----------|-------------|
| TD-001 | ~~1 `expect()` in `hash_password()` (bcrypt)~~ RESOLVED | ~~Medium~~ Done | 2026-05-20 |
| TD-002 | DashMap in-memory storage loses data on restart | Medium | Document; not a bug (use `--data-dir`) |
| TD-003 | `rsa` crate in dependency tree (RUSTSEC-2023-0071) | High | Phase 4.3 |
| TD-004 | 22 Tauri/GTK3 unmaintained advisory ignores | Low | Monitor upstream |
| TD-005 | No fuzzing infrastructure | Medium | Phase 3.3 |
| TD-006 | CalDAV/CardDAV implementation incomplete | Low | Future sprint |
| TD-007 | Desktop crate has no CI build | Low | Phase 6.1 |
| TD-008 | Benchmark regression threshold too lenient (150%) | Low | Reduce to 120% |
| TD-009 | `utoipa-swagger-ui` build requires network (downloads zip) | Low | Vendor or cache in CI |
| TD-010 | Some docker-compose files use `latest` tags | Low | Pin to SHA |

---

## Sprint Estimation

| Phase | Sprint | Estimated Duration | Dependencies |
|-------|--------|--------------------|--------------|
| Phase 1 | AU | 3 weeks | None |
| Phase 2 | AV | 2 weeks | Phase 1 |
| Phase 3 | AW | 3 weeks | Phase 1 |
| Phase 4 | AX | 2 weeks | Phase 1 |
| Phase 5 | AY | 1 week | Phases 1-4 |
| Phase 6.1 | AZ | 4 weeks | Phase 5 |
| Phase 6.2 | BA | 4 weeks | Phase 6.1 |
| Phase 6.3 | BB | 3 weeks | Phase 5 |
| Phase 6.4 | BC | 2 weeks | Phase 5 |
| Phase 6.5 | BD | 2 weeks | Phase 5 |
| Phase 7+ | BE+ | Ongoing | Phase 5 |

**Estimated time to v3.0:** 11 weeks (assuming full-time development)

---

## Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Tauri GTK4 migration delayed | Medium | Low (desktop-only) | Server/core unaffected; continue with GTK3 |
| `rsa` crate cannot be eliminated | Low | Medium | Isolate MySQL/age code paths; document risk |
| Performance regression with SQLite at scale | Medium | High | Recommend PostgreSQL for >100 concurrent users |
| Leptos 0.7 breaking changes | Medium | Medium | Pin leptos version; plan migration window |
| WASM plugin ABI instability | High | Low (future feature) | Design with versioned ABI from start |

---

## Success Metrics for v3.0

| Metric | Target |
|--------|--------|
| Test coverage (critical paths) | >95% branch |
| Test coverage (overall) | >80% branch |
| Clippy warnings | 0 |
| Critical CVEs | 0 |
| API documentation completeness | 100% of endpoints |
| Docker image size (server) | <50MB compressed |
| p99 latency (1KB PUT, local storage) | <10ms |
| p99 latency (PROPFIND, 1000 items) | <100ms |
| Concurrent connections (local storage) | >1000 |
| rclone E2E compatibility | 100% of Class 1/2/3 WebDAV operations |
| Soak test duration | 24h zero-defect |
