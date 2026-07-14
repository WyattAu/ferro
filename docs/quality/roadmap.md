# Code Quality Parity Roadmap

**Goal:** Close the gap from 51% to 80%+ parity with Tier-2 Big Tech standards within 12 weeks.
**Current:** 172K LOC, 60 crates, Rust 2024, 51% weighted parity score.

---

## Phase 1: Foundation (Weeks 1-2) — CI Pipeline + Measurement

**Target:** Establish automated quality gates so every commit is measured.

### Week 1: CI Pipeline Setup

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| GitHub Actions CI: `cargo clippy -D warnings` + `cargo test` on every PR | 2h | Every commit linted/tested | +5% |
| Add `cargo-deny` to CI (license compliance, advisory audit) | 1h | Automated license/vuln check | +3% |
| Add `cargo-machete` to CI (unused dependency detection) | 30m | Dead dependency cleanup | +1% |
| Add `cargo fmt --check` to CI with `.rustfmt.toml` style_edition 2024 | 30m | Formatting enforcement | Already parity |
| Add MSRV check to CI (`cargo +1.92 check`) | 1h | MSRV compliance | +1% |

### Week 2: Coverage Measurement

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Integrate `cargo-llvm-cov` into CI | 4h | Coverage measurement |
| Set coverage threshold: 80% line, 70% branch (initial) | 1h | Enforcement gate |
| Generate coverage reports (HTML + lcov for CI badges) | 2h | Visibility |
| Run coverage on critical crates first (server, webdav-core, security) | 2h | Prioritized coverage |

**Phase 1 Exit Criteria:**
- [ ] CI runs on every PR with clippy, test, fmt, deny, machete, coverage
- [ ] Coverage reported but not blocking (informational)
- [ ] All existing tests pass in CI

---

## Phase 2: Hardening (Weeks 3-5) — Security + Correctness

**Target:** Automated security scanning and formal correctness tools.

### Week 3: Security Automation

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Add `cargo-audit` to CI with failure on HIGH/CRITICAL | 1h | Dependency vulnerability gate |
| Add Semgrep CI scan (Rust security rules) | 3h | SAST automation |
| Integrate nuclei into CI (DAST against test server) | 4h | Dynamic security testing |
| Generate SBOM (SPDX format) on release | 3h | Supply chain transparency |
| Add gitleaks to CI (secret detection) | 1h | Prevent secret commits |

### Week 4: Correctness Tools

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Add Miri to CI for unsafe-heavy crates (client/ffi, fuse) | 3h | UB detection |
| Add `cargo-semver-checks` to CI for library crates | 2h | Breaking change detection |
| Integrate `cargo-mutants` into CI (weekly, not per-PR) | 4h | Test quality measurement |
| Add ThreadSanitizer test run (weekly) | 4h | Data race detection |

### Week 5: Fuzzing Expansion

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Add 6 new fuzz targets (XML parser, CalDAV, CardDAV, WebSocket, API auth, config parser) | 8h | Input validation coverage |
| Integrate cargo-fuzz into CI (nightly, 10min timeout) | 2h | Continuous fuzzing |
| Add corpus minimization and coverage tracking | 2h | Fuzzing efficiency |

**Phase 2 Exit Criteria:**
- [ ] SAST/DAST automated on every PR
- [ ] Miri runs on unsafe crates (weekly)
- [ ] 10 fuzz harnesses with CI integration
- [ ] SBOM generated on release
- [ ] Semver-checks prevent accidental API breaks

---

## Phase 3: Architecture (Weeks 6-8) — Decompose God Objects

**Target:** Break the 3 worst offenders into testable, maintainable modules.

### Week 6: Decompose `main()` (1,232 lines)

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Extract `cli.rs` — CLI argument parsing (~150 lines) | 2h | Testable CLI |
| Extract `config.rs` already exists — verify clean separation | 1h | Config isolation |
| Extract `startup.rs` — state construction, daemon spawning (~300 lines) | 4h | Startup testability |
| Extract `tls.rs` — TLS certificate loading (~100 lines) | 2h | TLS testability |
| Reduce `main()` to <100 lines (delegation only) | 2h | Entry point clarity |

### Week 7: Decompose `state.rs` (1,822 lines)

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Extract `state/database.rs` — DB handle, migrations, with_db (~400 lines) | 3h | DB state isolation |
| Extract `state/storage.rs` — storage backend config (~200 lines) | 2h | Storage config |
| Extract `state/security.rs` — auth, rate limiting, CSRF (~300 lines) | 3h | Security config |
| Extract `state/collaboration.rs` — collab, chat, comments (~200 lines) | 2h | Collaboration config |
| Reduce `state.rs` to <500 lines (struct definition + constructors) | 2h | Module clarity |

### Week 8: Decompose `webdav.rs` (1,767 lines)

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Extract `webdav/put.rs` — handle_put + handle_put_streaming (~600 lines) | 4h | PUT handler isolation |
| Extract `webdav/proppatch.rs` — PROPPATCH handler (~300 lines) | 2h | PROPPATCH isolation |
| Extract `webdav/props.rs` — property parsing/formatting (~400 lines) | 3h | Property isolation |
| Reduce `webdav.rs` to <500 lines (dispatch + shared utils) | 2h | Core handler clarity |

