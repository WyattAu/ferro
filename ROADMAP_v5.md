# Ferro Roadmap v5: Detailed Path to Production and Beyond

**Version:** 2.5.1 | **Date:** 2026-05-20 | **Author:** Nexus (Principal Systems Architect)

---

## 1. Current State Assessment

### 1.1 Codebase Health

| Metric | Value | Assessment |
|--------|-------|------------|
| Workspace crates | 20 | Correct -- matches Cargo.toml |
| Unit + integration tests | 833 passed, 1 ignored | All green |
| Doc-tests | 2 passed, 1 ignored | Acceptable (persistence doctest) |
| Clippy warnings | 0 | Clean (`-D warnings`) |
| Rustfmt | Clean | Enforced by pre-commit |
| cargo-deny | Clean (warnings only: transitive dep duplicates) | Acceptable |
| Production `unwrap()` | 0 | Zero panics on external input |
| Production `expect()` | 0 | All bcrypt expects eliminated |
| Debug artifacts (`dbg!`, `println!`) | 0 in library code | Clean |
| Hardcoded secrets in production | 0 | Clean |
| `#[allow(dead_code)]` | 11 in fuse crate (justified) | Low severity |

### 1.2 CI/CD Pipeline Health

| Workflow | Jobs | Status |
|----------|------|--------|
| checks.yml | 10 (fmt, clippy, test, test-pg, test-cloud x3, audit, build, docker) | All green |
| extended-checks.yml | 2 (E2E Playwright, Code Coverage) | All green |
| bench.yml | 1 (Criterion benchmarks) | Green |
| docs.yml | 2 (mdBook build, GitHub Pages deploy) | Green |
| release.yml | 3 (multi-arch build, release, Docker push) | Configured (on tag) |
| Dependabot | Weekly cargo + GitHub Actions | Active |

### 1.3 Documentation Health

| Artifact | Status |
|----------|--------|
| README.md | Comprehensive, emoji-free, links valid |
| GitHub Pages docs (mdBook) | Deployed, navigation working, 30+ pages |
| API reference | Complete with request/response examples |
| Deployment guide | Docker, Kubernetes, nginx, Caddy, bare metal |
| Configuration reference | All 37+ CLI flags documented |
| OWASP checklist | Complete with remediation notes |
| STRIDE threat model | Complete |
| SECURITY.md | Complete with pen-test guide |
| VERSION.md | Current, accurate |

### 1.4 Architecture Strengths

- Zero `unwrap()` in production code -- exceptional for a project of this size
- Full WebDAV Class 1/2/3 (RFC 4918) with bounded depth:infinity
- Multiple storage backends with unified trait interface
- Cedar policy engine for fine-grained authorization
- OIDC PKCE authentication with simple auth fallback
- WASM sandboxed worker runtime with fuel/memory/time limits
- Tantivy full-text search with auto-indexing
- ActivityPub federation with HTTP signatures
- CalDAV/CardDAV with full iCalendar/vCard parsing
- Leptos WASM web frontend with dark mode, responsive design
- Criterion benchmark suite with regression detection

### 1.5 Identified Gaps

| Gap | Severity | Phase |
|-----|----------|-------|
| No Helm chart for Kubernetes | Medium | Phase 5 |
| No streaming uploads (full buffer before write) | Medium | Phase 6.5 |
| No formal fuzzing infrastructure | Medium | Phase 3 |
| No property-based testing | Medium | Phase 3 | **DONE** -- 19 proptest cases |
| CalDAV/CardDAV not fully tested end-to-end | Low | Phase 3 |
| Desktop crate has no CI build | Low | Phase 6.1 |
| No SRI hashes on static assets | Low | Phase 4 | **DONE** -- post-build script |
| No CSRF double-submit cookies | Low | Phase 4 |
| No account lockout after failed auth | Low | Phase 1 |
| No OIDC token refresh flow | Low | Phase 1 | **N/A** -- server is resource server, refresh is client-side |
| 11 `#[allow(dead_code)]` in fuse crate | Low | Phase 2 |
| Benchmark regression threshold 150% (should be 120%) | Low | Phase 2 | **DONE** |
| `utoipa-swagger-ui` build requires network | Low | Phase 2 |

---

## 2. Phase 1: Production Hardening (Sprint AU)

**Goal:** Make Ferro safe to deploy for real users with real data.
**Duration:** 3 weeks | **Dependencies:** None

