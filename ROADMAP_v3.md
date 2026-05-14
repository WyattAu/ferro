# Ferro Roadmap: v2.5.1 to Production and Beyond

**Version:** 2.5.1 | **Date:** 2026-05-14 | **Status:** Active Development

---

## 1. Current State Assessment

### 1.1 Codebase Metrics

| Metric | Value |
|--------|-------|
| Workspace crates | 20 |
| Total tests | 814 passed, 1 ignored |
| Clippy warnings | 0 |
| cargo-deny | PASS (advisories, bans, licenses, sources) |
| cargo-audit | 1 unsound (glib, desktop-only), 20 unmaintained (desktop GTK3 chain) |
| Pre-commit hooks | Configured (fmt, clippy, test, cargo-deny) |
| Code formatting | rustfmt clean |

### 1.2 Architecture Summary

Ferro is a self-hosted file sync and sharing platform written in Rust (edition 2024, MSRV 1.85). The 20-crate workspace implements:

- **WebDAV** (RFC 4918 Class 1/2/3) with full PROPFIND/GET/PUT/DELETE/MKCOL/COPY/MOVE/LOCK/UNLOCK
- **CalDAV** (RFC 4791) and **CardDAV** (RFC 6352)
- **REST API** (70+ endpoints) for files, users, shares, favorites, trash, tags, locks, versions, WebSockets
- **GraphQL API** (async-graphql v7)
- **Storage backends**: in-memory, local filesystem, S3, GCS, Azure Blob (feature-gated)
- **Persistence**: SQLite (bundled, WAL mode, 15 tables), PostgreSQL (optional)
- **Authentication**: simple HTTP Basic (bcrypt), OIDC PKCE flow
- **Authorization**: Cedar policy engine (middleware-enforced)
- **WASM worker runtime**: wasmtime v44 (sandboxed, fuel-limited)
- **Search**: Tantivy full-text search with auto-indexing
- **Federation**: ActivityPub (HTTP Signature, inbox/outbox, follow/accept)
- **FUSE**: Linux filesystem mount with offline cache
- **Desktop**: Tauri v2 with system tray
- **Web UI**: Leptos 0.6 (CSR, dark mode, command palette, file preview, bulk ops)
- **Admin dashboard**: Separate Leptos WASM application
- **E2E encryption**: age crate (passphrase-based)
- **Observability**: JSON structured logging, Prometheus metrics, request ID propagation

### 1.3 Known Issues and Technical Debt

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| TD-001 | CI workflow (`ci.yml`) fails to parse on GitHub Actions (pre-existing, zero jobs created) | Critical | Open |
| TD-002 | `rsa` crate RUSTSEC-2023-0071 in dependency tree (sqlx-mysql transitive) | High | Open |
| TD-003 | `glib` RUSTSEC-2024-0429 unsound (Tauri/GTK3 transitive, desktop-only) | High | Documented |
| TD-004 | 20 unmaintained advisories (all GTK3/Tauri desktop chain) | Low | Monitored |
| TD-005 | README missing 14 CLI flags documented in configuration.md | Medium | Open |
| TD-006 | README missing 40+ API endpoints | Medium | Open |
| TD-007 | `deploy/Dockerfile.web` and `deploy/Dockerfile.admin` referenced but do not exist | Medium | Open |
| TD-008 | Dockerfile does not build admin frontend | Medium | Open |
| TD-009 | Release binary built with `--no-default-features` (no cloud support) | Medium | Documented |
| TD-010 | No `redis` or `ldap` feature CI test coverage | Medium | Open |
| TD-011 | `bincode` 1.3.3 unmaintained (fuse3 transitive) | Low | Monitored |
| TD-012 | `rustls-pemfile` 2.2.0 unmaintained (object_store transitive) | Low | Monitored |
| TD-013 | Benchmark regression threshold too lenient (150%) | Low | Open |
| TD-014 | ~~Docs: rate limiter described as "token bucket" but implemented as fixed-window counter~~ CLOSED: implementation IS token bucket; docs are correct | Low | Closed |
| TD-015 | `e2e/package-lock.json` missing (npm install fallback used) | Low | Open |

