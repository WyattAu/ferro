# Ferro Roadmap v4: Consolidated Path to Production

**Version:** 2.5.1 | **Date:** 2026-05-15 | **Author:** Nexus (Principal Systems Architect)

---

## 0. Audit Summary (2026-05-15)

### Codebase Health

| Metric | Value | Assessment |
|--------|-------|------------|
| Workspace crates | 21 | All implemented, zero stubs |
| Total Rust LOC | ~71,852 | |
| Unit tests | 813 passed, 0 failed, 1 ignored | |
| Integration tests | 126 passed | |
| Clippy warnings | 0 | Clean |
| rustfmt | Clean | |
| cargo-deny | PASS (advisories, bans, licenses, sources) | Duplicate crate warnings only (transitive) |
| Pre-commit hooks | Active (fmt, clippy, test, deny) | |
| CI/CD (Checks) | 10/10 green | |
| CI/CD (Docs) | Green | |
| CI/CD (Benchmarks) | Requires bench-data branch init | |
| CI/CD (E2E) | Docker-based, needs test update | |

### Issues Found and Fixed

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| CI-001 | Extended Checks E2E used `cargo build --release` (timeout >120s) | Critical | Fixed: dev build |
| CI-002 | Benchmarks referenced non-existent `gh-pages` branch | Critical | Fixed: bench-data branch |
| CI-003 | Docs workflow curl pipe broken for mdBook download | High | Fixed: wget |
| CI-004 | E2E tests hardcoded `localhost:8080` in browser context | Medium | Fixed: env var |
| CI-005 | E2E tests need WASM UI (requires trunk build) | High | Fixed: Docker image |
| CI-006 | CORS wildcard rejected when auth enabled | High | Fixed: explicit origin |
| DOC-001 | Rust MSRV listed as 1.85 in 5 files | Medium | Fixed: 1.92 |
| DOC-002 | init_requirements.md claimed SSR (actual: CSR) | Medium | Fixed |
| DOC-003 | init_requirements.md described rclone sidecar (actual: FUSE) | Medium | Fixed |
| DOC-004 | Test count 814 in ROADMAP (actual: 813) | Low | Fixed |
| DOC-005 | Crate count 20 in ROADMAP (actual: 21) | Low | Fixed |

### Outstanding Issues (Not Fixed)

| ID | Description | Severity | Planned Fix |
|----|-------------|----------|-------------|
| E2E-001 | 24 E2E tests fail (UI elements mismatch with current WASM build) | Medium | Sprint AV |
| TD-001 | ~300 `unwrap()` calls in server crate (all on internal data, not external input) | Medium | Phase 2.4 |
| TD-003 | `rsa` crate in transitive deps (RUSTSEC-2023-0071, via sqlx-mysql) | Medium | Phase 4.3 |
| TD-010 | Docker compose files use `latest` tags | Low | Pin to SHA |
| DENY-001 | 16 duplicate crate versions in dependency tree (thiserror, toml, etc.) | Low | Natural attrition |

---

## 1. Phase AU: Production Hardening (3 weeks)

### 1.1 Critical (P0) -- Must ship for v3.0

| Item | Description | Verification |
|------|-------------|--------------|
| E2E test alignment | Update 24 Playwright tests to match current WASM UI selectors | All 24 pass |
| Atomic file writes | Write to temp file, then rename on PUT | No partial uploads after crash |
| WAL mode for SQLite | `PRAGMA journal_mode=WAL` on all connections | Concurrent read/write verified |
| Config validation | Reject invalid combinations (CORS `*` + auth) at startup | Server refuses to start with bad config |
| Secret redaction | Never log passwords, tokens, API keys | Grep audit of all logging |
| Enforce password change | Reject default `changeme` on first login | 401 with specific error code |
| Rate limit login | 5 attempts/min per IP on `/auth/login` | 429 after 5 failures |
| Health check deepening | Verify storage backend connectivity in `/healthz` | Returns 503 if storage dead |

### 1.2 High Priority (P1)

| Item | Description |
|------|-------------|
| Database backup API | Admin endpoint to trigger SQLite backup download |
| Checksum verification | Verify CAS store integrity on boot |
| OIDC token refresh | Silent refresh before expiry |
| Session token rotation | Re-issue on password change |
| Config schema versioning | Pin `ferro.toml` schema version; migrate on upgrade |
| Expand ferro.toml.example | Add all 37 documented keys |
| Trash auto-purge | Background task for `--trash-ttl` enforcement |

