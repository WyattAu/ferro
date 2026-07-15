# ATE_FUN: Functional Testing

## Assurance Family Requirement

The developer shall test the security functions of the TOE and document the test plan, test results, and coverage analysis.

**EAL Level:** EAL 3+ (ATE_FUN.2)

## Evidence Artifacts

### 1. Unit Tests (2000+)

**Evidence:** All crates contain `#[cfg(test)] mod tests` blocks.

| Crate Category | Test Focus |
|----------------|-----------|
| `ferro-common` | Type serialization, error handling |
| `ferro-core` | Storage backends, CAS, search, WASM |
| `ferro-auth` | OIDC, TOTP, Cedar, LDAP |
| `ferro-crypto` | SHA-256, HMAC, password hashing |
| `ferro-server` | HTTP handlers, WebDAV, REST API |
| `ferro-server-security-middleware` | Auth middleware, CORS, rate limiting |
| `ferro-server-compliance` | WORM, retention, DLP |
| `ferro-server-storage-ops` | Upload, download, snapshots |
| `ferro-dav` | CalDAV/CardDAV protocol |

### 2. Integration Tests

**Evidence:** CI runs full integration test suite:

```bash
cargo test --locked --all --features "s3,gcs,azure,pg,redis,ldap"
```

**Test configurations (18 total):**

| Configuration | Tests |
|--------------|-------|
| Default features | Full test suite |
| Individual features (6) | `pg`, `redis`, `ldap`, `s3`, `gcs`, `azure` |
| Feature combos (12) | `pg+redis`, `pg+ldap`, `s3+pg`, `all`, etc. |
| PostgreSQL service | Live database integration |

### 3. Property-Based Tests (proptest)

**Evidence:** proptest is used for property-based testing throughout the codebase.

Key properties tested:
- Path normalization: all inputs produce valid, normalized paths
- Serialization roundtrip: serialize → deserialize preserves data
- Hash consistency: same input produces same hash
- Rate limiter: never exceeds configured limits
- CRDT convergence: concurrent operations converge

### 4. CI Test Matrix

**File:** `.github/workflows/ci.yml`

| Job | Configurations | Timeout |
|-----|---------------|---------|
| `test` | Default features, all features | 30 min |
| `test-features` | 6 individual features | 30 min |
| `test-feature-combos` | 12 feature combinations | 30 min |
| `test-pg` | PostgreSQL with service container | 30 min |
| `msrv` | MSRV compatibility (Rust 1.92) | 30 min |

**Total: 20+ unique CI configurations**

### 5. Test Coverage Areas

| Security Function | Test Coverage |
|-------------------|--------------|
| SF-AC (Access Control) | OIDC flow, TOTP enrollment, Cedar policy evaluation, RBAC |
| SF-AU (Audit) | Audit entry creation, chain verification, log retrieval |
| SF-CP (Crypto) | SHA-256 hashing, AES-GCM encryption, ECDSA signing |
| SF-KM (Key Mgmt) | Key derivation, wrapping, rotation |
| SF-DP (Data Protection) | WORM enforcement, retention policies, E2EE |
| SF-IC (Integrity) | CAS dedup, hash chain, manifest verification |
| SF-RC (Recovery) | Snapshot creation, restore, backup verification |

### 6. Test Commands

```bash
# Run all tests
cargo test --all

# Run with all features
cargo test --all --features "s3,gcs,azure,pg,redis,ldap"

# Run specific crate tests
cargo test -p ferro-auth
cargo test -p ferro-crypto
cargo test -p ferro-server

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

### 7. Test Artifacts

| Artifact | Location |
|----------|----------|
| Test source | `crates/*/src/**/*.rs` (inline tests) |
| Integration tests | `tests/` directory |
| Fuzz corpora | `fuzz/corpus/` |
| Benchmark results | `crates/benchmarks/` |

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Formal test plan | Medium | Need documented test strategy |
| Test coverage report | Medium | `cargo-tarpaulin` not in CI |
| Functional test results | Medium | Need aggregated test report |

## Verification Instructions

```bash
# Run full test suite
cargo test --all --features "s3,gcs,azure,pg,redis,ldap" 2>&1 | tail -20

# Count tests
cargo test --all 2>&1 | grep "test result" | head -20

# Verify test coverage of security functions
cargo test -p ferro-auth -- --list | wc -l
cargo test -p ferro-crypto -- --list | wc -l
cargo test -p ferro-server-security-middleware -- --list | wc -l

# Verify CI test matrix
gh run list --workflow=ci.yml --limit 1 --json jobs --jq '.jobs[] | select(.name | startswith("test")) | .name'
```

## References

- `.github/workflows/ci.yml` — CI test pipeline
- `.github/workflows/quality-gate.yml` — Quality gates
- `CONTRIBUTING.md` — Testing guidelines
- `README.md:329-336` — Development section