---

## 2. Phase AU: CI/CD Repair (1 week)

**Goal:** Fix the broken CI pipeline so every push to main runs the full test matrix.

### 2.1 Critical: ci.yml Parsing Failure

The `ci.yml` workflow has never successfully executed on GitHub Actions (0 jobs created across 100+ attempts). The workflow renders as `.github/workflows/ci.yml` instead of `CI`, indicating a parsing failure. The YAML is syntactically valid but GitHub Actions rejects it.

**Investigation steps:**
1. Strip ci.yml down to a minimal 1-job workflow (fmt only) and verify it parses
2. Incrementally add back jobs to identify which job/feature causes the parse failure
3. Suspect causes: service container syntax, complex matrix strategies, action version references, or file length
4. Once identified, restructure the offending section

**Acceptance criteria:** CI workflow name renders as "CI" with all jobs executing on push to main.

### 2.2 Benchmarks Workflow

- Create `gh-pages` branch so `benchmark-action` auto-push works
- Reduce regression threshold from 150% to 120%
- Standardize caching to `Swatinem/rust-cache@v2`

### 2.3 Documentation Deployment

- Enable GitHub Pages in repository settings (or document that it must be enabled)
- Fix `chromium-browser` to `chromium` for Ubuntu 24.04+

### 2.4 Missing Infrastructure

- Create `deploy/Dockerfile.web` and `deploy/Dockerfile.admin` or remove compose files referencing them
- Fix `deploy/docker-compose.yml` healthcheck endpoint to match root `docker-compose.yml` (`/.well-known/ferro`)
- Fix Firecracker Dockerfile binary paths (`/app/` not `/usr/local/bin/`)

---

## 3. Phase AV: Security Hardening (2 weeks)

**Goal:** Eliminate all critical and high-severity security findings.

### 3.1 Dependency Security

| Item | Action | Priority |
|------|--------|----------|
| RUSTSEC-2023-0071 (rsa) | Isolate sqlx-mysql behind feature gate; server default excludes mysql | P0 |
| RUSTSEC-2024-0429 (glib) | Monitor Tauri GTK4 migration; document as desktop-only risk | P1 |
| bincode 1.3.3 | Monitor fuse3 for bincode 2.0 migration | P2 |
| rustls-pemfile 2.2.0 | Monitor object_store for migration | P2 |

### 3.2 Authentication Hardening

| Item | Description | Priority |
|------|-------------|----------|
| Default password enforcement | Reject `changeme` on first login; require immediate change | P0 |
| Login rate limiting | Separate 5/min per-IP limit on `/auth/login` | P0 |
| Account lockout | Lock for 15 min after 10 consecutive failures | P1 |
| OIDC token refresh | Silent refresh before expiry | P1 |
| CSRF protection | Double-submit cookie on all mutating endpoints | P0 |
| Secure cookie flags | HttpOnly, Secure, SameSite on session tokens | P0 |

### 3.3 Input Validation

| Item | Description | Priority |
|------|-------------|----------|
| File name sanitization | Reject control characters, reserved names (CON, AUX, NUL) | P0 |
| Content-Type verification | Validate uploaded Content-Type against magic bytes | P0 |
| XML entity expansion limits | Limit depth and total size in WebDAV XML bodies | P1 |
| Share link brute-force protection | Rate limit token guesses | P1 |

### 3.4 CSP Hardening

| Item | Description | Priority |
|------|-------------|----------|
| Remove `'unsafe-inline'` | Use nonce-based or hash-based CSP for styles/scripts | P1 |
| Subresource integrity | SRI hashes on CDN-served assets | P2 |

---

## 4. Phase AW: Data Integrity and Reliability (2 weeks)

**Goal:** Ensure zero data loss under crash, corruption, and concurrent access scenarios.

### 4.1 Atomic Operations

