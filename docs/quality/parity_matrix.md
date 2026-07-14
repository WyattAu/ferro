# Code Quality Parity Matrix

**Date:** 2026-07-05
**Ferro State:** 172K LOC, 60 crates, Rust 2024, MSRV 1.92

---

## Comparison Targets

| Tier | Companies | Typical Standards |
|------|-----------|-------------------|
| **HFT** | Jane Street, Citadel Securities, Jump Trading, Two Sigma | Nanosecond latency, formal verification, zero panics, custom allocators, nightly compiler |
| **Tier-1 FAANG** | Google, Meta, Apple | Massive-scale CI, 2+ reviewer policy, static analysis at scale, chaos engineering, SLO/SLA |
| **Tier-2 Big Tech** | Amazon, Netflix, Stripe, Cloudflare | Strong CI/CD, security-first, incident automation, blameless post-mortems |
| **Ferro (Current)** | — | Our current state |

---

## Parity Matrix

### 1. Static Analysis & Linting

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| Compiler warnings as errors | Yes | Yes | Yes | **Yes** | PARITY |
| Clippy `-D warnings` | Yes + pedantic | Yes | Yes | **Yes** (perf/complexity/suspicious) | SMALL — pedantic not enforced workspace-wide |
| Clippy pedantic | Yes (all lints) | Yes | Partial | **No** (ferro-crdt fails) | MEDIUM |
| `unsafe` block auditing | Yes (every block reviewed) | Yes (OSS-Fuzz + review) | Yes | **Partial** (26 blocks, all SAFETY-commented) | SMALL |
| Custom lint rules (semantic) | Yes (custom rustc plugins) | Yes (via lint passes) | Partial | **No** | LARGE |
| MISRA-C / CERT Rust | Some adopt CERT | Partial | No | **No** | LARGE |
| Undefined behavior detection (Miri) | Yes (CI-gated) | Yes | Partial | **No** | LARGE |
| `cargo-deny` (licenses/bans) | Yes | Yes | Yes | **Installed, not CI-gated** | MEDIUM |
| `cargo-machete` (unused deps) | Yes | Yes | Yes | **Not installed** | MEDIUM |
| `cargo-semver-checks` | Yes | Yes | Yes | **Not installed** | MEDIUM |

### 2. Testing

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| Unit test count | 10K+ per repo | 100K+ | 10K+ | **~700** | LARGE |
| Line coverage | >95% critical | >80% overall | >80% | **Unknown** (no measurement) | LARGE |
| Branch coverage | >95% | >80% | >70% | **Unknown** | LARGE |
| MC/DC coverage | Yes (safety-critical) | Partial | No | **No** | LARGE |
| Coverage enforcement in CI | Yes (threshold gates) | Yes | Yes | **No** | LARGE |
| Mutation testing | Yes (custom) | Yes (Sapling) | Partial | **cargo-mutants installed, not CI-gated** | LARGE |
| Property-based testing | Yes (QuickCheck/custom) | Yes (Proptest) | Partial | **4 fuzz harnesses** (44.9M iterations) | MEDIUM |
| Fuzz testing | Yes (AFL, libFuzzer) | Yes (OSS-Fuzz) | Yes | **4 harnesses, no CI integration** | MEDIUM |
| Integration tests | Yes (extensive) | Yes | Yes | **~250** (webdav-litmus, CalDAV) | MEDIUM |
| E2E tests | Yes | Yes (Playwright/Selenium) | Yes | **No** | LARGE |
| Load/stress testing | Yes (continuous) | Yes | Yes | **1 soak test** (24h script) | MEDIUM |
| Chaos engineering | Yes (GameDay) | Yes (Chaos Monkey) | Yes (Chaos Kong) | **No** | LARGE |
| Regression detection | Yes (automated) | Yes | Yes | **Baseline metrics defined, not CI-gated** | LARGE |

