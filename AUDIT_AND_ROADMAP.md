# Ferro Production Roadmap: Audit Findings and Path Forward

**Date:** 2026-05-14 | **Version:** 2.5.1 | **Status:** Active Development

---

## 1. Comprehensive Audit Summary

### 1.1 Test Results

| Metric | Value |
|--------|-------|
| Workspace crates | 20 |
| Total tests | 814 passed, 1 ignored (persistence doctest) |
| Clippy warnings (default) | 0 |
| Clippy warnings (all-features) | 0 (fixed) |
| cargo fmt | Clean |
| mdBook documentation build | Clean |
| Pre-commit hook | Operational (fmt, clippy, test, cargo-deny) |

### 1.2 CI/CD Status

The CI pipeline was non-functional (0s parsing failure on every push). Root cause: GitHub Actions rejects workflow files exceeding approximately 160 lines. The workflow has been split into:

| Workflow | File | Jobs | Status |
|----------|------|------|--------|
| Core checks | `.github/workflows/checks.yml` | 8 (fmt, clippy, test, cloud, pg, audit, build, docker) | Working |
| Extended checks | `.github/workflows/extended-checks.yml` | 2 (e2e, coverage) | Working |
| Benchmarks | `.github/workflows/bench.yml` | 1 | Working |
| Release | `.github/workflows/release.yml` | 3 (build, release, docker) | Fixed (QEMU, cross-compile, default-features) |
| Docs | `.github/workflows/docs.yml` | 2 | Fails (GitHub Pages not enabled on repo) |

**CI Job Results (latest run):**

| Job | Result | Notes |
|-----|--------|-------|
| Rustfmt | PASS | Clean |
| Clippy | PASS | All features (s3,gcs,azure,pg,redis,ldap) |
| Test | PASS | 814 tests, PostgreSQL service |
| Cloud (s3) | PASS | |
| Cloud (gcs) | PASS | |
| Cloud (azure) | PASS | |
| PostgreSQL | PASS | pg feature |
| Build Release | PASS | Artifacts uploaded |
| Security Audit | FAIL | cargo-audit exit code 2 (advisories in dependency tree) |
| Docker Build | FAIL | Dockerfile issue: rustup install in multi-stage build fails |

### 1.3 Code Quality Audit Findings

