# Parity Roadmap: Ferro vs Industry Standards

**Version:** 1.0 | **Date:** 2026-07-14 | **Status:** ACTIVE

---

## Roadmap Overview

This roadmap addresses the gaps identified in `docs/comparative_analysis.md`. Items are ordered by priority (Critical > High > Medium) and grouped into phases that can be executed sequentially or in parallel.

**Total estimated effort:** 8-10 weeks (single engineer)

---

## Phase 1: Architectural Resilience (Week 1-2)

**Goal:** Add circuit breakers, retry, and bulkhead patterns to prevent cascade failures.

### CR-001: Circuit Breakers on External Calls

| Attribute | Value |
|-----------|-------|
| Priority | CRITICAL |
| Standard | FANG, HFT |
| Effort | 3 days |
| Impact | Prevents cascade failures when storage/OIDC/LDAP/ClamAV goes down |

**Implementation:**
1. Use `tower::circuitbreaker` or implement the pattern from `server-infra/src/circuit_breaker.rs`
2. Wrap all external calls: storage backends (S3/GCS/Azure), OIDC provider, LDAP, ClamAV, PostgreSQL, Redis
3. Configure per-service thresholds (failure count, recovery timeout)
4. Expose circuit breaker state via health endpoint
5. Add circuit breaker state to Prometheus metrics

**Verification:**
- Unit tests: state transitions (Closed -> Open -> HalfOpen -> Closed)
- Integration test: simulate storage backend failure, verify circuit opens
- Load test: verify no cascade failure when one backend is down

### CR-002: Retry with Exponential Backoff

| Attribute | Value |
|-----------|-------|
| Priority | CRITICAL |
| Standard | FANG |
| Effort | 2 days |
| Impact | Transient network/storage failures become recoverable |

**Implementation:**
1. Create `server-retry` crate with retry middleware
2. Implement exponential backoff with jitter (base 100ms, max 10s, jitter 20%)
3. Configurable per-operation: storage ops (3 retries), OIDC (2 retries), LDAP (2 retries)
4. Add retry count to request logging
5. Circuit breaker integration: stop retrying when circuit is open

**Verification:**
- Unit tests: retry count, backoff timing, jitter distribution
- Integration test: simulate transient failure, verify retry succeeds
- Load test: verify no thundering herd on retry

### CR-003: Bulkhead Isolation

| Attribute | Value |
|-----------|-------|
| Priority | CRITICAL |
| Standard | FANG |
| Effort | 3 days |
| Impact | Resource exhaustion in one subsystem doesn't affect others |

**Implementation:**
1. Separate connection pools: storage, auth, database, cache
2. Separate async task pools: I/O-bound vs CPU-bound
3. Separate memory budgets: WASM workers vs main server
4. Configure pool sizes per subsystem
5. Monitor pool utilization via metrics

**Verification:**
- Unit tests: pool exhaustion behavior
- Load test: saturate one pool, verify others unaffected

### CR-004: SLO/SLI/Error Budget

| Attribute | Value |
|-----------|-------|
| Priority | CRITICAL |
| Standard | FANG (Google SRE) |
| Effort | 2 days |
| Impact | Quantified reliability targets |

**Implementation:**
1. Define SLOs: 99.9% availability, P99 < 500ms for reads, P999 < 2s for writes
2. Implement SLI collection: request success rate, latency percentiles
3. Wire health endpoint to SLO status (Healthy/Degraded/Unhealthy)
4. Add error budget tracking (remaining budget percentage)
5. Alert when error budget is consumed

**Verification:**
- Unit tests: SLO calculation, budget tracking
- Integration test: simulate failures, verify budget consumption

---

## Phase 2: Low-Latency Optimization (Week 3-4)

**Goal:** Reduce hot-path latency from milliseconds to microseconds.

### LL-001: Replace async RwLock with Lock-Free Structures

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | HFT, ECN |
| Effort | 1 week |
| Impact | Sub-millisecond storage operations |

**Implementation:**
1. Replace `Arc<RwLock<HashMap>>` in storage with `DashMap` (lock-striped)
2. Replace `Arc<RwLock<HashSet>>` in dedup with `DashSet`
3. Use `arc-swap` for read-heavy config data
4. Eliminate `.cloned()` on hot-path reads (use references)
5. Benchmark: target 10x improvement on get/put latency

**Verification:**
- Benchmark: before/after latency comparison
- Load test: 10K concurrent operations, measure P99

### LL-002: Zero-Copy Data Paths

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | HFT, ECN |
| Effort | 3 days |
| Impact | Eliminate allocation overhead on hot paths |