| Item | Description | Priority |
|------|-------------|----------|
| Atomic file writes | Write to temp file, then rename (prevent partial uploads on crash) | P0 |
| WAL mode verification | Confirm `PRAGMA journal_mode=WAL` is set on all SQLite connections | P0 |
| Startup integrity check | Verify CAS store checksums on boot | P1 |

### 4.2 Backup and Recovery

| Item | Description | Priority |
|------|-------------|----------|
| Database backup API | Admin endpoint to trigger and download SQLite backup | P0 |
| Data directory migration tool | CLI command to migrate between storage backends | P1 |
| Trash auto-purge daemon | Background task for items past `--trash-ttl` | P2 |

### 4.3 Configuration Safety

| Item | Description | Priority |
|------|-------------|----------|
| Startup validation | Reject invalid config combinations (CORS `*` with auth) | P0 |
| Secret redaction | Never log passwords, tokens, or API keys | P0 |
| Config schema version | Pin `ferro.toml` schema version; auto-migrate on upgrade | P1 |

### 4.4 Error Handling

| Item | Description | Priority |
|------|-------------|----------|
| Production unwrap audit | Target: zero unwraps on external input paths | P0 |
| Panic handler | Catch panics in request handlers; return 500 instead of killing connection | P1 |
| Graceful degradation | If search index fails to load, serve files without search | P1 |

---

## 5. Phase AX: Test Coverage Expansion (3 weeks)

**Goal:** Achieve >95% branch coverage on critical paths, >80% overall.

### 5.1 Property-Based Testing

| Item | Description | Priority |
|------|-------------|----------|
| Storage engine properties | proptest: PUT then GET returns identical content for random byte sequences | P0 |
| Path normalization | Verify no path escapes after N random transformations | P0 |
| Lock protocol state machine | Exhaustively test lock/refresh/unlock transitions | P1 |
| XML parsing | Proptest-generated XML fed to WebDAV parser; must not panic | P1 |

### 5.2 Missing Integration Tests

| Item | Description | Priority |
|------|-------------|----------|
| ActivityPub federation E2E | Actor discovery, inbox delivery, follow/accept | P1 |
| WASM worker pipeline E2E | Upload module, dispatch, verify result | P1 |
| GraphQL E2E | Queries, mutations, subscriptions against live server | P1 |
| File versioning E2E | PUT, overwrite, list versions, restore, diff | P1 |
| redis feature tests | Verify Redis caching with `--features redis` | P2 |
| ldap feature tests | Verify LDAP auth with `--features ldap` | P2 |

### 5.3 Fuzzing

| Item | Description | Priority |
|------|-------------|----------|
| WebDAV request fuzzer | cargo-fuzz targeting the WebDAV handler | P1 |
| XML parser fuzzer | Fuzz PROPFIND/PROPPATCH request bodies | P1 |

### 5.4 Load Testing

| Item | Description | Priority |
|------|-------------|----------|
| Concurrent upload benchmark | 100+ simultaneous PUT requests; measure throughput | P1 |
| Large directory listing | PROPFIND with 10,000+ entries; verify pagination | P1 |
| 24h soak test | Continuous random operations; zero panics or data loss | P2 |

---

## 6. Phase AY: Documentation Completion (2 weeks)

**Goal:** Complete and accurate documentation for all public interfaces.

### 6.1 API Reference

| Item | Description | Priority |
|------|-------------|----------|
| Complete endpoint documentation | Document all 70+ endpoints in `docs/api.md` | P0 |
| Fix health check documentation | Correct `/.well-known/ferro` response (plain text, not JSON) | P0 |
| Add missing CLI flags to README | Add 14 missing flags from config.rs | P0 |

### 6.2 Crate Documentation

| Item | Description | Priority |
|------|-------------|----------|
| Document 10 undocumented crates | Add mdBook pages for auth, webdav-handler, graphql, observability, admin, server-activitypub, server-webrtc, server-wopi, server-versioning, benchmarks | P1 |

### 6.3 Deployment

| Item | Description | Priority |
|------|-------------|----------|
| Production deployment guide | Docker, bare metal, Kubernetes step-by-step | P1 |
| Upgrade guide | Document migration path between versions | P1 |
| Release binary features | Document that release binary has no cloud features | P1 |