### 3. Security

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| SAST (static) | Yes (Coverity/CodeQL) | Yes (Infer/CodeQL) | Yes (CodeQL) | **Clippy + manual audit** | LARGE |
| DAST (dynamic) | Yes | Yes (ZAP/Burp) | Yes | **nuclei installed, not CI-gated** | LARGE |
| Dependency scanning | Yes (continuous) | Yes (Dependabot/Snyk) | Yes | **cargo-audit (manual)** | LARGE |
| Secret detection | Yes (truffleHog/gitleaks) | Yes | Yes | **Manual audit (0 found)** | MEDIUM |
| Penetration testing | Yes (red team) | Yes (external) | Yes | **Scope doc written, not executed** | LARGE |
| Supply chain (SBOM) | Yes (SLSA Level 3+) | Yes (SLSA) | Yes | **No SBOM** | LARGE |
| CSRF protection | Yes | Yes | Yes | **Yes (just wired in)** | PARITY |
| Path traversal defense | Yes | Yes | Yes | **Yes (comprehensive)** | PARITY |
| XXE prevention | Yes | Yes | Yes | **Yes (entity expansion disabled)** | PARITY |
| SQL injection prevention | Yes | Yes | Yes | **Yes (parameterized queries)** | PARITY |
| TLS enforcement | Yes (no exceptions) | Yes | Yes | **Partial** (migration tool skips TLS) | SMALL |
| CVE response SLA | <24h critical | <72h | <1 week | **No SLA** | LARGE |

### 4. Performance

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| Latency budgets | Nanosecond | Millisecond | Millisecond | **No formal budgets** | LARGE |
| Benchmarking in CI | Yes (continuous) | Yes | Yes | **Criterion available, not CI-gated** | LARGE |
| Profiling | Yes (perf/VTune) | Yes | Yes | **No automated profiling** | LARGE |
| Memory leak detection | Yes (Valgrind/ASan) | Yes (ASan/MSan) | Yes | **No CI integration** | LARGE |
| Custom allocator | Yes (jemalloc/tcmalloc) | Yes | Yes | **System allocator** | LARGE |
| Lock-free data structures | Yes (critical paths) | Partial | No | **Some** (DashMap, atomics) | SMALL |
| Async runtime tuning | Yes (tokio runtime config) | Yes | Yes | **Basic tokio config** | MEDIUM |
| WCET analysis | Yes (real-time) | No | No | **No** | LARGE (if real-time needed) |

### 5. Concurrency & Thread Safety

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| `Send`/`Sync` audit | Yes | Yes | Yes | **Removed redundant unsafe impls** | PARITY |
| Mutex across .await detection | Yes (Clippy) | Yes | Yes | **Fixed (spawn_blocking)** | PARITY |
| Deadlock detection | Yes (ThreadSanitizer) | Yes (TSan) | Yes | **No CI integration** | LARGE |
| Race condition detection | Yes (TSan) | Yes | Yes | **No CI integration** | LARGE |
| Lock hierarchy enforcement | Yes (custom tooling) | Yes | Partial | **No** | LARGE |
| Atomic ordering audit | Yes | Partial | No | **No** | MEDIUM |

### 6. Error Handling

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| No unwrap in production | Yes (Clippy restriction) | Yes | Yes | **Yes** (2 Mutex poison only) | PARITY |
| No panic in production | Yes (catch_unwind) | Partial | Partial | **Yes** (0 panics in prod) | PARITY |
| Error context propagation | Yes (anyhow/custom) | Yes | Yes | **Yes (thiserror + anyhow)** | PARITY |
| Discarded Results audit | Yes (Clippy `#[must_use]`) | Yes | Yes | **Fixed critical sites** | SMALL — 55 total still |
| Structured logging | Yes | Yes | Yes | **Yes (tracing)** | PARITY |
| Error budget (SLO) | Yes | Yes | Yes | **No** | LARGE |

### 7. Code Architecture

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| Module size <500 LOC | Yes | Yes | Yes | **No** (1,822 LOC state.rs, 1,767 LOC webdav.rs) | LARGE |
| Function size <50 LOC | Yes | Yes | Partial | **No** (1,232 LOC main()) | LARGE |
| Cyclomatic complexity <10 | Yes | Yes | Yes | **No** (depth 12 in worker_runner) | LARGE |
| Dependency direction (DIP) | Yes | Yes | Yes | **Partial** (trait-based state, some concrete deps) | MEDIUM |
| API surface minimization | Yes | Yes | Yes | **Partial** (too many `pub` items) | MEDIUM |
| `#[non_exhaustive]` enums | Yes | Yes | Partial | **Partial** (many missing) | SMALL |
| `pub(crate)` enforcement | Yes | Yes | Yes | **21 in 60 crates** | MEDIUM |

