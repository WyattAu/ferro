# ATE_IND: Independent Testing

## Assurance Family Requirement

The evaluator shall perform independent testing to verify the security functions of the TOE, including functional testing, vulnerability analysis, and penetration testing.

**EAL Level:** EAL 3+ (ATE_IND.2)

## Evidence Artifacts

### 1. Fuzz Testing (15 Targets, 2.6M+ Iterations)

**Path:** `fuzz/`

| Target | Component | Purpose |
|--------|-----------|---------|
| `fuzz_escape_xml` | WebDAV handler | XML injection, entity expansion |
| `fuzz_xml` | XML parser | Malformed XML input |
| `fuzz_json` | JSON parser | Malformed JSON input |
| `fuzz_ical` | CalDAV | iCalendar edge cases |
| `fuzz_vcard` | CardDAV | vCard edge cases |
| `fuzz_calcardav` | CalDAV/CardDAV | Protocol parsing |
| `fuzz_proppatch` | WebDAV | PROPPATCH handling |
| `fuzz_lock_request` | WebDAV | LOCK request parsing |
| `fuzz_path_normalize` | Core | Path traversal, normalization |
| `fuzz_api_auth` | Auth | Authentication bypass attempts |
| `fuzz_config` | Config | Configuration parsing |
| `fuzz_wasm_magic` | WASM | WASM module validation |
| `fuzz_crypto` | Crypto | Cryptographic operations |
| `fuzz_fips` | FIPS | FIPS compliance |
| `fuzz_circuit_breaker` | Resilience | Circuit breaker state |

**Total:** 2.6M+ fuzzing iterations

**Corpus:** `fuzz/corpus/` — accumulated test inputs
**Artifacts:** `fuzz/artifacts/` — crash reproductions

### 2. Property-Based Testing (proptest)

**Evidence:** proptest generates random inputs to verify properties hold for all inputs.

Key property tests:
- **Path safety:** Any path input produces valid, normalized output
- **Serialization roundtrip:** Any struct serializes and deserializes correctly
- **Hash consistency:** Same input produces same hash; different input produces different hash
- **Rate limiting:** Request count never exceeds configured burst
- **CRDT convergence:** Concurrent operations converge to consistent state

### 3. Mutation Testing (cargo-mutants)

**Evidence:** cargo-mutants introduces mutations to verify test suite detects them.

```bash
cargo mutants --workspace
```

CI integration ensures tests catch code mutations that change behavior.

### 4. Formal Verification (Independent)

**Path:** `formal/` — 19 Lean4 proof files

Properties formally verified:
- Authentication correctness
- Token security properties
- Path traversal prevention
- Hash function properties
- Cache consistency
- Rate limiter correctness
- Circuit breaker behavior
- CRDT convergence

### 5. Security Scanning (Independent)

| Scanner | Scope | Enforcement |
|---------|-------|-------------|
| `cargo-deny` | Dependencies (advisories, licenses, bans, sources) | Required in CI |
| Trivy | Filesystem vulnerability scan | CRITICAL/HIGH blocks CI |
| `cargo audit` | Rust security advisories | Via cargo-deny |
| SBOM generation | SPDX JSON | Generated per build |

### 6. Penetration Test Scope

**File:** `docs/security/penetration_test_scope.md`

| Category | Targets |
|----------|---------|
| Authentication | OIDC, TOTP, WebAuthn, SAML, LDAP, API keys |
| Authorization | Cedar RBAC, role hierarchy, path validation |
| Input validation | Path traversal, XML injection, SSRF |
| Cryptography | Key management, encryption, hashing |
| API security | Rate limiting, CORS, session management |
| Infrastructure | Docker, TLS, network security |

**Scope:** Server binary only (Web UI out of scope)
**Timeline:** 4-week engagement recommended

### 7. Security Assessment

**File:** `docs/compliance/security_assessment.md`

| Method | Result |
|--------|--------|
| STRIDE threat model | All 6 categories assessed |
| NIST SP 800-53 | 24/27 controls implemented (89%) |
| OWASP ASVS v4.0 | 8/10 categories compliant |
| SOC 2 Type II | 3/5 criteria aligned |

**Findings:** 0 critical, 2 high, 3 medium, 3 low

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Penetration test execution | High | Scope defined, test not yet conducted |
| Independent audit report | Medium | Need third-party assessment |
| Fuzzing summary report | Low | No aggregated fuzzing results document |

## Verification Instructions

```bash
# Run fuzz targets (short run)
cargo fuzz run fuzz_escape_xml -- -max_len=1024 -max_total_time=60
cargo fuzz run fuzz_path_normalize -- -max_len=256 -max_total_time=60

# Run property-based tests
cargo test --workspace | grep -i proptest

# Run cargo-deny
cargo deny check advisories bans licenses sources

# Run Trivy scan
trivy fs --severity CRITICAL,HIGH --exit-code 1 .

# Verify formal verification
cd formal && lake build
```

## References

- `fuzz/` — Fuzz testing targets and corpora
- `formal/` — Lean4 formal verification
- `docs/security/penetration_test_scope.md` — Pentest scope
- `docs/compliance/security_assessment.md` — Security assessment
- `docs/compliance/nist_sp80053_mapping.md` — NIST control mapping
- `.github/workflows/sanitizers.yml` — Sanitizer CI
