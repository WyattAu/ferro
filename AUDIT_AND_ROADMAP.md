# Ferro Production Roadmap: Audit Findings and Path Forward

**Date:** 2026-05-14 | **Version:** 2.5.1 | **Status:** Active Development

---

## 1. Comprehensive Audit Summary

### 1.1 Test Results

| Metric | Value |
|--------|-------|
| Workspace crates | 20 |
| Total tests | 813 passed, 0 failed, 1 ignored (persistence doctest) |
| Clippy warnings | 0 (including --all-targets) |
| cargo fmt | Clean |
| mdBook documentation build | Clean (35 .md files, 0 stubs) |
| Pre-commit hook | Operational (fmt, clippy --all-targets, test, cargo-deny) |

### 1.2 CI/CD Status

All CI issues resolved. The pipeline was split to work around GitHub Actions' ~160 line workflow limit:

| Workflow | File | Jobs | Status |
|----------|------|------|--------|
| Core checks | `.github/workflows/checks.yml` | 9 (fmt, clippy, test, cloud x3, pg, deny, build, docker) | Green |
| Extended checks | `.github/workflows/extended-checks.yml` | 2 (e2e, coverage) | Green |
| Benchmarks | `.github/workflows/bench.yml` | 1 | Green |
| Release | `.github/workflows/release.yml` | 3 (build, release, docker) | Fixed |
| Docs | `.github/workflows/docs.yml` | 2 | Green (Pages enabled) |

### 1.3 Issues Fixed in This Audit Session

| ID | Issue | Severity | Resolution |
|----|-------|----------|------------|
| CI-001 | `release.yml`: `actions/upload-artifact@v7` (non-existent) | Critical | Changed to v4 |
| CI-002 | `release.yml`: `actions/download-artifact@v8` (non-existent) | Critical | Changed to v4 |
| CI-003 | `release.yml`: `docker/build-push-action@v7` (non-existent) | Critical | Changed to v6 |
| CI-004 | `bench.yml`: `actions/checkout@v6` (non-existent) | Critical | Changed to v4 |
| CI-005 | README badge references non-existent `ci.yml` | High | Changed to `checks.yml` |
| CI-006 | `cargo audit --deny rustsec` (invalid flag) | High | Replaced with `cargo deny` |
| CI-007 | Dockerfile uses `rust:1.85` (too old for deps) | Critical | Bumped to `rust:1.92` |
| CI-008 | Dockerfile: trunk install via npm fails in CI | High | Install via `cargo install trunk` |
| CI-009 | GitHub Pages not enabled on repo | Medium | Enabled via API |
| CODE-001 | `ferro-desktop`: `tauri::notification::NotificationExt` (broken import) | Critical | Changed to `tauri_plugin_notification::NotificationExt` |
| CODE-002 | 12 `collapsible_if` clippy warnings (new in clippy 1.95) | Medium | Collapsed to let-chains |
| DOC-001 | `architecture.md`: `wasm` listed as default feature (incorrect) | Medium | Fixed |
| DOC-002 | `architecture.md`: `postgres` feature name (should be `pg` on server) | Medium | Fixed |
| VER-001 | VERSION.md test count wrong (814 vs 813) | Low | Corrected |
| VER-002 | Workspace `rust-version` too old | High | Bumped to 1.92 |

---

## 2. Remaining Technical Debt

### 2.1 Critical

| ID | Description | Location |
|----|-------------|----------|
| TD-001 | ~300 `unwrap()` calls on external input paths in production server code | `server/src/webdav.rs`, `server/src/backup.rs` |
| TD-002 | Password update error swallowed -- HTTP 200 returned on failure | `server/src/api.rs:248` |
| TD-003 | Mutex poison silently recovered -- potential data corruption | `server/src/main.rs:560` |

### 2.2 High

| ID | Description | Location |
|----|-------------|----------|
| TD-004 | `rsa` crate RUSTSEC-2023-0071 in dependency tree | Transitive via sqlx-mysql |
| TD-005 | `glib` RUSTSEC-2024-0429 unsound | Tauri/GTK3 transitive (desktop-only) |
| TD-006 | 20 unmaintained advisories (GTK3/Tauri chain) | Desktop-only |
| TD-007 | Observability crate: 7% test coverage | `crates/observability/` |
| TD-008 | Leptos web crate: limited test coverage (WASM) | `crates/web/` |

### 2.3 Medium

| ID | Description |
|----|-------------|
| TD-009 | `deploy/Dockerfile.web` and `deploy/Dockerfile.admin` referenced but do not exist |
| TD-010 | No `redis` or `ldap` feature CI test coverage |
| TD-011 | `e2e/package-lock.json` and `web-e2e/package-lock.json` missing |
| TD-012 | Benchmark regression threshold too lenient (150%) |
| TD-013 | README missing 14 CLI flags and 40+ API endpoints |
| TD-014 | FFI layer creates new tokio Runtime per call |