**Phase 3 Exit Criteria:**
- [ ] `main()` < 100 lines
- [ ] `state.rs` < 500 lines
- [ ] `webdav.rs` < 500 lines
- [ ] All extracted modules have unit tests
- [ ] No function > 100 lines in extracted modules

---

## Phase 4: Test Quality (Weeks 9-10) — Coverage + Mutation

**Target:** Enforce coverage thresholds and validate test quality with mutation testing.

### Week 9: Coverage Enforcement

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Raise coverage threshold to 85% line, 75% branch (enforce in CI) | 1h | Quality gate |
| Add coverage diff reporting on PRs (delta) | 2h | PR-level visibility |
| Write tests for uncovered critical paths (identified by coverage) | 8h | Coverage improvement |
| Add property-based tests (proptest) for path normalization, XML escaping, config parsing | 6h | Input validation |

### Week 10: Mutation Testing Gate

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Run cargo-mutants on critical crates, establish baseline score | 4h | Test quality baseline |
| Set mutation score threshold: 70% (initial) | 1h | Enforcement gate |
| Fix surviving mutants (weak assertions) | 8h | Test strengthening |
| Add integration test for full server lifecycle (startup, request, shutdown) | 4h | E2E coverage |

**Phase 4 Exit Criteria:**
- [ ] Coverage: 85% line, 75% branch (enforced)
- [ ] Mutation score: 70%+ (measured, enforced weekly)
- [ ] Property-based tests for 5 critical input parsers
- [ ] Integration test for server lifecycle

---

## Phase 5: Performance & Operations (Weeks 11-12) — Observability + Performance

**Target:** Automated performance regression detection and operational readiness.

### Week 11: Performance Automation

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Add Criterion benchmarks for critical paths (PUT, GET, PROPFIND, auth) | 6h | Performance baseline |
| Add benchmark regression detection to CI (fail on >5% regression) | 3h | Regression prevention |
| Add `cargo-bloat` analysis (binary size tracking) | 1h | Size monitoring |
| Profile release binary with `perf` (identify hot paths) | 4h | Optimization roadmap |
| Add tokio-console integration for async runtime monitoring | 3h | Async debugging |

### Week 12: Operational Readiness

| Task | Effort | Impact | Parity Lift |
|------|--------|--------|-------------|
| Add structured metrics endpoint (Prometheus) — verify existing works | 2h | Observability |
| Add distributed tracing (OpenTelemetry) | 6h | Request tracing |
| Write runbooks for top 5 failure modes | 4h | Incident response |
| Add health check deeper probes (DB, storage, Redis) | 3h | Health granularity |
| Add graceful degradation patterns (circuit breakers for external deps) | 6h | Resilience |

**Phase 5 Exit Criteria:**
- [ ] Benchmarks in CI with regression detection
- [ ] Prometheus metrics endpoint functional
- [ ] OpenTelemetry tracing initialized
- [ ] 5 runbooks written
- [ ] Circuit breakers on external dependencies

---

## Summary: Parity Trajectory

| Phase | Weeks | Focus | Parity After |
|-------|-------|-------|-------------|
| Current | — | — | **51%** |
| Phase 1 | 1-2 | CI + Measurement | **60%** |
| Phase 2 | 3-5 | Security + Correctness | **68%** |
| Phase 3 | 6-8 | Architecture Decomposition | **74%** |
| Phase 4 | 9-10 | Test Quality | **79%** |
| Phase 5 | 11-12 | Performance + Operations | **83%** |

### What We Will NOT Close (Structural Gaps)

These require team size or organizational changes that a solo/early-stage project cannot replicate:

| Gap | Reason | Mitigation |
|-----|--------|------------|
| 2+ code reviewers | Solo developer | External reviewers for security-critical changes |
| Chaos engineering | Requires production traffic | Load testing in staging |
| SOC 2 / ISO 27001 | Requires org-level certification | Document controls for future audit |
| Custom allocators | Premature optimization | Profile first, optimize later |
| Nightly compiler features | Stability risk | Use stable, adopt nightly features as they stabilize |
| Canary/blue-green deployment | Requires orchestrator (K8s) | Document deployment strategy for future |
| Incident response automation | Requires on-call rotation | Document runbooks for future |

### Tool Installation Checklist

```
cargo install cargo-machete       # Unused dependency detection
cargo install cargo-semver-checks # API breaking change detection
cargo install cargo-bloat         # Binary size analysis
cargo install cargo-tarpaulin     # Coverage (alternative to llvm-cov)
cargo install cargo-nextest       # Faster test runner
```

### CI Pipeline Architecture

```
PR Created
  |
  +---> cargo fmt --check        (formatting)
  +---> cargo clippy -D warnings (linting)
  +---> cargo test --lib          (unit tests)
  +---> cargo test --test         (integration tests)
  +---> cargo-deny check          (licenses + advisories)
  +---> cargo-machete             (unused deps)
  +---> cargo-llvm-cov            (coverage measurement)
  +---> cargo-semver-checks       (API compatibility)
  +---> gitleaks detect           (secret scanning)
  +---> semgrep scan              (SAST)
  |
  v
Merge to main
  |
  +---> cargo-audit               (dependency vulnerabilities)
  +---> nuclei --scan             (DAST against staging)
  +---> cargo-mutants --test      (mutation testing, weekly)
  +---> miri test                 (UB detection, weekly)
  +---> Criterion benchmarks      (performance regression)
  +---> SBOM generation           (supply chain)
  +---> Release build + deploy    (if tagged)
```
