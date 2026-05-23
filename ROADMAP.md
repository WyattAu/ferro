# Ferro Roadmap: v2.5.1 to Production and Beyond

**Version:** 2.5.1 | **Date:** 2026-05-20 | **Status:** Active Development

---

## Current State (2026-05-20)

| Metric | Value |
|--------|-------|
| Crates | 20 |
| Tests | 833 passed, 1 ignored |
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

---

## Phase 1: Production Hardening (Sprint AU)

**Goal:** Make Ferro safe to deploy for real users with real data.

### 1.1 Authentication and Authorization Hardening

| Item | Description | Priority |
|------|-------------|----------|
| Enforce password change on first login | Reject default `changeme` password; require immediate change | P0 |
| Rate limit login attempts | Separate, stricter rate limit on `/auth/login` (5/min per IP) | P0 |
| Account lockout after failed attempts | Lock account for 15 min after 10 consecutive failures | P1 |
| Session token rotation | Re-issue tokens on sensitive operations (password change, settings update) | P1 |
| OIDC token refresh flow | Implement silent token refresh before expiry | P1 |
| LDAP group mapping | Map LDAP groups to Cedar roles | P2 |

### 1.2 Data Integrity and Recovery

| Item | Description | Priority |
|------|-------------|----------|
| Atomic file writes | Write to temp file, then rename (prevent partial uploads on crash) | P0 |
| WAL mode for SQLite | Enable `PRAGMA journal_mode=WAL` for concurrent read/write | P0 |
| Database backup API | Admin endpoint to trigger and download SQLite backup | P0 |
| Data directory migration tool | CLI command to migrate data between storage backends | P1 |
| Checksum verification on startup | Verify CAS store integrity on boot (compare stored vs. computed hashes) | P1 |
| Trash auto-purge daemon | Background task to purge items past `--trash-ttl` | P2 |

### 1.3 Configuration Safety

| Item | Description | Priority |
|------|-------------|----------|
| Config validation on startup | Reject invalid combinations (e.g., CORS `*` with auth enabled) | P0 |
| Secret redaction in logs | Never log passwords, tokens, or API keys | P0 |
| Config file schema version | Pin `ferro.toml` schema version; migrate on upgrade | P1 |

### 1.4 Documentation Completion

| Item | Description | Priority |
|------|-------------|----------|
| Complete API reference | Document all 90+ endpoints in `docs/api.md` | P0 |
| Complete configuration reference | Document all 37 CLI flags and TOML keys | P0 |
| Deployment guide for production | Step-by-step for Docker, bare metal, Kubernetes | P1 |
| Upgrade guide | Document migration path between versions | P1 |

---

## Phase 2: Reliability and Observability (Sprint AV)

**Goal:** Make Ferro operable in production with full visibility into system health.

### 2.1 Structured Logging

| Item | Description | Priority |
|------|-------------|----------|
| Request tracing correlation | Propagate `X-Request-ID` through all log lines | P0 |
| Log level per crate | Allow `FERRO_LOG=ferro_server=debug,ferro_core=trace` | P0 |
| Slow query logging | Log SQLite queries exceeding 100ms | P1 |
| Audit log immutability | Append-only audit table; prevent deletion | P1 |

### 2.2 Metrics

| Item | Description | Priority |
|------|-------------|----------|
| Prometheus endpoint completeness | Expose request latency histograms, error rates, active connections | P0 |
| Storage backend metrics | PUT/GET latency per backend, cache hit/miss ratio | P1 |
| WASM worker metrics | Dispatch count, fuel consumption, error rate per module | P1 |
| Dashboard templates | Grafana dashboard JSON for common views | P2 |

### 2.3 Health Checks

| Item | Description | Priority |
|------|-------------|----------|
| Deep health check | Verify storage backend connectivity, not just process liveness | P0 |
| Readiness gate | `/readyz` returns 503 until search index is loaded | P1 |
| Startup probe | Separate probe for container orchestration (longer timeout) | P2 |

### 2.4 Error Handling

| Item | Description | Priority |
|------|-------------|----------|
| Reduce production `expect()` count | Target: zero expects on external input paths; 1 remaining in `hash_password()` | P0 |
| Global error handler | Consistent JSON error format across all 90+ endpoints | P0 |
| Panic handler | Catch panics in request handlers; return 500 instead of killing connection | P1 |
| Graceful degradation | If search index fails to load, serve files without search (not 500) | P1 |

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
| Storage engine properties | `proptest`: PUT then GET returns identical content for random byte sequences | P0 |
| Path normalization properties | Verify no path escapes after N random transformations | P0 |
| Lock protocol properties | Lock, refresh, unlock state machine exhaustively tested | P1 |
| XML parsing properties | Proptest-generated XML fed to WebDAV parser; must not panic | P1 |

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
| CSRF protection | Double-submit cookie or SameSite=Strict on all mutating endpoints | P0 |
| Secure cookie flags | HttpOnly, Secure, SameSite on all session tokens | P0 |
| Content Security Policy | Remove `'unsafe-inline'` from CSP; use nonce-based or hash-based styles | P0 |
| Subresource integrity | SRI hashes on all CDN-served assets | P1 |

### 4.2 Input Validation

| Item | Description | Priority |
|------|-------------|----------|
| File name sanitization | Reject names with control characters, reserved names (CON, AUX, NUL) | P0 |
| Content-Type verification | Validate uploaded Content-Type against actual file content (magic bytes) | P0 |
| XML entity expansion limits | Limit entity expansion depth and total size in WebDAV XML | P1 |
| Share link brute-force protection | Rate limit share token guesses | P1 |

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
