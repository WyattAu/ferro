# AVA_VAN: Vulnerability Assessment

## Assurance Family Requirement

The developer shall perform a vulnerability analysis to identify potential vulnerabilities in the TOE, and the evaluator shall perform an independent vulnerability analysis.

**EAL Level:** EAL 3+ (AVA_VAN.3)

## Evidence Artifacts

### 1. cargo-deny Results

**File:** `deny.toml`

**CI Enforcement:** `.github/workflows/ci.yml:179-192`

| Check | Status | Notes |
|-------|--------|-------|
| Advisories | Pass | 6 documented ignores with rationale |
| Licenses | Pass | 14 approved licenses |
| Bans | Pass | No wildcard dependencies |
| Sources | Pass | No unknown registries or git sources |

**Documented Advisory Ignores:**

| Advisory | Package | Rationale |
|----------|---------|-----------|
| RUSTSEC-2025-0141 | bincode | Transitive via fuse3, FUSE-only, no server impact |
| RUSTSEC-2024-0436 | paste | Transitive via leptos, web frontend only |
| RUSTSEC-2024-0384 | instant | Transitive via reed-solomon-erasure, distributed only |
| RUSTSEC-2026-0173 | proc-macro-error2 | Transitive via leptos, web frontend only |
| RUSTSEC-2026-0204 | (transitive) | Limited scope, no server impact |
| RUSTSEC-2025-0119 | (transitive) | Limited scope, no server impact |

### 2. Trivy Scan Results

**CI Enforcement:** `.github/workflows/ci.yml:223-239`

```yaml
scan-type: 'fs'
scan-ref: '.'
format: 'table'
exit-code: '1'  # Fails on findings
severity: 'CRITICAL,HIGH'
ignore-unfixed: true
```

**Configuration:** Trivy scans the filesystem for known vulnerabilities in dependencies. CI fails on any CRITICAL or HIGH severity finding.

### 3. Penetration Test Scope

**File:** `docs/security/penetration_test_scope.md`

**Scope Definition:**

| Category | Targets | Depth |
|----------|---------|-------|
| Authentication | OIDC, TOTP, WebAuthn, SAML, LDAP, API keys | Bypass attempts |
| Authorization | Cedar RBAC, role hierarchy, path validation | Privilege escalation |
| Input validation | Path traversal, XML injection, SSRF, XSS | Exploitation |
| Cryptography | Key management, encryption, hashing | Weakness analysis |
| API security | Rate limiting, CORS, session management | Abuse testing |
| Infrastructure | Docker, TLS, network | Configuration review |

**Severity Classification:**
- Critical: Remote code execution, auth bypass, data exfiltration
- High: Privilege escalation, significant data exposure
- Medium: Limited data exposure, denial of service
- Low: Information disclosure, minor issues

### 4. Security Assessment

**File:** `docs/compliance/security_assessment.md`

**STRIDE Threat Model Results:**

| Threat | Mitigation | Residual Risk |
|--------|------------|---------------|
| Spoofing | OIDC/PKCE, SAML validation, HMAC signatures | Low |
| Tampering | Hash chain audit, WORM, content hashing | Low |
| Repudiation | Comprehensive audit logging, chain verification | Low |
| Information Disclosure | E2EE, TLS 1.3, bcrypt | Low (H-001 gap) |
| Denial of Service | Rate limiting, WASM fuel limits, body limits | Low |
| Elevation of Privilege | Cedar RBAC, role hierarchy, path validation | Low |

**Findings Summary:**

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 0 | None |
| High | 2 | H-001 (zeroize), H-002 (key hierarchy) |
| Medium | 3 | M-001 (secure deletion), M-002 (config audit), M-003 (hot-reload) |
| Low | 3 | L-001 (SAML XML-DSIG), L-002 (guest audit), L-003 (per-route limits) |

### 5. Security Controls Matrix

**File:** `docs/security/security_controls_matrix.md`

Maps implemented controls to security requirements across:
- NIST SP 800-53 Rev. 5 (27 controls, 89% implemented)
- OWASP ASVS v4.0 (8/10 categories compliant)
- SOC 2 Type II (3/5 criteria aligned)

### 6. Attack Scenarios

**File:** `docs/security/attack_scenarios.md`

Documented attack scenarios covering:
- Unauthorized file access
- Audit log tampering
- Key compromise
- API abuse
- Container escape
- Federation spoofing

### 7. Audit Scope

**File:** `docs/security/audit_scope.md`

Defines the scope for security audits including:
- Code review boundaries
- Infrastructure review boundaries
- Third-party dependency assessment
- Compliance audit requirements

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Penetration test execution | High | Scope defined, test not yet conducted |
| Threat model document | High | STRIDE assessment done, formal model needed |
| Vulnerability scan history | Medium | Need trend tracking |
| Zeroize for secret material | High | H-001: `zeroize` crate integration needed |
| Key hierarchy documentation | High | H-002: Formal key derivation chain needed |

## Verification Instructions

```bash
# Run cargo-deny
cargo deny check advisories bans licenses sources

# Run Trivy scan
trivy fs --severity CRITICAL,HIGH --exit-code 1 .

# Run cargo audit directly
cargo audit

# Check for unsafe code in security paths
grep -r "unsafe" crates/server-security/src/ crates/crypto/src/ crates/auth/src/

# Verify no secrets in code
git log --all --oneline | head -20
# Check for any committed secrets in history

# Review security assessment
cat docs/compliance/security_assessment.md | grep -A5 "Findings"
```

## References

- `deny.toml` — cargo-deny configuration
- `.github/workflows/ci.yml:179-239` — Security scanning CI
- `docs/security/penetration_test_scope.md` — Pentest scope document
- `docs/compliance/security_assessment.md` — Security assessment report
- `docs/compliance/nist_sp80053_mapping.md` — NIST control mapping
- `docs/security/security_controls_matrix.md` — Controls matrix
- `docs/security/attack_scenarios.md` — Attack scenarios
- `docs/security/audit_scope.md` — Audit scope