### 8. CI/CD & Process

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| CI on every commit | Yes | Yes | Yes | **Partial** (pre-commit hooks) | MEDIUM |
| 2+ code reviewers | Yes | Yes (mandatory) | Yes | **No** (solo dev) | LARGE (structural) |
| Automated formatting | Yes (rustfmt) | Yes | Yes | **Yes** (just added .rustfmt.toml) | PARITY |
| Build reproducibility | Yes (hermetic) | Yes | Yes | **Partial** (Nix flake exists) | SMALL |
| Rollback capability | Yes (instant) | Yes | Yes | **Yes** (git tags) | PARITY |
| Canary deployments | Yes | Yes | Yes | **No** | LARGE |
| Blue-green deployments | Yes | Yes | Yes | **No** | LARGE |
| Incident response automation | Yes | Yes | Yes | **No** | LARGE |
| Blameless post-mortems | Yes | Yes | Yes | **Partial** (Level 4+ only) | SMALL |
| Technical debt tracking | Yes (automated) | Yes | Yes | **Manual** (audit report) | MEDIUM |

### 9. Documentation

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| API documentation | Yes (rustdoc) | Yes | Yes | **Partial** (missing docs on many items) | MEDIUM |
| Architecture docs | Yes | Yes | Yes | **Yes** (Blue/Yellow papers) | PARITY |
| ADR process | Yes | Yes | Yes | **Yes** (.adrs/) | PARITY |
| Runbooks | Yes | Yes | Yes | **Partial** | MEDIUM |
| On-call documentation | Yes | Yes | Yes | **No** | LARGE (if deployed) |
| Changelog | Yes | Yes | Yes | **Yes** (CHANGELOG.md) | PARITY |

### 10. Compliance & Standards

| Capability | HFT | Google/Meta | Amazon/Netflix | Ferro | Gap |
|------------|-----|-------------|----------------|-------|-----|
| License compliance | Yes (automated) | Yes | Yes | **cargo-deny installed, not CI-gated** | MEDIUM |
| SBOM generation | Yes (SPDX/CycloneDX) | Yes | Yes | **No** | LARGE |
| GDPR compliance | Yes | Yes | Yes | **Partial** (GDPR API exists) | SMALL |
| SOC 2 | Yes | Yes | Yes | **No** | LARGE (enterprise requirement) |
| ISO 27001 | Some | Yes | Yes | **No** | LARGE (enterprise requirement) |

---

## Overall Parity Score

| Domain | HFT | Google/Meta | Amazon/Netflix | Ferro |
|--------|-----|-------------|----------------|-------|
| Static Analysis | 95% | 90% | 75% | **65%** |
| Testing | 95% | 90% | 80% | **40%** |
| Security | 90% | 95% | 85% | **55%** |
| Performance | 98% | 80% | 70% | **35%** |
| Concurrency | 95% | 90% | 80% | **60%** |
| Error Handling | 95% | 85% | 75% | **75%** |
| Architecture | 85% | 85% | 75% | **45%** |
| CI/CD | 95% | 95% | 90% | **40%** |
| Documentation | 80% | 85% | 75% | **60%** |
| Compliance | 90% | 90% | 85% | **30%** |
| **Weighted Average** | **92%** | **89%** | **79%** | **51%** |

---

## Key Gaps by Priority

### Tier 1: Critical (blocks production deployment)
1. No code coverage measurement or enforcement
2. No CI pipeline (pre-commit hooks only)
3. No SAST/DAST in automated pipeline
4. No SBOM generation
5. `main()` at 1,232 lines (unmaintainable)
6. `state.rs` at 1,822 lines (God module)

### Tier 2: High (required for enterprise adoption)
7. No mutation testing CI gate
8. No benchmark regression detection
9. No deadlock/race detection in CI
10. No chaos engineering
11. No canary/blue-green deployment
12. No incident response automation
13. No SOC 2 / ISO 27001 compliance path

### Tier 3: Medium (quality of life)
14. Clippy pedantic not enforced workspace-wide
15. Missing `#[non_exhaustive]` on public enums
16. Too many `pub` items (should be `pub(crate)`)
17. Missing rustdoc on public APIs
18. ~55 remaining discarded Results
19. No automated dependency update (Dependabot/Renovate)
