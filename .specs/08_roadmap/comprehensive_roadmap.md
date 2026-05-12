# Ferro Comprehensive Production Roadmap

**Version:** 2.5.1 | **Date:** 2026-05-12 | **Author:** Principal Systems Architect

---

## 1. Current State Assessment

### 1.1 Audit Results (2026-05-12)

| Metric | Value | Status |
|--------|-------|--------|
| Total tests | 809 (790 + 19 new graphql) | 0 failures |
| Clippy warnings | 0 | Clean |
| Format violations | 0 | Clean |
| cargo-deny advisories | 0 active, 3 documented ignores | Passed |
| Production `unwrap()` | 0 | All confined to tests |
| `todo!()` / `unimplemented!()` | 0 | None in codebase |
| Emojis in docs | 0 | All technical |
| Pre-commit hook | Active | fmt + clippy + test + deny |
| Test coverage (estimated) | ~70% overall, ~85% critical paths | Target: 80% / 95% |

### 1.2 Fixed in This Audit

| Issue | Status |
|-------|--------|
| `deny.toml`: 20 stale advisory ignores removed | Fixed |
| `deny.toml`: 2 stale licenses removed | Fixed |
| `SECURITY.md`: Auth mechanism corrected (Bearer -> HTTP Basic) | Fixed |
| `SECURITY.md`: Known vulnerabilities updated with current IDs | Fixed |
| `docs/api.md`: Health check response corrected (plain text -> JSON) | Fixed |
| `docs/api.md`: `/healthz`, `/readyz` endpoints documented | Fixed |
| `docs/api.md`: `auth_type` field added to auth info response | Fixed |
| `ROADMAP.md`: Versioning contradiction resolved (v1.0 -> v3.0) | Fixed |
| `VERSION.md`: Phase, test count, rollback updated | Fixed |
| `RELEASE_NOTES.md`: Historical banner added | Fixed |
| `ferro-graphql`: 19 unit tests added (was zero) | Fixed |
| `ferro-server-wopi`: Unused import removed | Fixed |

### 1.3 Remaining Issues (Documented)

| Issue | Severity | Owner |
|-------|----------|-------|
| `docs/api.md`: 40+ undocumented endpoints | High | Sprint AW |
| `docs/configuration.md`: 12+ missing CLI flags | High | Sprint AW |
| `CONTRIBUTING.md`: Coverage targets aspirational, not met | Medium | Sprint AW |
| `SECURITY.md`: ADR references point to non-existent docs | Medium | Sprint AV |
| `ferro-benchmarks` crate: benchmarks only, no tests | Medium | Sprint AW |
| `docs/benchmarks.md`: Placeholder performance numbers | Medium | Sprint AV |
| Cargo.lock: 42 duplicate crate warnings (SemVer incompat) | Low | Monitor |
| `CONTRIBUTING.md`: Node.js requirement misleading for web UI | Low | Sprint AW |
| WOPI: `urlsrc=""` empty in discovery XML when WOPI URL unset | Low | Sprint AU |

---

## 2. Architecture Quality Summary

### 2.1 Strengths

- **Zero production panics**: All `unwrap()` confined to `#[cfg(test)]` modules
- **Zero stubs**: No `todo!()`, `unimplemented!()`, or empty function bodies
- **Clean dependency graph**: 20 crates with clear responsibility boundaries
- **Comprehensive specs**: Yellow Papers, Blue Papers, Lean proofs, interface contracts
- **Multi-backend storage**: Local, S3, GCS, Azure via `object_store`
- **Standards-compliant WebDAV**: RFC 4918 with CalDAV/CardDAV extensions
- **Cedar authorization**: Fine-grained policy engine
- **OIDC + HTTP Basic**: Dual auth modes
- **WASM extensibility**: Worker sandbox for plugins
- **ActivityPub federation**: HTTP signatures, actor model
- **Pre-commit enforcement**: Format, lint, test, security audit

### 2.2 Technical Debt Register