### 2.1 P0 Items

| ID | Item | Description | Acceptance Criteria |
|----|------|-------------|-------------------|
| AU-001 | Enforce password change on first login | Reject `changeme` default; require change on first successful auth | Integration test verifies 403 with default password |
| AU-002 | Rate limit login attempts | 5 attempts/min per IP on auth endpoints | Rate limiter test; 429 after 5 failures |
| AU-003 | ~~Atomic file writes~~ | Write to temp, then rename to prevent partial uploads | **DONE** -- object_store already does atomic writes (hard link + rename) |
| AU-004 | ~~Database backup API~~ | `POST /api/admin/backup` triggers SQLite `.backup()` | **DONE** -- uses `VACUUM INTO` for consistent backup |
| AU-005 | ~~Config validation on startup~~ | Reject CORS `*` with auth, warn on http external_url | **DONE** -- http external_url warning added |
| AU-006 | ~~Secret redaction in logs~~ | Verify no passwords/tokens in any log output | **DONE** -- password removed from log output |
| AU-007 | Complete API reference in docs | Document all endpoints with request/response | Peer review of docs/api.md |
| AU-008 | Complete config reference | Document all CLI flags and TOML keys | Peer review of docs/configuration.md |

### 2.2 P1 Items

| ID | Item | Description |
|----|------|-------------|
| AU-009 | Account lockout (10 failures, 15 min lock) |
| AU-010 | ~~OIDC token refresh flow~~ | **N/A** -- Ferro is a resource server; token refresh is client-side |
| AU-011 | ~~Config file schema versioning~~ | **DONE** -- `schema_version` field in config |
| AU-012 | Production deployment guide (Docker, bare metal, K8s) |
| AU-013 | ~~Upgrade guide between versions~~ | **DONE** -- `docs/src/guides/upgrade.md` |
| AU-014 | ~~Checksum verification on startup for CAS store~~ | **DONE** -- SHA-256 verification with mismatch logging |
| AU-015 | Trash auto-purge daemon (background task) |

---

## 3. Phase 2: Reliability and Observability (Sprint AV)

**Goal:** Operable in production with full system health visibility.
**Duration:** 2 weeks | **Dependencies:** Phase 1

### 3.1 P0 Items

| ID | Item | Description | Acceptance Criteria |
|----|------|-------------|-------------------|
| AV-001 | Request tracing correlation | Propagate `X-Request-ID` through all log lines | Grep test: all `tracing::info!` include request_id |
| AV-002 | Per-crate log levels | `FERRO_LOG=ferro_server=debug,ferro_core=trace` | Verify with `RUST_LOG` env var |
| AV-003 | Prometheus endpoint completeness | Request latency histograms, error rates, active connections | curl `/metrics/prometheus` returns histogram buckets |
| AV-004 | Deep health check | Verify storage backend connectivity | Health check returns storage status |
| AV-005 | Consistent JSON error format | All endpoints return `{code, message, details}` | Integration test for every error path |
| AV-006 | ~~Panic handler in request handlers~~ | **DONE** -- logs 500 with request context |
| AV-007 | ~~Graceful degradation~~ | **DONE** -- search returns 200 with `degraded: true` |

### 3.2 P1 Items

| ID | Item | Description |
|----|------|-------------|
| AV-008 | ~~Slow query logging (SQLite >100ms)~~ | **DONE** -- rusqlite `profile()` callback |
| AV-009 | ~~Audit log immutability (append-only)~~ | **DONE** -- SHA-256 hash chain, no delete API |
| AV-010 | ~~Storage backend metrics (PUT/GET latency, cache hit/miss)~~ | **DONE** -- `ferro_storage_operations_total` in Prometheus |
| AV-011 | ~~WASM worker metrics (dispatch count, fuel, error rate)~~ | **DONE** -- dynamic worker count in Prometheus |
| AV-012 | Grafana dashboard JSON templates |
| AV-013 | Readiness gate (`/readyz` returns 503 until ready) |
| AV-014 | ~~Reduce benchmark regression threshold to 120%~~ | **DONE** |

---

## 4. Phase 3: Test Coverage Expansion (Sprint AW)

**Goal:** >95% branch coverage on critical paths, >80% overall.
**Duration:** 3 weeks | **Dependencies:** Phase 1

### 4.1 P0 Items