---

## 2. Phase AV: Reliability and Observability (2 weeks)

### 2.1 Structured Logging

| Item | Description |
|------|-------------|
| Request tracing | Propagate `X-Request-ID` through all log lines |
| Per-crate log level | `FERRO_LOG=ferro_server=debug,ferro_core=trace` |
| Slow query logging | Log SQLite queries >100ms |

### 2.2 Metrics

| Item | Description |
|------|-------------|
| Prometheus endpoint | Request latency histograms, error rates, active connections |
| Storage backend metrics | PUT/GET latency per backend, cache hit/miss |
| Grafana dashboard | JSON template for common views |

### 2.3 Error Handling

| Item | Description |
|------|-------------|
| Reduce unwrap count | Target: zero unwraps on external input paths |
| Global panic handler | Catch panics in request handlers; return 500 |
| Graceful degradation | If search fails, serve files without search |

---

## 3. Phase AW: Test Coverage Expansion (3 weeks)

### 3.1 Property-Based Testing

| Item | Description |
|------|-------------|
| Storage properties | `proptest`: PUT then GET returns identical content for random bytes |
| Path normalization | No path escapes after N random transformations |
| Lock protocol | Exhaustive state machine testing |
| XML parsing | Proptest-generated XML to WebDAV parser |

### 3.2 Fuzzing

| Item | Description |
|------|-------------|
| WebDAV request fuzzer | `cargo-fuzz` targeting WebDAV handler |
| XML parser fuzzer | Fuzz PROPFIND/PROPPATCH bodies |

### 3.3 Load Testing

| Item | Description |
|------|-------------|
| Concurrent upload benchmark | 100+ simultaneous PUTs |
| Large directory listing | PROPFIND with 10,000+ entries |
| 24h soak test | Continuous random operations, zero panics |

---

## 4. Phase AX: Security Certification (2 weeks)

### 4.1 Authentication Security

| Item | Description |
|------|-------------|
| CSRF protection | Double-submit cookie or SameSite=Strict |
| Secure cookie flags | HttpOnly, Secure, SameSite on session tokens |
| CSP hardening | Remove `'unsafe-inline'`; use nonce-based styles |
| SRI hashes | Subresource integrity on CDN assets |

### 4.2 Dependency Security

| Item | Description |
|------|-------------|
| Eliminate `rsa` crate | Replace sqlx-mysql path or isolate |
| Resolve duplicate crates | Address 16 duplicate versions in deny.toml |
| Pin docker-compose tags | Replace `latest` with SHA |
| SBOM automation | Auto-generate SPDX on release |

---

## 5. Phase AY: v3.0 Release (1 week)

### Release Criteria

All P0 items from Phases 1-4 must be satisfied:

- [ ] 95%+ branch coverage on critical paths
- [ ] 80%+ overall branch coverage
- [ ] Zero critical/high CVEs in dependency tree
- [ ] 24 E2E tests passing
- [ ] 24h soak test: zero panics, zero data loss
- [ ] Multi-arch release: linux-amd64, linux-arm64, macos-arm64
- [ ] Docker image <50MB compressed on ghcr.io
- [ ] Helm chart for Kubernetes

### Release Artifacts

| Artifact | Platforms |
|----------|-----------|
| Server binary (musl) | linux-amd64, linux-arm64 |
| CLI binary | linux-amd64, macos-arm64, windows |
| Docker image (multi-arch) | linux/amd64, linux/arm64 |
| Helm chart | Any Kubernetes |
| SBOM (SPDX) | Bundled |

---

## 6. Post-v3.0 Growth

### 6.1 Desktop Client (v3.1) -- 4 weeks

| Item | Priority |
|------|----------|
| File sync daemon with conflict resolution | P0 |
| Selective sync per-folder | P1 |
| System tray indicator | P1 |
| Windows MSI installer | P1 |
| macOS universal binary | P1 |

### 6.2 Mobile (v3.2) -- 4 weeks

| Item | Priority |
|------|----------|
| iOS Files app integration | P1 |
| Android SAF provider | P1 |
| Offline mode with conflict resolution | P2 |

### 6.3 Collaboration (v3.3) -- 3 weeks

| Item | Priority |
|------|----------|
| Real-time co-editing via CRDT/WebRTC | P1 |
| Per-file comment threads | P2 |
| File locking visual indicator | P2 |

### 6.4 Admin and Compliance (v3.4) -- 2 weeks