---

## 7. Phase AZ: Release Engineering (1 week)

**Goal:** Ship v3.0.0 with all release artifacts.

### 7.1 Release Criteria

All of the following must be satisfied:

- [ ] CI pipeline fully operational (all jobs green on main)
- [ ] Zero P0 items from Phases AU-AY remaining
- [ ] >95% branch coverage on critical paths (storage, auth, WebDAV)
- [ ] >80% overall branch coverage
- [ ] Zero critical or high CVEs in dependency tree
- [ ] All 70+ endpoints documented in API reference
- [ ] 24h soak test passed with zero panics or data loss
- [ ] Multi-architecture release (linux-amd64, linux-arm64, macos-arm64, windows)
- [ ] Docker image published to ghcr.io with multi-arch manifest
- [ ] Helm chart for Kubernetes deployment

### 7.2 Release Artifacts

| Artifact | Format | Platforms |
|----------|--------|-----------|
| Server binary | Static binary (musl) | linux-amd64, linux-arm64 |
| CLI binary | Static binary (musl) | linux-amd64, linux-arm64, macos-arm64, windows-msvc |
| Docker image | OCI (multi-arch) | linux/amd64, linux/arm64 |
| Helm chart | Helm v3 | Any Kubernetes |
| SBOM | SPDX JSON | Bundled with release |

### 7.3 Versioning Strategy

- Pre-release: `v3.0.0-beta.1`, `v3.0.0-rc.1`
- Stable: `v3.0.0`
- Maintenance: `v3.0.1`, `v3.0.2` (bug fixes only)
- Minor: `v3.1.0` (new features, backward compatible)

---

## 8. Phase BA: Desktop Client v3.1 (4 weeks)

**Goal:** Ship a functional desktop sync client.

| Item | Description | Priority |
|------|-------------|----------|
| File sync daemon | Background sync with conflict resolution (CRDT-based) | P0 |
| Selective sync | Per-folder sync toggle | P1 |
| System tray indicator | Sync status, recent changes, pause/resume | P1 |
| macOS universal binary | Intel + Apple Silicon | P1 |
| Windows MSI installer | Shell integration, file context menu | P1 |
| GTK4 migration | Eliminate 20 unmaintained GTK3 advisories | P1 |

---

## 9. Phase BB: Mobile v3.2 (4 weeks)

**Goal:** Provide mobile access to Ferro files.

| Item | Description | Priority |
|------|-------------|----------|
| iOS File Provider | iOS Files app integration | P1 |
| Android SAF provider | Storage Access Framework | P1 |
| Offline mode | Local cache with conflict resolution | P2 |
| Push notifications | Share received, quota warning | P2 |

---

## 10. Phase BC: Collaboration v3.3 (3 weeks)

**Goal:** Multi-user real-time collaboration.

| Item | Description | Priority |
|------|-------------|----------|
| Real-time co-editing | CRDT-based document collaboration via WebRTC | P1 |
| Comments | Per-file comment threads | P2 |
| File locking UI | Visual indicator when file is locked by another user | P2 |
| Activity notifications | Email/webhook on share, comment, mention | P2 |

---

## 11. Phase BD: Admin and Compliance v3.4 (2 weeks)

**Goal:** Enterprise administration and regulatory compliance.

| Item | Description | Priority |
|------|-------------|----------|
| Admin dashboard | User management, storage stats, audit log in web UI | P0 |
| Two-factor authentication | TOTP for admin and user accounts | P1 |
| SSO/SAML | SAML 2.0 service provider | P2 |
| Data retention policies | Automatic deletion past retention period | P2 |
| GDPR data export | All user data in machine-readable format | P2 |

---

## 12. Phase BE: Performance v3.5 (2 weeks)

**Goal:** Sub-second response times at scale.