| ID | Item | Description | Acceptance Criteria |
|----|------|-------------|-------------------|
| AW-001 | ActivityPub federation E2E | Test actor discovery, inbox delivery, follow/accept | E2E test passes with live server |
| AW-002 | WASM worker pipeline E2E | Upload module, dispatch on PUT, verify result | E2E test verifies transformed output |
| AW-003 | ~~Storage engine property tests~~ | **DONE** -- 19 proptest cases (put/get roundtrip, overwrite, isolation) | 1000 random byte sequences tested |
| AW-004 | ~~Path normalization property tests~~ | **DONE** -- slash prefix, no dotdot, idempotent, no double slash | proptest-generated paths all pass |
| AW-005 | ~~Lock protocol property tests~~ | **DONE** -- exclusive/shared conflict, double release, infinity lock | Lock, refresh, unlock tested |

### 4.2 P1 Items

| ID | Item | Description |
|----|------|-------------|
| AW-006 | GraphQL E2E (queries, mutations, subscriptions) |
| AW-007 | File versioning E2E (PUT, overwrite, list, restore) |
| AW-008 | CardDAV E2E (vCard CRUD via WebDAV) |
| AW-009 | XML parsing property tests (proptest-generated XML) |
| AW-010 | WebDAV request fuzzer (cargo-fuzz) |
| AW-011 | Concurrent upload benchmark (100+ simultaneous PUTs) |
| AW-012 | Large directory listing benchmark (10,000+ entries) |
| AW-013 | 24h soak test with continuous random operations |

### 4.3 P2 Items

| ID | Item | Description |
|----|------|-------------|
| AW-014 | Multi-browser E2E (Firefox, WebKit Playwright targets) |
| AW-015 | XML parser fuzzer (PROPFIND/PROPPATCH bodies) |
| AW-016 | WASM module fuzzer (bytecode input) |

---

## 5. Phase 4: Security Certification (Sprint AX)

**Goal:** Pass independent security audit.
**Duration:** 2 weeks | **Dependencies:** Phase 1

### 5.1 P0 Items

| ID | Item | Description | Acceptance Criteria |
|----|------|-------------|-------------------|
| AX-001 | CSRF protection | Double-submit cookie or SameSite=Strict on mutations | POST without CSRF token returns 403 |
| AX-002 | Secure cookie flags | HttpOnly, Secure, SameSite on session tokens | Cookie headers verified in response |
| AX-003 | ~~Content Security Policy~~ | **DONE** -- `base-uri`, `form-action` added |
| AX-004 | File name sanitization | Reject control chars, reserved names (CON, AUX, NUL) | Integration test with bad filenames |
| AX-005 | Content-Type magic byte verification | Validate uploaded Content-Type against file content | Test with mismatched Content-Type header |

### 5.2 P1 Items

| ID | Item | Description |
|----|------|-------------|
| AX-006 | ~~Subresource integrity (SRI hashes on CDN assets)~~ | **DONE** -- `tools/sri-inject.sh` post-build script |
| AX-007 | ~~XML entity expansion limits in WebDAV~~ | **DONE** -- 10 MiB body size limit on all parsers |
| AX-008 | ~~Share link brute-force protection~~ | **DONE** -- 10 attempts, 5-min lockout (DashMap) |
| AX-009 | Resolve `rsa` crate dependency (RUSTSEC-2023-0071) |
| AX-010 | ~~Threat model update (federation, WASM attack surface)~~ | **DONE** -- added to `docs/src/security.md` |
| AX-011 | Penetration test execution (full OWASP checklist) |

### 5.3 P2 Items

| ID | Item | Description |
|----|------|-------------|
| AX-012 | Monitor Tauri GTK4 migration for desktop |
| AX-013 | Dependabot auto-merge for patch updates |
| AX-014 | Reproducible builds (pin toolchain in Nix/Dockerfile) |
| AX-015 | SBOM automation on release |

---

## 6. Phase 5: Production Release v3.0 (Sprint AY)

**Goal:** Ship production-hardened v3.0.0.
**Duration:** 1 week | **Dependencies:** Phases 1-4

### 6.1 Release Criteria (ALL must be satisfied)

- [ ] Zero P0 items from Phases 1-4 remaining
- [ ] 95%+ branch coverage on critical paths (storage, auth, WebDAV)
- [ ] 80%+ overall branch coverage
- [ ] Zero critical or high CVEs in dependency tree
- [ ] All endpoints documented in API reference
- [ ] Upgrade guide from v2.x to v3.0
- [ ] 24h soak test passed with zero panics or data loss
- [ ] Multi-arch release (linux-amd64, linux-arm64, macos-arm64, windows)
- [ ] Docker image published to ghcr.io with multi-arch manifest
- [ ] Helm chart for Kubernetes deployment
- [ ] Independent security review completed (internal or external)