| Category | Critical | High | Medium | Low |
|----------|:--------:|:----:|:------:|:---:|
| Stubs/TODO/unimplemented | 0 | 0 | 0 | 0 |
| Production unwrap() | 2 | 6 | 0 | 0 |
| Production expect() | 0 | 4 | 4 | 5 |
| Unsafe blocks | 0 | 1 | 1 | 1 |
| Dead code (#[allow(dead_code)]) | 0 | 0 | 0 | 30 |
| Error handling (swallowed errors) | 1 | 3 | 5 | 1 |
| Concurrency safety | 0 | 1 | 2 | 2 |
| Determinism issues | 0 | 0 | 0 | 5 |
| Performance anti-patterns | 0 | 0 | 2 | 3 |
| Missing test coverage | 2 | 2 | 2 | 4 |
| **Total** | **5** | **17** | **15** | **51** |

### 1.4 Documentation Audit Findings

| Category | Count |
|----------|:-----:|
| No emojis | Confirmed (clean) |
| Stale test counts (790 -> 814) | Fixed |
| Stale sprint count (25 -> 46) | Fixed |
| Closed false tech debt (TD-014 rate limiter) | Fixed |
| Endpoint count contradiction (90+ vs 70+) | Open |
| CONTRIBUTING.md publish status wrong | Open |
| Missing ADR documents | Open |
| Missing 8 crate documentation pages in mdBook | Open |
| ferro.toml.example incomplete | Open |
| GitHub Pages not enabled (docs deploy fails) | Open |

---

## 2. Critical Issues Requiring Immediate Action

### 2.1 Production unwrap() Audit (P0)

**Location:** `crates/server/src/webdav.rs` (136 instances), `crates/server/src/backup.rs` (40), `crates/auth/src/simple_auth.rs` (28)

A single malformed HTTP request can crash the server. Systematic migration from `.unwrap()` to `?` operator or proper error responses is required for production safety.

**Approach:** Prioritize by exposure:
1. Auth middleware (`simple_auth.rs`) -- external input, P0
2. WebDAV handler (`webdav.rs`) -- external input, P0
3. API handlers (`api.rs`) -- external input, P0
4. Backup/restore (`backup.rs`) -- admin-only, P1
5. Internal modules -- P2

### 2.2 Error Handling: Swallowed Errors (P0)

**`server/src/api.rs:248`** -- Password update failure is logged but HTTP 200 returned. User believes password changed but it did not.

**`server/src/main.rs:560`** -- Mutex poison silently recovered via `unwrap_or_else(|e| e.into_inner())`. Potential data corruption if thread panicked while holding SQLite lock.

**`server/src/backup.rs`** -- Partial backup/restore failures reported as success.

### 2.3 CI Pipeline Stability (P0)

The checks.yml workflow has a hard limit of approximately 160 lines. This is a GitHub Actions limitation. The current 8-job split works. The extended checks (e2e, coverage, wasm, rclone-e2e) are in a separate workflow file.

**Additional CI failures to fix:**
- Security Audit: `cargo audit` fails with exit code 2 (advisories in transitive deps)
- Docker Build: Multi-stage Dockerfile's rustup install step fails on CI runners

### 2.4 Dockerfile Repair (P0)

The Dockerfile fails to build because the multi-stage Rust installation step fails. The container needs either a pre-built binary or a different installation strategy.

---

## 3. Path to v3.0.0 Release

### Phase 1: Safety-Critical Fixes (Week 1-2)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| FIX-001 | Migrate auth middleware unwrap() to error responses | P0 | 2d |
| FIX-002 | Migrate WebDAV handler unwrap() to error responses | P0 | 3d |
| FIX-003 | Fix password update error propagation (api.rs:248) | P0 | 0.5d |
| FIX-004 | Handle Mutex poison in main.rs:560 properly | P0 | 0.5d |
| FIX-005 | Fix partial backup/restore error reporting | P1 | 1d |
| FIX-006 | Fix Dockerfile multi-stage build | P0 | 1d |
| FIX-007 | Fix security audit CI job (update deny.toml) | P1 | 0.5d |
| FIX-008 | Add panic handler to HTTP request handlers | P1 | 1d |

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
| PROD-005 | Panic handler: catch panics in handlers, return 500 | P1 | 1d |
| PROD-006 | OIDC token silent refresh before expiry | P1 | 2d |
| PROD-007 | LDAP group-to-Cedar-role mapping | P2 | 2d |
| PROD-008 | FFI layer: shared tokio Runtime, bounds validation | P1 | 1d |
| PROD-009 | Replace std::sync::Mutex with parking_lot::Mutex | P1 | 0.5d |
| PROD-010 | Replace thread::sleep with tokio::time::sleep | P1 | 0.5d |

### Phase 4: Documentation (Week 4-5)

| ID | Item | Priority | Effort |
|----|------|----------|--------|
| DOC-001 | Enable GitHub Pages for docs deployment | P0 | 0.5d |
| DOC-002 | Add mdBook pages for 8 undocumented crates | P1 | 3d |
| DOC-003 | Complete all endpoint documentation in api.md | P0 | 2d |
| DOC-004 | Fix endpoint count contradiction (90+ vs 70+) | P1 | 0.5d |
| DOC-005 | Fix CONTRIBUTING.md publish status | P1 | 0.5d |
| DOC-006 | Expand ferro.toml.example with all documented keys | P1 | 1d |
| DOC-007 | Create ADR documents (ADR-001 through ADR-005) | P1 | 2d |

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

### v3.2: Mobile (4 weeks)

| Item | Description |
|------|-------------|
| iOS File Provider | Files app integration via FFI bindings |
| Android SAF provider | Storage Access Framework |
| Offline mode | Local cache with conflict resolution |
| Push notifications | Share received, quota warning |

### v3.3: Collaboration (3 weeks)

| Item | Description |
|------|-------------|
| Real-time co-editing | CRDT-based via WebRTC data channels |
| Per-file comments | Comment threads with notifications |
| Activity notifications | Email/webhook on share, comment, mention |

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

## 5. Technical Debt Register

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| TD-001 | CI workflow parsing failure (GitHub Actions ~160 line limit) | Critical | Resolved (split into 2 files) |
| TD-002 | `rsa` crate RUSTSEC-2023-0071 (sqlx-mysql transitive) | High | Open |
| TD-003 | `glib` RUSTSEC-2024-0429 unsound (Tauri/GTK3 transitive) | High | Documented (desktop-only) |
| TD-004 | 20 unmaintained advisories (GTK3/Tauri chain) | Low | Monitored |
| TD-005 | README missing 14 CLI flags documented in configuration.md | Medium | Open |
| TD-006 | README missing 40+ API endpoints | Medium | Open |
| TD-007 | `deploy/Dockerfile.web` and `deploy/Dockerfile.admin` do not exist | Medium | Open |
| TD-008 | Dockerfile multi-stage build fails | Critical | Open |
| TD-009 | No redis or ldap feature CI test coverage | Medium | Open |
| TD-010 | `bincode` 1.3.3 unmaintained (fuse3 transitive) | Low | Monitored |
| TD-011 | `rustls-pemfile` 2.2.0 unmaintained (object_store transitive) | Low | Monitored |
| TD-012 | Benchmark regression threshold too lenient (150%) | Low | Open |
| TD-013 | `e2e/package-lock.json` missing | Low | Open |
| TD-014 | Rate limiter docs vs implementation mismatch | Low | Closed (docs were correct) |
| TD-015 | Production unwrap() audit (~300 instances across 7 files) | Critical | Open |
| TD-016 | Mutex poison silent recovery (main.rs:560) | High | Open |
| TD-017 | Password update error swallowed (api.rs:248) | Critical | Open |
| TD-018 | FFI layer creates new tokio Runtime per call | Medium | Open |
| TD-019 | Observability crate 7% test coverage | High | Open |
| TD-020 | Web crate (Leptos) 7% test coverage | High | Open (WASM testing complex) |

---

## 6. Effort Estimation

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

## 7. Key Architecture Decisions

### ADR-NEXT-001: CI Workflow Split
**Status:** Accepted | **Date:** 2026-05-14

GitHub Actions silently rejects workflow files exceeding approximately 160 lines (no error message, 0s duration). Solution: split into `checks.yml` (core quality gates, 8 jobs) and `extended-checks.yml` (e2e, coverage, 2 jobs).

### ADR-NEXT-002: Pre-commit Hook Scope
**Status:** Accepted | **Date:** 2026-05-14

The pre-commit hook runs `cargo test --workspace` which compiles all 20 crates. This takes approximately 2 minutes. The hook skips entirely when no `.rs` or `.toml` files are staged. This is acceptable for the current codebase size but should be revisited if compile times increase.

### ADR-NEXT-003: Action Version Pinning
**Status:** Accepted | **Date:** 2026-05-14

All GitHub Actions pinned to v4 (checkout, cache, artifact). v5+ versions exist but cause workflow parsing failures. Node.js 20 deprecation warnings noted but non-blocking until September 2026.

### ADR-NEXT-004: Clippy All-Features Gate
**Status:** Accepted | **Date:** 2026-05-14

CI runs `cargo clippy --all-targets --features "s3,gcs,azure,pg,redis,ldap"` which catches warnings only visible with feature flags enabled. This is stricter than the local pre-commit hook which only runs `--workspace`. Found and fixed 4 warnings (needless_borrows_for_generic_args, let_and_return) that were invisible in default mode.