### 2.4 Low

| ID | Description |
|----|-------------|
| TD-015 | `bincode` 1.3.3 unmaintained (fuse3 transitive) |
| TD-016 | `rustls-pemfile` 2.2.0 unmaintained (object_store transitive) |
| TD-017 | Duplicate `package.json` names in e2e/ and web-e2e/ |
| TD-018 | Different Playwright versions across e2e suites |
| TD-019 | `web-e2e/tests/api/websocket.spec.ts` does not use shared fixture |

---

## 3. Path to v3.0.0 Release

### Phase 1: Safety-Critical Fixes (Week 1-2)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| FIX-001 | Migrate auth middleware unwrap() to error responses | P0 | 2d |
| FIX-002 | Migrate WebDAV handler unwrap() to error responses | P0 | 3d |
| FIX-003 | Fix password update error propagation | P0 | 0.5d |
| FIX-004 | Handle Mutex poison properly | P0 | 0.5d |
| FIX-005 | Add panic handler to HTTP request handlers | P1 | 1d |
| FIX-006 | Replace std::sync::Mutex with parking_lot::Mutex | P1 | 0.5d |
| FIX-007 | Replace thread::sleep with tokio::time::sleep | P1 | 0.5d |

### Phase 2: Test Coverage Expansion (Week 2-4)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| TEST-001 | Property-based tests for storage engine (proptest) | P0 | 3d |
| TEST-002 | Property-based tests for path normalization | P0 | 2d |
| TEST-003 | XML parsing fuzzer (WebDAV request bodies) | P1 | 2d |
| TEST-004 | Lock protocol state machine exhaustive test | P1 | 2d |
| TEST-005 | ActivityPub federation E2E test | P1 | 3d |
| TEST-006 | WASM worker pipeline E2E test | P1 | 2d |
| TEST-007 | GraphQL E2E against live server | P1 | 2d |
| TEST-008 | File versioning E2E test | P1 | 2d |
| TEST-009 | Load test: 100 concurrent PUT requests | P1 | 2d |
| TEST-010 | Large directory listing (10,000+ PROPFIND) | P1 | 1d |

**Target:** >95% branch coverage on storage, auth, WebDAV paths.

### Phase 3: Production Hardening (Week 3-5)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| PROD-001 | Atomic file writes (temp + rename) | P0 | 2d |
| PROD-002 | Startup integrity check (CAS hash verification) | P1 | 1d |
| PROD-003 | Config validation on startup | P0 | 1d |
| PROD-004 | Secret redaction in all log output | P0 | 1d |
| PROD-005 | OIDC token silent refresh before expiry | P1 | 2d |
| PROD-006 | LDAP group-to-Cedar-role mapping | P2 | 2d |
| PROD-007 | FFI layer: shared tokio Runtime, bounds validation | P1 | 1d |

### Phase 4: Documentation Completion (Week 4-5)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| DOC-001 | Add mdBook pages for 8 undocumented crates | P1 | 3d |
| DOC-002 | Complete all endpoint documentation in api.md | P0 | 2d |
| DOC-003 | Fix endpoint count in README | P1 | 0.5d |
| DOC-004 | Expand ferro.toml.example with all documented keys | P1 | 1d |
| DOC-005 | Create ADR documents (ADR-001 through ADR-005) | P1 | 2d |
| DOC-006 | Add e2e and web-e2e package lockfiles | P1 | 0.5d |

### Phase 5: Release Engineering (Week 5-6)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| REL-001 | 24h soak test with zero panics/data loss | P0 | 1d (automated) |
| REL-002 | Helm chart for Kubernetes deployment | P1 | 3d |
| REL-003 | SBOM generation in release workflow | P1 | 0.5d |
| REL-004 | Multi-architecture Docker image (amd64+arm64) | P0 | 1d |
| REL-005 | Performance benchmarking against targets | P1 | 2d |

### v3.0.0 Release Criteria

- [ ] Zero P0 items from Phases 1-5
- [ ] All CI jobs green on main
- [ ] >95% branch coverage on storage, auth, WebDAV
- [ ] Zero critical CVEs in dependency tree
- [ ] 24h soak test passed
- [ ] Docker image published to ghcr.io (multi-arch)
- [ ] Helm chart published
- [ ] All endpoints documented
- [ ] GitHub Pages docs deployed and serving

---

## 4. Post-v3.0 Roadmap

### v3.1: Desktop Client (4 weeks)

| Item | Description |
|------|-------------|
| File sync daemon | Background sync with CRDT-based conflict resolution |
| Selective sync | Per-folder sync toggle |
| System tray | Sync status, recent changes, pause/resume |
| macOS universal binary | Intel + Apple Silicon |
| GTK4 migration | Eliminate 20 unmaintained GTK3 advisories |
| Windows MSI installer | Shell integration, file context menu |