### 6.2 Release Artifacts

| Artifact | Format | Platforms |
|----------|--------|-----------|
| Server binary | Static (musl) | linux-amd64, linux-arm64 |
| CLI binary | Static (musl) | linux-amd64, linux-arm64, macos-arm64, windows-msvc |
| Docker image | OCI multi-arch | linux/amd64, linux/arm64 |
| Helm chart | Helm v3 | Any K8s cluster |
| SBOM | SPDX JSON | Bundled with release |

### 6.3 Versioning Strategy

- Pre-release: `v3.0.0-beta.1`, `v3.0.0-rc.1`
- Stable: `v3.0.0`
- Maintenance: `v3.0.1`, `v3.0.2` (bug fixes only)
- Minor: `v3.1.0` (new features, backward compatible)

### 6.4 Success Metrics

| Metric | Target |
|--------|--------|
| Test coverage (critical paths) | >95% branch |
| Test coverage (overall) | >80% branch |
| Clippy warnings | 0 |
| Critical CVEs | 0 |
| API documentation completeness | 100% of endpoints |
| Docker image size (compressed) | <50MB |
| p99 latency (1KB PUT, local storage) | <10ms |
| p99 latency (PROPFIND, 1000 items) | <100ms |
| Concurrent connections (local storage) | >1000 |
| rclone E2E compatibility | 100% of Class 1/2/3 operations |
| Soak test duration | 24h zero-defect |

---

## 7. Phase 6: Post-v3.0 Growth (v3.1 - v3.5)

### 7.1 Desktop Client (v3.1) -- 4 weeks

| Item | Priority | Description |
|------|----------|-------------|
| File sync daemon | P0 | Background sync with conflict resolution |
| Selective sync | P1 | Per-folder sync toggle |
| System tray indicator | P1 | Sync status, recent changes, pause/resume |
| macOS universal binary | P1 | Intel + Apple Silicon support |
| Windows MSI installer | P1 | Shell integration, file associations |

### 7.2 Mobile (v3.2) -- 4 weeks

| Item | Priority | Description |
|------|----------|-------------|
| iOS file provider | P1 | Files app integration via File Provider extension |
| Android SAF provider | P1 | Storage Access Framework for Android |
| Offline mode | P2 | Local cache with conflict resolution |
| Push notifications | P2 | Share received, quota warning |

### 7.3 Collaboration (v3.3) -- 3 weeks

| Item | Priority | Description |
|------|----------|-------------|
| Real-time co-editing | P1 | CRDT-based via WebRTC |
| Comments and annotations | P2 | Per-file comment threads |
| File locking UI | P2 | Visual indicator when file is locked |
| Activity notifications | P2 | Email/webhook on share, comment |

### 7.4 Admin and Compliance (v3.4) -- 2 weeks

| Item | Priority | Description |
|------|----------|-------------|
| Admin dashboard in web UI | P0 | User management, storage stats, audit log viewer |
| Two-factor authentication | P1 | TOTP for admin and user accounts |
| SSO/SAML | P2 | SAML 2.0 service provider |
| GDPR data export | P2 | All user data in machine-readable format |
| Data retention policies | P2 | Automatic deletion past retention period |

### 7.5 Performance (v3.5) -- 2 weeks

| Item | Priority | Description |
|------|----------|-------------|
| True streaming uploads | P0 | No full buffering before write |
| Ranged GET with caching | P1 | `Range` header for partial content |
| Thumbnail cache | P1 | Persistent cache with LRU eviction |
| Search index sharding | P2 | Partition Tantivy for >1M files |
| Connection pooling | P2 | Configurable pool for cloud backends |

---

## 8. Phase 7: Platform Evolution (v4.0+)

### 8.1 Plugin System (v4.0)

- Stable WASM plugin API with versioned ABI
- Plugin marketplace (thumbnails, antivirus, OCR)
- Capability-based security model for WASM sandbox
- Hot-reload (load/unload without restart)

### 8.2 Multi-Tenant (v4.1)

- Organization support with per-org storage, quotas, policies
- Per-tenant rate limits, connection pools, storage backends
- Cross-org sharing with access control