**Implementation:**
1. Replace `normalize_path().into_owned()` with borrowed path handling
2. Replace `ContentHash::compute(data.as_ref())` with streaming hash
3. Use `bytes::Bytes` (reference-counted) instead of `Vec<u8>` for data buffers
4. Eliminate `FileMetadata::new()` allocation on hot path (use pre-allocated buffer)

**Verification:**
- Benchmark: allocation count reduction
- ASAN: verify no memory leaks from borrowed paths

### LL-003: Cache-Line Alignment

| Attribute | Value |
|-----------|-------|
| Priority | MEDIUM |
| Standard | HFT |
| Effort | 2 days |
| Impact | Reduced cache misses on hot-path structs |

**Implementation:**
1. Add `#[repr(align(64))]` to `FileMetadata`, `CASKey`, hot-path structs
2. Group related fields on same cache line
3. Use `#[cold]` on error paths
4. Use `#[inline]` on small hot-path functions

**Verification:**
- Benchmark: cache miss rate before/after (perf stat)

### LL-004: io_uring Integration

| Attribute | Value |
|-----------|-------|
| Priority | MEDIUM |
| Standard | HFT, ECN |
| Effort | 2 weeks |
| Impact | Kernel-bypass I/O for maximum throughput |

**Implementation:**
1. Evaluate `tokio-uring` or `monoio` for storage I/O
2. Replace `tokio::fs::read/write` with io_uring operations
3. Batch I/O operations for sequential access patterns
4. Benchmark: compare with standard async I/O

**Verification:**
- Benchmark: IOPS improvement
- Correctness: verify fsync semantics preserved

---

## Phase 3: Defense/Mil-Spec Compliance (Week 5-7)

**Goal:** Meet FIPS 140-2/3 and prepare for Common Criteria evaluation.

### DM-001: FIPS 140-2/3 Runtime Validation

| Attribute | Value |
|-----------|-------|
| Priority | CRITICAL |
| Standard | FIPS 140-2/3 |
| Effort | 1 week |
| Impact | Deployable to regulated environments |

**Implementation:**
1. Enable FIPS mode at startup (not just compile-time flag)
2. Add CAVP algorithm testing on startup
3. Add DRBG health checks (NIST SP 800-90A Section 4)
4. Add continuous RNG test
5. Add power-up self-test for all cryptographic algorithms
6. Log FIPS mode status in health endpoint

**Verification:**
- Unit tests: FIPS mode activation, self-test pass/fail
- Integration test: verify FIPS mode with hardware RNG

### DM-002: Key Hierarchy and Management

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | NIST SP 800-57 |
| Effort | 1 week |
| Impact | Key compromise doesn't expose all data |