| ID | Description | Severity | Fix Sprint |
|----|-------------|----------|------------|
| TD-001 | ~500 `.unwrap()` in test code (test hygiene, not production risk) | Low | AW |
| TD-002 | `ferro-graphql` had zero tests (now 19, target 40+) | Medium | AW |
| TD-003 | `rsa` crate via sqlx-mysql/age transitive dep (removed from default builds) | Low | Monitor |
| TD-004 | Tauri/wry GTK3 chain (18 advisories, desktop-only) | Low | AZ |
| TD-005 | Sqlite performance at scale (>100 concurrent users) | Medium | BD |
| TD-006 | No fuzzing infrastructure | Medium | AW |
| TD-007 | Desktop crate has no CI build | Medium | AZ |
| TD-008 | Leptos 0.6 (0.7+ available with breaking changes) | Medium | BA |
| TD-009 | No property-based testing (proptest/quickcheck) | Medium | AW |
| TD-010 | Server restart loses in-memory state (DashMap) | Medium | BD |

---

## 3. Production Roadmap: v2.5.1 to v3.0

### Phase 1: Production Hardening (Sprint AU) — 3 weeks

**Goal:** Deployable for real users with real data.

| ID | Task | Priority | Dependencies |
|----|------|----------|--------------|
| AU-01 | Enforce password change on first login | P0 | None |
| AU-02 | Rate limit login attempts (5/min per IP) | P0 | None |
| AU-03 | Account lockout after 10 consecutive failures (15 min) | P1 | AU-02 |
| AU-04 | Session token rotation on sensitive ops | P1 | None |
| AU-05 | OIDC token refresh flow | P1 | None |
| AU-06 | Database backup/restore CLI command | P0 | None |
| AU-07 | File integrity verification (SHA-256 audit) | P0 | None |
| AU-08 | Graceful shutdown of all subsystems | P1 | None |
| AU-09 | Trash auto-purge (configurable TTL) | P1 | None |
| AU-10 | Read-only maintenance mode | P2 | None |
| AU-11 | Config validation on startup (reject invalid) | P0 | None |
| AU-12 | `docs/configuration.md`: document all 45+ CLI flags | P1 | None |
| AU-13 | WOPI: handle empty `urlsrc` gracefully | P2 | None |
| AU-14 | LDAP group mapping to Cedar roles | P2 | ldap feature |

### Phase 2: Reliability and Observability (Sprint AV) — 2 weeks

**Goal:** Production-grade observability and error handling.

| ID | Task | Priority | Dependencies |
|----|------|----------|--------------|
| AV-01 | Structured JSON logging (tracing-subscriber JSON layer) | P0 | None |
| AV-02 | Prometheus metrics: request duration, error rate, storage usage | P0 | None |
| AV-03 | Health check subsystem depth (DB, cache, storage reachability) | P0 | None |
| AV-04 | `/healthz` liveness check (fast, no DB dependency) | P0 | None |
| AV-05 | `/readyz` readiness check (all subsystems ready) | P0 | None |
| AV-06 | Error taxonomy: every error has unique code + message | P1 | None |
| AV-07 | Request tracing with `X-Request-ID` propagation | P1 | None |
| AV-08 | Audit log persistence (currently in-memory DashMap, lost on restart) | P0 | None |
| AV-09 | `docs/benchmarks.md`: replace placeholder values with real measurements | P1 | None |
| AV-10 | `SECURITY.md`: create referenced ADR documents (ADR-001 through ADR-005) | P2 | None |

### Phase 3: Test Coverage Expansion (Sprint AW) — 3 weeks

**Goal:** 80% overall branch coverage, 95% critical paths.

| ID | Task | Priority | Dependencies |
|----|------|----------|--------------|
| AW-01 | `ferro-graphql`: expand to 40+ tests (edge cases, error paths) | P0 | None |
| AW-02 | Property-based testing: proptest for parser, serializer, crypto | P0 | None |
| AW-03 | Fuzzing infrastructure: cargo-fuzz for WebDAV XML, HTTP parsing | P0 | None |
| AW-04 | `ferro-benchmarks`: add correctness tests alongside benchmarks | P1 | None |
| AW-05 | Integration tests for undocumented endpoints (40+ remaining) | P1 | AU-12 |
| AW-06 | E2E test suite: Playwright for all web UI flows | P1 | None |
| AW-07 | Test coverage measurement: add tarpaulin to CI | P0 | None |
| AW-08 | `docs/api.md`: document all 90+ endpoints | P0 | AU-12 |
| AW-09 | `docs/configuration.md`: document all 45+ CLI flags | P0 | AU-12 |
| AW-10 | `CONTRIBUTING.md`: fix Node.js claim, update coverage targets | P2 | None |
| AW-11 | Remove `.unwrap()` from test code where practical | P2 | None |

