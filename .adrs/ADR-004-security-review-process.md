# ADR-004: Security Review Process

**Status:** Accepted
**Date:** 2026-07-12
**Deciders:** Wyatt (Sole developer)

## Context

Ferro handles sensitive data (files, credentials, federation tokens, E2E encryption keys) and exposes multiple attack surfaces (WebDAV, REST API, CalDAV, GraphQL, WASM sandbox, federation inbox, FUSE mount). The existing `SECURITY.md` defines a vulnerability disclosure process and STRIDE threat model, but there is no formal process for when internal security reviews are required during development, who performs them, or how threats are modeled for new features.

As a solo-developer project, there is no security team. The review process must be lightweight enough for one person to follow consistently, yet rigorous enough to catch real vulnerabilities before they ship.

## Decision

### When Security Reviews Are Required

| Trigger | Review Type | Scope |
|---------|------------|-------|
| New authentication/authorization endpoint | Full threat model | STRIDE analysis + code review |
| New external-facing API surface | Full threat model | STRIDE analysis + code review |
| New storage backend or format change | Data flow analysis | Encryption, access control, integrity |
| New dependency with >1M downloads | Dependency audit | CVE check, license, maintainer status |
| New dependency with <1M downloads | Enhanced review | Source audit, fuzzing, replaceability assessment |
| WASM host function changes | Sandboxing review | Resource limits, escape vectors |
| Federation protocol changes | Protocol review | Signature verification, spoofing, injection |
| Cryptographic code changes | Crypto review | Algorithm correctness, constant-time, key management |
| Any `unsafe` code | Unsafe audit | SAFETY comments, Miri testing, FFI bounds |
| Configuration format changes | Config review | Defaults, secrets handling, privilege escalation |
| Bug fix for a security-tagged issue | Regression check | Verify fix, check for variants |

### Threat Modeling Process

For each new feature that triggers a full threat model:

1. **Identify assets**: What data/code/credentials does this feature handle?
2. **Identify trust boundaries**: Where does untrusted input cross into trusted processing?
3. **STRIDE analysis**: For each component, enumerate Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Elevation of Privilege risks
4. **Rate risk**: Use the existing STRIDE table in `SECURITY.md` as a template; add new rows for new attack surfaces
5. **Document mitigations**: Each identified threat must have a documented mitigation or accepted risk
6. **Implement controls**: Code the mitigations
7. **Verify**: Write security-focused tests (see existing 33 security tests in CI)

### Vulnerability Response SLA

Maintains the existing `SECURITY.md` response timeline with no changes:

| Severity | Initial Response | Patch Release |
|----------|-----------------|---------------|
| Critical (RCE, auth bypass) | 24 hours | 72 hours |
| High (data exposure, privilege escalation) | 48 hours | 7 days |
| Medium (CSRF, XSS, information disclosure) | 72 hours | 14 days |
| Low (best practices, minor issues) | 1 week | Next release |

### Security Review Checklist

Each review must verify:

```markdown
## Security Review: [Feature Name]

**Reviewer:** Wyatt
**Date:** [date]
**Feature:** [description]

### Input Validation
- [ ] All user input is validated (path, headers, body, query params)
- [ ] Path traversal prevention (normalized paths, `..` rejection)
- [ ] Content-Type validation
- [ ] Request body size limits enforced

### Authentication & Authorization
- [ ] Endpoints require appropriate auth (Basic, OIDC, or public)
- [ ] Authorization checked (Cedar policy evaluation)
- [ ] No privilege escalation paths
- [ ] Session/token handling is secure

### Cryptography
- [ ] No custom crypto (use established crates: aes-gcm, sha2, x25519-dalek)
- [ ] Constant-time comparisons for secrets
- [ ] Key material zeroized after use
- [ ] TLS 1.3 for transport

### Data Protection
- [ ] Sensitive data not logged (passwords, tokens, keys)
- [ ] Error messages don't leak internal state
- [ ] Audit logging captures access events

### DoS Resistance
- [ ] Rate limiting applied
- [ ] Resource limits enforced (body size, recursion depth, timeout)
- [ ] No unbounded allocations from user input

### Dependencies
- [ ] No new critical CVEs in dependencies
- [ ] cargo-deny passes
- [ ] No copyleft licenses introduced (AGPL-3.0 compatible only)

### Testing
- [ ] Security-focused unit tests written
- [ ] Edge cases covered (empty input, oversized input, malformed input)
- [ ] Existing security tests still pass
```