### 8.3 Distributed Storage (v4.2)

- Reed-Solomon erasure coding for data durability
- Geo-replication between data centers
- Raft-based metadata consensus

### 8.4 AI Integration (v4.3)

- Vector embeddings for semantic search
- ML-based content classification and auto-tagging
- OCR for image/PDF text extraction
- Perceptual hashing for near-duplicate detection

---

## 9. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Tauri GTK4 migration delayed | Medium | Low (desktop-only) | Server/core unaffected; continue with GTK3 |
| `rsa` crate cannot be eliminated | Low | Medium | Isolate MySQL/age code paths |
| SQLite scaling under high concurrency | Medium | High | Recommend PostgreSQL for >100 concurrent users |
| Leptos 0.7 breaking changes | Medium | Medium | Pin version; plan migration window |
| WASM plugin ABI instability | High | Low (future) | Design with versioned ABI from start |
| Performance regression from WASM overhead | Medium | Medium | Benchmark with representative workloads |

---

## 10. Sprint Estimation

| Phase | Sprint | Duration | Dependencies | Parallelizable With |
|-------|--------|----------|--------------|-------------------|
| Phase 1 | AU | 3 weeks | None | Phase 3 (tests) |
| Phase 2 | AV | 2 weeks | Phase 1 | Phase 4 (security) |
| Phase 3 | AW | 3 weeks | Phase 1 | Phase 4 (security) |
| Phase 4 | AX | 2 weeks | Phase 1 | Phase 3 (tests) |
| Phase 5 | AY | 1 week | Phases 1-4 | -- |
| Phase 6.1 | AZ | 4 weeks | Phase 5 | Phase 6.2-6.5 |
| Phase 6.2 | BA | 4 weeks | Phase 6.1 | Phase 6.3 |
| Phase 6.3 | BB | 3 weeks | Phase 5 | Phase 6.4 |
| Phase 6.4 | BC | 2 weeks | Phase 5 | Phase 6.5 |
| Phase 6.5 | BD | 2 weeks | Phase 5 | -- |
| Phase 7+ | BE+ | Ongoing | Phase 5 | -- |

**Estimated time to v3.0:** 11 weeks (assuming full-time development)

---

## 11. Technical Debt Register

| ID | Description | Severity | Status | Planned Fix |
|----|-------------|----------|--------|------------|
| TD-001 | `expect()` in `hash_password()` | Medium | **RESOLVED** | 2026-05-20 |
| TD-002 | DashMap in-memory storage loses data on restart | Medium | Documented | Use `--data-dir` |
| TD-003 | `rsa` crate in dependency tree (RUSTSEC-2023-0071) | High | Active | Phase 4.3 |
| TD-004 | 22 Tauri/GTK3 advisory ignores | Low | Monitoring | Phase 6.1 |
| TD-005 | No fuzzing infrastructure | Medium | Open | Phase 3.3 |
| TD-006 | CalDAV/CardDAV not fully tested | Low | Open | Phase 3 |
| TD-007 | Desktop crate has no CI build | Low | Open | Phase 6.1 |
| TD-008 | Benchmark regression threshold 150% | Low | Open | Phase 2 |
| TD-009 | `utoipa-swagger-ui` requires network | Low | Open | Phase 2 |
| TD-010 | docker-compose files use `latest` tags | Low | Open | Phase 2 |
| TD-011 | 11 `#[allow(dead_code)]` in fuse crate | Low | Open | Phase 2 |
| TD-012 | No CSRF protection | Low | Open | Phase 4 |
| TD-013 | No account lockout | Low | Open | Phase 1 |
| TD-014 | No OIDC token refresh | Low | Open | Phase 1 |
| TD-015 | No streaming uploads | Medium | Open | Phase 6.5 |
| TD-016 | No SRI hashes on static assets | Low | Open | Phase 4 |

---

## 12. Decision Log

| ADR ID | Title | Date | Status |
|--------|-------|------|--------|
| ADR-001 | Zero-production-unwrap policy | 2026-04-26 | Accepted |
| ADR-002 | Token-bucket rate limiting over sliding window | 2026-05-20 | Accepted |
| ADR-003 | SQLite as default persistence (single-file) | 2026-05-15 | Accepted |
| ADR-004 | hash_password returns Result instead of panicking | 2026-05-20 | Accepted |
| ADR-005 | mdBook for documentation over raw markdown | 2026-04-01 | Accepted |