### Phase 4: Security Certification (Sprint AX) — 2 weeks

**Goal:** Zero critical/high CVEs, security review completed.

| ID | Task | Priority | Dependencies |
|----|------|----------|--------------|
| AX-01 | CSRF token for all state-changing endpoints | P0 | None |
| AX-02 | Content-Security-Policy header hardening | P0 | None |
| AX-03 | Input validation audit: path traversal, SQL injection, XSS, XXE | P0 | None |
| AX-04 | Dependency security audit: eliminate/replace all high-severity CVEs | P0 | None |
| AX-05 | Rate limiting: per-route configuration (not just per-IP) | P1 | None |
| AX-06 | Security.txt: `/.well-known/security.txt` | P1 | None |
| AX-07 | Internal security review: OWASP ASVS Level 2 checklist | P1 | None |
| AX-08 | CSP: remove `'unsafe-inline'` from default policy | P2 | None |
| AX-09 | CORS: strict origin validation (not wildcard default) | P2 | None |
| AX-10 | Dependency: monitor and migrate from `rsa` crate when upstream fix lands | P2 | None |

### Phase 5: Production Release v3.0 (Sprint AY) — 1 week

**Goal:** Release v3.0.0 with confidence.

| ID | Task | Priority | Dependencies |
|----|------|----------|--------------|
| AY-01 | All P0 items from Phases 1-4 resolved | P0 | AU-AX |
| AY-02 | 95%+ branch coverage on critical paths | P0 | AW-07 |
| AY-03 | 80%+ overall branch coverage | P0 | AW-07 |
| AY-04 | Zero critical/high CVEs in dependency tree | P0 | AX-04 |
| AY-05 | 24-hour soak test: zero panics, zero data loss | P0 | None |
| AY-06 | Multi-arch Docker image: linux/amd64, linux/arm64 | P0 | CI |
| AY-07 | Helm chart for Kubernetes deployment | P0 | None |
| AY-08 | SPDX SBOM automated on release | P0 | CI |
| AY-09 | Upgrade guide: v2.x to v3.0 | P0 | None |
| AY-10 | All 90+ API endpoints documented | P0 | AW-08 |
| AY-11 | Release notes with changelog, known issues, migration guide | P0 | None |

### Total estimated time to v3.0: 11 weeks (full-time)

---

## 4. Post-v3.0 Growth (v3.1 to v3.5)

### 4.1 Desktop Client (v3.1) — Sprint AZ, 4 weeks

- File sync daemon with background sync and conflict resolution
- Selective sync (per-folder toggle)
- System tray indicator with sync status
- macOS universal binary (Intel + Apple Silicon)
- Windows MSI installer with shell integration
- Desktop crate CI build (currently missing)
- Migrate from Tauri GTK3 to GTK4 when upstream lands

### 4.2 Mobile (v3.2) — Sprint BA, 4 weeks

- iOS File Provider extension (Files app integration)
- Android Storage Access Framework provider
- Offline mode with local cache and conflict resolution
- Push notifications (share received, quota warning)
- Leptos 0.7 migration (breaking changes expected)

### 4.3 Collaboration (v3.3) — Sprint BB, 3 weeks

- Real-time co-editing via CRDT + WebRTC data channels
- Comments and annotations (per-file threads)
- File locking visual indicator in web UI
- Activity notifications (email/webhook on share, comment, mention)

### 4.4 Admin and Compliance (v3.4) — Sprint BC, 2 weeks

- Admin dashboard (user management, storage stats, audit log viewer)
- Two-factor authentication (TOTP)
- SAML 2.0 service provider
- Data retention policies (auto-deletion past retention period)
- GDPR export compliance (all user data in machine-readable format)

### 4.5 Performance (v3.5) — Sprint BD, 2 weeks

- True streaming uploads (no full buffering before write)
- Ranged GET with caching (HTTP Range header support)
- Persistent thumbnail cache with LRU eviction
- Search index sharding for >1M files
- Configurable connection pooling for cloud backends
- PostgreSQL support for >100 concurrent users
- Persistent audit log (replace DashMap with SQLite/PG)

---

## 5. Platform Evolution (v4.0+)

### 5.1 Plugin System (v4.0)