**Implementation:**
1. Implement key hierarchy: Master Key -> Key Encryption Keys -> Data Keys
2. Add key wrapping (AES-256-KW) for data key storage
3. Add key versioning (key ID in encrypted data header)
4. Add key rotation (background job, configurable interval)
5. Add key destruction verification (overwrite + verify)
6. Add HSM interface (PKCS#11) for master key storage

**Verification:**
- Unit tests: key wrapping/unwrapping, rotation, destruction
- Integration test: key rotation without data loss
- Security test: verify old keys cannot decrypt new data

### DM-003: AES-GCM Data-at-Rest Encryption

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | FIPS 197 |
| Effort | 3 days |
| Impact | FIPS-compliant data protection |

**Implementation:**
1. Add AES-256-GCM encryption option (alongside existing `age` encryption)
2. Implement transparent data encryption (TDE) for storage backends
3. Add envelope encryption: data key encrypted by master key
4. Add encryption metadata in file headers
5. Make encryption opt-in via config flag

**Verification:**
- Unit tests: encrypt/decrypt roundtrip, key rotation
- FIPS test: verify AES-GCM implementation passes CAVP

### DM-004: Post-Quantum Readiness

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | NIST SP 800-208 |
| Effort | 2 weeks |
| Impact | Future-proof against quantum attacks |

**Implementation:**
1. Add ML-KEM (Kyber) for key exchange (hybrid mode: X25519 + ML-KEM)
2. Add ML-DSA (Dilithium) for signatures (hybrid mode: ECDSA + ML-DSA)
3. Implement algorithm agility framework (configurable algorithm selection)
4. Add migration path: encrypt with PQ + classical, decrypt with either
5. Benchmark: PQ overhead vs classical

**Verification:**
- Unit tests: hybrid key exchange, signature verification
- Benchmark: PQ overhead measurement

---

## Phase 4: Testing Hardening (Week 7-8)

**Goal:** Increase test ratio from 1.15% to 10%+ and add missing test categories.

### TH-001: Increase Test Coverage

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | All |
| Effort | 2 weeks |
| Impact | Regression protection |

**Implementation:**
1. Add integration tests for all feature-flag combinations (pg+redis, s3+ldap, etc.)
2. Add property-based tests for all data type parsers (iCal, vCard, XML, JSON)
3. Add conformance tests for WebDAV RFC 4918 compliance suite
4. Add performance regression tests (benchmark thresholds in CI)
5. Target: 5,000+ tests, 10%+ test-to-code ratio

**Verification:**
- CI: all new tests pass
- Coverage: measure with `cargo-llvm-cov`

### TH-002: Security Fuzzing Expansion

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | Defense |
| Effort | 3 days |
| Impact | Vulnerability detection in crypto/auth paths |

**Implementation:**
1. Add fuzz targets for: OIDC token validation, SAML parsing, WebAuthn attestation
2. Add fuzz targets for: E2EE encrypt/decrypt, key wrapping, AES-GCM
3. Add fuzz targets for: WORM policy enforcement, retention policy evaluation
4. Add fuzz targets for: circuit breaker state machine transitions
5. Run fuzz targets for 1 hour each in CI

**Verification:**
- Fuzz: 0 crashes after 1 hour per target
- Coverage: measure fuzz code coverage

### TH-003: Static Analysis Expansion

| Attribute | Value |
|-----------|-------|
| Priority | MEDIUM |
| Standard | Defense |
| Effort | 2 days |
| Impact | UB detection, memory safety verification |

**Implementation:**
1. Add Miri CI job for unsafe code paths
2. Add Kani model checking for critical algorithms
3. Add `#![forbid(unsafe_code)]` on new crates
4. Audit existing 42 unsafe blocks, add SAFETY comments

**Verification:**
- Miri: 0 UB detections
- Kani: all properties verified

---

## Phase 5: Documentation & Certification (Week 9-10)

**Goal:** Prepare compliance artifacts for regulated market entry.

### DC-001: NIST SP 800-53 Control Mapping

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | NIST SP 800-53 |
| Effort | 3 days |
| Impact | Compliance documentation |

**Implementation:**
1. Map all security controls to NIST SP 800-53 families
2. Document implementation status for each control
3. Create POA&M (Plan of Action and Milestones) for gaps
4. Document continuous monitoring plan

**Verification:**
- Audit: all controls mapped, no orphaned controls

### DC-002: Security Assessment Report

| Attribute | Value |
|-----------|-------|
| Priority | HIGH |
| Standard | Defense |
| Effort | 2 days |
| Impact | Third-party security validation |

**Implementation:**
1. Execute pentest guide in `docs/security/penetration_test_scope.md`
2. Document findings and remediations
3. Create security assessment report
4. Schedule annual re-assessment

**Verification:**
- Report: all test categories executed, findings documented

### DC-003: Common Criteria Preparation

| Attribute | Value |
|-----------|-------|
| Priority | MEDIUM |
| Standard | Common Criteria |
| Effort | 1 week |
| Impact | EAL certification readiness |

**Implementation:**
1. Define TOE (Target of Evaluation) boundary
2. Create Security Target (ST) document
3. Document assurance family requirements (ADV, AGD, ALC, ATE, AVA)
4. Create evaluation readiness checklist

**Verification:**
- Document: ST complete, all assurance families addressed

---

## Execution Timeline

```
Week 1-2:  Phase 1 (Architectural Resilience) ── CR-001, CR-002, CR-003, CR-004
Week 3-4:  Phase 2 (Low-Latency Optimization) ── LL-001, LL-002, LL-003
Week 5-7:  Phase 3 (Defense/Mil-Spec) ── DM-001, DM-002, DM-003, DM-004
Week 7-8:  Phase 4 (Testing Hardening) ── TH-001, TH-002, TH-003
Week 9-10: Phase 5 (Documentation & Certification) ── DC-001, DC-002, DC-003
```

**Parallel tracks:**
- Phase 1 and Phase 2 can run in parallel (different engineers)
- Phase 3 depends on Phase 1 (circuit breakers needed for FIPS self-test)
- Phase 4 can start after Phase 1 (test the new resilience patterns)
- Phase 5 can start after Phase 3 (document the FIPS implementation)

---

## Success Criteria

| Metric | Current | Target | Timeline |
|--------|---------|--------|----------|
| Circuit breakers | 0 | All external calls | Week 2 |
| Retry middleware | 0 | All transient operations | Week 2 |
| SLO definitions | 0 | 3 SLOs defined | Week 2 |
| P99 latency (reads) | ~50ms | <5ms | Week 4 |
| FIPS mode | Feature-gated | Runtime-validated | Week 6 |
| Key hierarchy | None | 3-level hierarchy | Week 7 |
| Test ratio | 1.15% | 10%+ | Week 8 |
| Fuzz targets | 12 | 20+ | Week 8 |
| NIST 800-53 mapping | None | All applicable controls | Week 10 |