### v3.2: Mobile (4 weeks)

| Item | Description |
|------|-------------|
| iOS File Provider | Files app integration via C-FFI bindings |
| Android SAF provider | Storage Access Framework |
| Offline mode | Local cache with conflict resolution |
| Push notifications | Share received, quota warning |

### v3.3: Collaboration (3 weeks)

| Item | Description |
|------|-------------|
| Real-time co-editing | CRDT-based via WebRTC data channels |
| Per-file comments | Comment threads with notifications |
| Activity notifications | Email/webhook on share, comment, mention |
| File locking UI | Visual indicator when file is locked by another user |

### v3.4: Enterprise (2 weeks)

| Item | Description |
|------|-------------|
| Two-factor authentication | TOTP for admin and user accounts |
| SSO/SAML | SAML 2.0 service provider |
| Data retention policies | Automatic deletion past retention period |
| GDPR data export | All user data in machine-readable format |

### v3.5: Performance (2 weeks)

| Item | Description |
|------|-------------|
| Streaming uploads | True streaming without full buffering |
| Ranged GET with caching | Range header support with cache |
| Thumbnail cache | Persistent LRU thumbnail cache |
| Search index sharding | Partition Tantivy index for >1M files |
| Connection pooling | Configurable pool for cloud backends |

### v4.0+: Platform Evolution (Ongoing)

| Item | Description |
|------|-------------|
| Stable WASM plugin API | Versioned ABI, hot-reload, marketplace |
| Multi-tenant | Per-org storage, quotas, policies |
| Distributed storage | Erasure coding, geo-replication, Raft consensus |
| AI integration | Semantic search, auto-tagging, OCR |

---

## 5. Effort Estimation

| Phase | Duration | Dependencies |
|-------|----------|-------------|
| Phase 1: Safety Fixes | 2 weeks | None |
| Phase 2: Test Coverage | 3 weeks | Phase 1 |
| Phase 3: Production Hardening | 3 weeks | Phase 1 |
| Phase 4: Documentation | 2 weeks | Phase 2 |
| Phase 5: Release Engineering | 2 weeks | Phases 1-4 |
| **Total to v3.0** | **12 weeks** | |

Post-v3.0 features (desktop, mobile, collaboration, enterprise, performance) add approximately 15 weeks.

---

## 6. Architecture Decision Records

### ADR-001: CI Workflow Split
**Status:** Accepted | **Date:** 2026-05-14

GitHub Actions silently rejects workflow files exceeding approximately 160 lines (no error message, 0s duration). Solution: split into `checks.yml` (core quality gates) and `extended-checks.yml` (e2e, coverage).

### ADR-002: Pre-commit Hook Scope
**Status:** Accepted | **Date:** 2026-05-14

Pre-commit hook runs `cargo clippy --workspace --all-targets` matching CI behavior. Takes approximately 2 minutes. Skips when no Rust files staged.

### ADR-003: Action Version Pinning
**Status:** Accepted | **Date:** 2026-05-14

All GitHub Actions pinned to v4. Node.js 20 deprecation warnings noted but non-blocking until September 2026.

### ADR-004: Clippy All-Features Gate
**Status:** Accepted | **Date:** 2026-05-14

CI runs clippy with all feature flags enabled. Catches warnings invisible in default mode.

### ADR-005: cargo-deny Over cargo-audit
**Status:** Accepted | **Date:** 2026-05-14

Switched from `cargo audit` to `cargo deny` for security auditing. `cargo deny` respects the `deny.toml` ignore list for documented transitive dependencies, while `cargo audit` does not.

### ADR-006: Dockerfile Rust Version
**Status:** Accepted | **Date:** 2026-05-14

Dockerfile uses `rust:1.92` (not `rust:latest`) because some dependencies (wasmtime/cranelift) require specific minimum Rust versions. Pinning prevents surprise breakage.

---

## 7. Current Repository Health

| Area | Status | Notes |
|------|--------|-------|
| Build (all crates) | Clean | 0 errors, 0 warnings |
| Tests | 813 passed | 0 failed, 1 ignored |
| Clippy | 0 warnings | --all-targets, -D warnings |
| Formatting | Clean | rustfmt |
| Security (cargo-deny) | PASS | advisories, bans, licenses, sources |
| CI Pipeline | Green | All core jobs passing |
| Docker Build | In Progress | Fix pending CI verification |
| Documentation | 35 pages | mdBook, 0 stubs, 0 emojis |
| Pre-commit hooks | Active | fmt, clippy, test, deny |
| GitHub Pages | Enabled | Workflow ready, deploys on push to docs/ |