- Stable WASM plugin API with versioned ABI
- Plugin marketplace (community registry)
- Capability-based security model for WASM sandbox
- Hot-reload plugins without server restart

### 5.2 Multi-Tenant (v4.1)

- Organization support with per-org storage, quotas, policies
- Resource isolation (rate limits, connection pools, storage backends)
- Cross-organization controlled sharing

### 5.3 Distributed Storage (v4.2)

- Erasure coding (Reed-Solomon) for data durability across nodes
- Geo-replication (async replication between data centers)
- Raft-based metadata consensus for distributed deployments

### 5.4 AI Integration (v4.3)

- Semantic search (vector embeddings for natural language file search)
- ML-based content classification and auto-tagging
- OCR and text extraction from images/PDFs for full-text search
- Perceptual hashing for near-duplicate detection

---

## 6. Continuous Improvement Protocol

### 6.1 Quality Gates (Every Commit)

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`
4. `cargo deny check advisories bans licenses sources`

### 6.2 Weekly Review

- Dependency updates (`cargo update` + vulnerability scan)
- stale advisory ignore list clean-up
- Test coverage trend monitoring
- Performance regression baseline comparison

### 6.3 Monthly Review

- Documentation accuracy audit
- E2E test suite health
- Security advisory monitoring
- Standard compliance review (ISO, NIST, IEC)

### 6.4 Pre-Release Review

- Complete audit per this document
- Independent security review
- Soak test (24h minimum)
- Multi-arch build verification
- SBOM generation and verification

---

## 7. Risk Register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Tauri GTK4 migration delayed | Medium | Low | Server/core unaffected; GTK3 acceptable for desktop |
| `rsa` crate cannot be eliminated | Low | Medium | MySQL/age code paths isolated; document risk |
| SQLite performance at scale | Medium | High | Recommend PostgreSQL for >100 concurrent |
| Leptos 0.7 breaking changes | Medium | Medium | Pin version; plan migration window |
| WASM plugin ABI instability | High | Low | Versioned ABI from start; feature flag |
| Dependency supply chain attack | Low | Critical | `cargo-deny` pre-commit; SBOM on release; pin SHA-256 |
| Pre-commit hook timeout on slow CI | Medium | Medium | See Section 8 notes |

---

## 8. Pre-Commit Hook Considerations

Current hook runs: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`, `cargo deny check`.

**Performance:** Full workspace test takes 5-8 minutes on modern hardware, potentially longer on CI or constrained environments. Recommendations:

- Keep full `--workspace` test: transitive breakage detection is critical
- Add `RUST_TEST_THREADS` configuration for parallelism tuning
- Document skip mechanism: `git commit --no-verify` for emergencies (with explicit warning)
- Future: Consider incremental test execution (changed crates + dependents) to reduce commit latency

The current hook correctly enforces deterministic, provable correctness at commit time. The cost is commit latency. The trade-off is justified for safety-critical software.

---

## 9. Success Metrics for v3.0

| Metric | Target | Current |
|--------|--------|---------|
| Test coverage (critical paths) | >95% branch | ~85% (estimated) |
| Test coverage (overall) | >80% branch | ~70% (estimated) |
| Zero-downtime deployment | Working | Not tested |
| Clippy warnings | 0 | 0 |
| Critical/high CVEs | 0 | 0 |
| API endpoints documented | 100% | ~55% |
| Multi-arch release | linux/amd64 + arm64 | Not yet |
| Helm chart | Working | Not yet |
| Soak test (24h) | 0 panics, 0 data loss | Not yet |
| Upgrade guide | Documented | Not yet |

---

## 10. Conclusion

Ferro v2.5.1 has strong fundamentals: correct WebDAV, multi-backend storage, Cedar authorization, ActivityPub federation, and a rigorous test suite (809 tests, 0 failures). The codebase is free of stubs, production panics, and TODO markers. Pre-commit hooks enforce deterministic quality.

The path to v3.0 requires 11 weeks of production hardening, observability improvements, test coverage expansion, and security certification. Post-v3.0 growth targets desktop sync, mobile clients, real-time collaboration, and enterprise admin features. Platform evolution (v4.0+) will introduce a plugin system, multi-tenancy, distributed storage, and AI-powered features.

The most critical immediate tasks (Sprint AU) are database backup/restore, file integrity verification, config validation on startup, and login rate limiting — all of which block production deployment for real users.