### Audit Schedule

| Activity | Frequency | Tool/Method |
|----------|-----------|-------------|
| `cargo audit` / `cargo deny` | Weekly (CI) | Automated |
| Dependency review | Monthly | Manual review of new dependencies |
| Full security review | Per feature (see triggers above) | Checklist + STRIDE |
| Penetration test | Before major releases | Manual + nuclei + sqlmap |
| `cargo miri test` | Monthly | Miri for unsafe code |
| Fuzz regression | Per release | 4 cargo-fuzz harnesses (2.6M+ iterations baseline) |

### Incident Response

When a vulnerability is confirmed:

1. **Triage**: Assign severity per `SECURITY.md` SLA
2. **Fix**: Develop and test fix in a private branch (if CVE is public, use `security` branch)
3. **Release**: Patch release within SLA timeline
4. **Disclose**: Publish GitHub Security Advisory after patch is available
5. **Retrospective**: Update `SECURITY.md` STRIDE table with new attack surface, add regression test

## Consequences

### Positive
- Structured process catches vulnerabilities before they ship
- Checklist ensures consistent review quality even under time pressure
- STRIDE model grows with the project (new surfaces documented as added)
- Leverages existing infrastructure (cargo-deny, fuzzing, security tests)

### Negative
- Solo developer is reviewer and implementer -- no separation of duties
- Checklist adds friction to feature development (estimated 30-60 min per review)
- Some threats are hard to test (e.g., timing side channels, physical access)

### Risks
- Review fatigue: under deadline pressure, reviews may become rubber-stamp
- Unknown unknowns: STRIDE doesn't cover every possible attack vector
- Federation security depends on third-party implementations being correct

## Alternatives Considered

### No Formal Process (Ad-Hoc)
- **Description:** Review security only when reminded by CVE alerts or bug reports
- **Pros:** Zero overhead, maximum velocity
- **Cons:** Reactive security means vulnerabilities ship to users; unacceptable for a file server
- **Why Rejected:** Existing pen test scope document (`docs/security/penetration_test_scope.md`) shows real attack surfaces that need proactive review

### External Security Audit
- **Description:** Hire a firm for annual penetration test
- **Pros:** Professional review, unbiased perspective, CVE assignment support
- **Cons:** Expensive ($10K-50K+), infrequent (annual at best), solo-developer budget constraints
- **Why Rejected:** Not feasible for a solo-developer open-source project; self-audit is the baseline

### Automated-Only Security (SAST/DAST)
- **Description:** Rely entirely on cargo-audit, cargo-deny, fuzzing, and Miri
- **Pros:** Fully automated, zero manual overhead
- **Cons:** Cannot catch logic flaws (e.g., authorization bypass via business logic), protocol-level issues, or design-level threats
- **Why Rejected:** Automation is a component, not a replacement for human threat modeling

## Related ADRs
- [ADR-001](ADR-001-error-budget-policy.md) -- Error Budget Policy (security incidents trigger reliability freeze)
- [ADR-002](ADR-002-deprecation-policy.md) -- Deprecation Policy (security fixes may bypass deprecation cycle)

## References
- Ferro SECURITY.md: `SECURITY.md` (existing vulnerability disclosure process)
- STRIDE threat model: `SECURITY.md` (existing, updated 2026-05-24)
- Pen test scope: `docs/security/penetration_test_scope.md` (80+ test cases)
- OWASP ASVS: Application Security Verification Standard
- cargo-deny: `deny.toml` (existing configuration)
- cargo-fuzz: `fuzz/` directory (4 harnesses)
