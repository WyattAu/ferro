# ALC_TAT: Tools, Techniques, and Procedures

## Assurance Family Requirement

The developer shall describe the development tools, techniques, and procedures used to develop the TOE, including evidence that appropriate tools and techniques are used.

**EAL Level:** EAL 3+ (ALC_TAT.2)

## Evidence Artifacts

### 1. Rust Toolchain

**File:** `rust-toolchain.toml`

| Attribute | Value |
|-----------|-------|
| Channel | `1.95.0` |
| Components | `rustfmt`, `clippy` |
| MSRV | 1.92 (verified in CI) |

**Toolchain Rationale:**
- Pinned version ensures reproducible builds
- `rustfmt` enforces consistent code style
- `clippy` provides static analysis and linting
- MSRV check ensures backward compatibility

### 2. Formal Verification (Lean4)

**Path:** `formal/`

**19 Lean4 proof files** covering critical security properties:

| File | Property Verified |
|------|-------------------|
| `formal/Ferro/Basic.lean` | Basic type properties |
| `formal/Ferro/DataTypes.lean` | Data type correctness |
| `formal/Ferro/Authentication.lean` | Authentication correctness |
| `formal/Ferro/AuthTokenProperties.lean` | Auth token security properties |
| `formal/Ferro/PathSafety.lean` | Path handling safety |
| `formal/Ferro/PathValidation.lean` | Input validation correctness |
| `formal/Ferro/PathTraversal.lean` | Traversal attack prevention |
| `formal/Ferro/HashProperties.lean` | Hash function properties |
| `formal/Ferro/HashConsistency.lean` | Hash consistency guarantees |
| `formal/Ferro/Cache.lean` | Cache correctness |
| `formal/Ferro/CacheConsistency.lean` | Cache consistency properties |
| `formal/Ferro/CacheEviction.lean` | Eviction policy correctness |
| `formal/Ferro/CacheInvalidation.lean` | Invalidation correctness |
| `formal/Ferro/RateLimiter.lean` | Rate limiter correctness |
| `formal/Ferro/RateLimiterProperties.lean` | Rate limiter security properties |
| `formal/Ferro/RateLimiterRefined.lean` | Refined rate limiter model |
| `formal/Ferro/CircuitBreaker.lean` | Circuit breaker correctness |
| `formal/Ferro/CircuitBreakerRefined.lean` | Refined circuit breaker model |
| `formal/Ferro/CRDTProperties.lean` | CRDT convergence properties |

**CI Integration:** `.github/workflows/formal_verification.yml`

### 3. Fuzzing (15 Targets)

**Path:** `fuzz/`

| Target | Component | Purpose |
|--------|-----------|---------|
| `fuzz_escape_xml` | WebDAV handler | XML injection/escaping |
| `fuzz_xml` | XML parser | XML parsing robustness |
| `fuzz_json` | JSON parser | JSON parsing robustness |
| `fuzz_ical` | CalDAV | iCalendar parsing |
| `fuzz_vcard` | CardDAV | vCard parsing |
| `fuzz_calcardav` | CalDAV/CardDAV | Protocol parsing |
| `fuzz_proppatch` | WebDAV | PROPPATCH handling |
| `fuzz_lock_request` | WebDAV | LOCK request parsing |
| `fuzz_path_normalize` | Core | Path normalization |
| `fuzz_api_auth` | Auth | API authentication |
| `fuzz_config` | Server config | Configuration parsing |
| `fuzz_wasm_magic` | WASM host | WASM magic byte validation |
| `fuzz_crypto` | Crypto | Cryptographic operations |
| `fuzz_fips` | FIPS | FIPS compliance checks |
| `fuzz_circuit_breaker` | Resilience | Circuit breaker behavior |

**Fuzzing Statistics:** 2.6M+ iterations across all targets

**CI Integration:** `.github/workflows/sanitizers.yml`, regression testing in CI

### 4. Static Analysis

| Tool | Purpose | Enforcement |
|------|---------|-------------|
| `cargo clippy` | Rust lints | `-D warnings` in CI |
| `cargo fmt` | Code formatting | `--check` in CI |
| `cargo-deny` | License/advisory audit | Required in CI |
| `cargo audit` | Security advisories | Via cargo-deny |
| Trivy | Container scanning | `exit-code: 1` on CRITICAL/HIGH |
| `cargo-mutants` | Mutation testing | In CI pipeline |

### 5. Testing Tools

| Tool | Purpose | Integration |
|------|---------|-------------|
| `cargo test` | Unit/integration tests | CI pipeline |
| `proptest` | Property-based testing | In-test dependency |
| `cargo-fuzz` | Fuzz testing | `fuzz/` directory |
| `criterion` | Benchmarking | `crates/benchmarks/` |
| Lean4 | Formal verification | `formal/` directory |
| `cargo-mutants` | Mutation testing | CI pipeline |

### 6. Build Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Cargo | Bundled with Rust | Package manager |
| trunk | 0.21.14 | WASM frontend build |
| binaryen | System | WASM optimization |
| Docker | Latest | Container builds |
| Nix | Flake-based | Reproducible environments |

### 7. Development Procedures

| Procedure | Documentation |
|-----------|--------------|
| Code review | `.github/pull_request_template.md` |
| PR process | `CONTRIBUTING.md` |
| Issue reporting | `.github/ISSUE_TEMPLATE/` |
| Security vulnerabilities | `.github/ISSUE_TEMPLATE/security_vulnerability.md` |

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Formal verification coverage | Medium | 19 files, but not all security properties covered |
| Mutation testing results | Low | cargo-mutants in CI but no summary report |

## Verification Instructions

```bash
# Verify toolchain
rustc --version  # Should show 1.95.0
rustfmt --version
cargo clippy --version

# Verify formal verification builds
cd formal && lake build

# Verify fuzz targets compile
cargo fuzz list
cargo fuzz build

# Verify static analysis
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check

# Run property-based tests
cargo test --workspace | grep proptest
```

## References

- `rust-toolchain.toml` — Pinned Rust version
- `formal/` — Lean4 formal verification proofs
- `fuzz/` — Fuzz testing targets
- `.github/workflows/formal_verification.yml` — Formal verification CI
- `.github/workflows/sanitizers.yml` — Sanitizer CI
- `deny.toml` — cargo-deny configuration