| Item | Priority |
|------|----------|
| Admin dashboard (users, storage, audit) in web UI | P0 |
| TOTP two-factor authentication | P1 |
| GDPR data export | P2 |
| Data retention policies | P2 |

### 6.5 Performance (v3.5) -- 2 weeks

| Item | Priority |
|------|----------|
| Streaming uploads (no full buffering) | P0 |
| Ranged GET with caching | P1 |
| Persistent thumbnail cache (LRU) | P1 |
| Search index sharding (>1M files) | P2 |

---

## 7. Platform Evolution (v4.0+)

### 7.1 Plugin System (v4.0)

- Versioned WASM plugin ABI
- Capability-based security sandbox
- Plugin marketplace registry
- Hot-reload without restart

### 7.2 Multi-Tenant (v4.1)

- Per-org storage, quotas, policies
- Resource isolation (rate limits, connection pools)
- Cross-org sharing controls

### 7.3 Distributed Storage (v4.2)

- Reed-Solomon erasure coding
- Geo-replication between data centers
- Raft-based metadata consensus

### 7.4 AI Integration (v4.3)

- Semantic search via vector embeddings
- ML-based content classification and auto-tagging
- OCR for image/PDF full-text extraction
- Perceptual hashing for near-duplicate detection

---

## 8. Technical Debt Register

| ID | Description | Severity | Target Phase |
|----|-------------|----------|--------------|
| TD-001 | ~300 `unwrap()` in server (internal data only) | Medium | AU |
| TD-002 | DashMap in-memory loses data on restart | Info | Document |
| TD-003 | `rsa` transitive (RUSTSEC-2023-0071) | Medium | AX |
| TD-004 | 20 GTK3 unmaintained advisories (desktop) | Low | Monitor |
| TD-005 | No fuzzing infrastructure | Medium | AW |
| TD-006 | CalDAV/CardDAV incomplete | Low | Future |
| TD-007 | Desktop crate has no CI build | Low | v3.1 |
| TD-008 | Benchmark regression threshold 150% (too lenient) | Low | Reduce to 120% |
| TD-009 | `utoipa-swagger-ui` requires network at build | Low | Vendor |
| TD-010 | Docker compose `latest` tags | Low | AX |
| TD-011 | 16 duplicate crate versions in lockfile | Low | Dependabot |
| TD-012 | E2E tests need update for current UI | Medium | AU |

---

## 9. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| E2E test alignment takes longer than expected | High | Medium | Pin to known UI selectors; parallel web+test dev |
| WASM plugin ABI instability | High | Low | Design with versioned ABI from start |
| Performance regression with SQLite at scale | Medium | High | Document PostgreSQL recommendation for >100 users |
| Leptos 0.7 breaking changes | Medium | Medium | Pin version; plan migration window |
| `rsa` crate cannot be eliminated | Low | Medium | Isolate MySQL code paths; document risk |
| Tauri GTK4 migration delayed | Medium | Low | Server/core unaffected; continue GTK3 |

---

## 10. Success Metrics for v3.0

| Metric | Current | Target |
|--------|---------|--------|
| Test coverage (critical paths) | ~70% est. | >95% branch |
| Test coverage (overall) | ~60% est. | >80% branch |
| Clippy warnings | 0 | 0 |
| Critical CVEs | 0 | 0 |
| E2E tests passing | 0/24 | 24/24 |
| Docker image size | TBD | <50MB compressed |
| p99 latency (1KB PUT) | TBD | <10ms |
| p99 latency (PROPFIND 1000) | TBD | <100ms |
| Soak test | Not run | 24h zero-defect |
| rclone compatibility | Partial | Full Class 1/2/3 |

---

## 11. Sprint Estimation

| Phase | Sprint | Duration | Dependencies |
|-------|--------|----------|--------------|
| AU | Production Hardening | 3 weeks | None |
| AV | Reliability | 2 weeks | AU |
| AW | Test Coverage | 3 weeks | AU |
| AX | Security | 2 weeks | AU |
| AY | v3.0 Release | 1 week | AU-AX |
| AZ | Desktop v3.1 | 4 weeks | AY |
| BA | Mobile v3.2 | 4 weeks | AZ |
| BB | Collaboration v3.3 | 3 weeks | AY |
| BC | Admin v3.4 | 2 weeks | AY |
| BD | Performance v3.5 | 2 weeks | AY |
| BE+ | Platform v4.0+ | Ongoing | AY |

**Time to v3.0: 11 weeks** (assuming full-time development on production path)
**Time to v3.5: 21 weeks**