| Item | Description | Priority |
|------|-------------|----------|
| Streaming uploads | True streaming (no full buffering before write) | P0 |
| Ranged GET with caching | `Range` header support with caching | P1 |
| Thumbnail cache | Persistent LRU thumbnail cache | P1 |
| Search index sharding | Partition Tantivy index for >1M files | P2 |
| Connection pooling | Configurable pool for cloud backends | P2 |

### Performance Targets

| Metric | Target |
|--------|--------|
| p99 latency (1KB PUT, local storage) | <10ms |
| p99 latency (PROPFIND, 1000 items) | <100ms |
| Concurrent connections (local storage) | >1000 |
| Docker image size (server) | <50MB compressed |

---

## 13. Phase BF: Platform Evolution v4.0+ (Ongoing)

### 13.1 Plugin System v4.0

| Item | Description |
|------|-------------|
| Stable WASM plugin API | Versioned ABI for plugins (beyond ad-hoc workers) |
| Plugin marketplace | Community plugin registry (thumbnails, antivirus, OCR) |
| Plugin permissions | Capability-based security model for WASM sandbox |
| Hot-reload | Load/unload plugins without restart |

### 13.2 Multi-Tenant v4.1

| Item | Description |
|------|-------------|
| Organization support | Per-org storage, quotas, policies |
| Resource isolation | Per-tenant rate limits and connection pools |
| Cross-org sharing | Controlled sharing between organizations |

### 13.3 Distributed Storage v4.2

| Item | Description |
|------|-------------|
| Erasure coding | Reed-Solomon encoding for durability |
| Geo-replication | Async replication between data centers |
| Consensus | Raft-based metadata consensus |

### 13.4 AI Integration v4.3

| Item | Description |
|------|-------------|
| Semantic search | Vector embeddings for natural language file search |
| Auto-tagging | ML-based content classification |
| OCR | Text extraction from images and PDFs |
| Smart deduplication | Perceptual hashing for near-duplicate detection |

---

## 14. Sprint Estimation

| Phase | Sprint | Duration | Dependencies |
|-------|--------|----------|--------------|
| CI/CD Repair | AU | 1 week | None |
| Security Hardening | AV | 2 weeks | AU |
| Data Integrity | AW | 2 weeks | None |
| Test Coverage | AX | 3 weeks | AW |
| Documentation | AY | 2 weeks | AX |
| Release Engineering | AZ | 1 week | AV+AW+AX+AY |
| Desktop Client | BA | 4 weeks | AZ |
| Mobile | BB | 4 weeks | BA |
| Collaboration | BC | 3 weeks | AZ |
| Admin/Compliance | BD | 2 weeks | AZ |
| Performance | BE | 2 weeks | AZ |
| Platform Evolution | BF+ | Ongoing | AZ |

**Estimated time to v3.0:** 11 weeks (assuming full-time development)

---

## 15. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| CI workflow cannot be fixed without major restructuring | Medium | High | Split into multiple smaller workflow files |
| `rsa` crate cannot be eliminated from sqlx tree | Low | Medium | Isolate mysql behind feature gate; document risk |
| Performance regression with SQLite at scale | Medium | High | Recommend PostgreSQL for >100 concurrent users |
| Leptos 0.7 breaking changes | Medium | Medium | Pin leptos version; plan migration window |
| Tauri GTK4 migration delayed | Medium | Low | Server/core unaffected; continue with GTK3 |
| WASM plugin ABI instability | High | Low | Design with versioned ABI from start |

---

## 16. Success Metrics for v3.0

| Metric | Target |
|--------|--------|
| Test coverage (critical paths) | >95% branch |
| Test coverage (overall) | >80% branch |
| Clippy warnings | 0 |
| Critical CVEs | 0 |
| API documentation completeness | 100% of endpoints |
| CI pipeline reliability | >99% green on main |
| Docker image size (server) | <50MB compressed |
| p99 latency (1KB PUT, local) | <10ms |
| p99 latency (PROPFIND, 1000 items) | <100ms |
| Concurrent connections (local) | >1000 |
| rclone E2E compatibility | 100% Class 1/2/3 WebDAV |
| Soak test | 24h zero-defect |
