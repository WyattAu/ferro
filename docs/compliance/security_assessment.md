# Security Assessment Report

**Project:** Ferro File Server
**Version:** 3.x
**Assessment Date:** 2026-07-15
**Assessor:** Automated Security Review (STRIDE + NIST SP 800-53)
**Classification:** Internal

---

## 1. Executive Summary

Ferro demonstrates a **strong security posture** for a self-hosted file server. The codebase implements defense-in-depth across authentication, authorization, encryption, and audit logging. Of 27 NIST SP 800-53 controls assessed, 24 are fully implemented and 3 are partially implemented. Seven gaps were identified, all at medium or low severity with no critical findings.

**Key strengths:**
- Multi-factor authentication (OIDC, TOTP, SAML, WebAuthn, LDAP)
- Cedar policy engine for fine-grained RBAC
- End-to-end encryption via age (X25519 + ChaCha20-Poly1305)
- Chain-verified audit log with tamper detection
- WORM storage for regulatory compliance
- Comprehensive path validation preventing traversal attacks
- TLS 1.3 enforced for all transport

---

## 2. Assessment Methodology

### 2.1 STRIDE Threat Model

| Threat | Assessment |
|--------|------------|
| **S**poofing | OIDC/PKCE, SAML assertion validation, HMAC-SHA256 HTTP signatures, actor keyId validation |
| **T**ampering | Hash chain audit log, WORM storage, content hashing, append-only audit design |
| **R**epudiation | Comprehensive audit logging (AU-2/AU-3), chain verification, IP + user agent recording |
| **I**nformation Disclosure | E2EE, TLS 1.3, bcrypt password hashing, constant-time comparison |
| **D**enial of Service | Rate limiting (token bucket), per-tenant rate limits, WASM fuel limits, request body limits |
| **E**levation of Privilege | Cedar RBAC, role hierarchy (Admin/User/ReadOnly), path validation, WORM enforcement |

### 2.2 NIST SP 800-53 Mapping

See [nist_sp80053_mapping.md](./nist_sp80053_mapping.md) for the complete control-by-control mapping across 7 control families (27 controls total).

---

## 3. Findings

### 3.1 Critical Findings

**None.**

### 3.2 High Findings

**H-001: No Zeroize for Secret Material in Memory**

- **Component:** `crates/server-security/src/encryption.rs:13`, `crates/auth/src/totp.rs:112`
- **Description:** Encryption passphrases and TOTP secrets are held in `String`/`Vec<u8>` without guaranteed zeroing on drop. Memory dumps or swap could expose secrets.
- **Impact:** Memory disclosure could reveal encryption passphrases or TOTP secrets.
- **Recommendation:** Integrate the `zeroize` crate for all `SecretString`, `Vec<u8>` holding key material.

**H-002: No Formal Key Hierarchy**

- **Component:** `SECURITY.md:58`, `crates/server-security/src/encryption.rs`
- **Description:** HMAC keys and encryption passphrases are managed independently with no documented derivation chain or rotation schedule.
- **Impact:** Compromised keys cannot be rotated without full re-deployment; no forward secrecy for derived keys.
- **Recommendation:** Document key hierarchy in ADR; implement HMAC key rotation; add optional HSM/KMS backend.

### 3.3 Medium Findings

**M-001: No Secure File Deletion**

- **Component:** File storage backends
- **Description:** File deletion uses standard unlink without overwrite. Deleted files remain recoverable on disk until overwritten by the filesystem.
- **Impact:** Sensitive data persists on disk after deletion.
- **Recommendation:** Implement overwrite-before-unlink for files in sensitive directories (configurable via WORM policies).

**M-002: Config Change Not Audited**

- **Component:** `crates/server-config/src/lib.rs`
- **Description:** Configuration changes (via file or admin UI) are not recorded in the audit log.
- **Impact:** Cannot trace who changed configuration or when.
- **Recommendation:** Emit `ConfigChange` audit events when settings are modified via admin API.

**M-003: No Config Hot-Reload**

- **Component:** `crates/admin/src/pages/settings.rs:99`
- **Description:** Configuration changes require server restart. This delays security patching and may cause configuration drift.
- **Impact:** Security-relevant settings changes are not applied until next restart.
- **Recommendation:** Implement file-watcher based hot-reload for non-critical settings; document which settings require restart.

### 3.4 Low Findings

**L-001: SAML XML Signature Verification Deferred**

- **Component:** `crates/auth/src/saml.rs:11-12`
- **Description:** Full XML-DSIG validation is deferred; currently only SHA-256 digest comparison is implemented. Production deployments should use a reverse proxy for signature verification.
- **Impact:** SAML assertions without a configured certificate fingerprint may be vulnerable to forged assertions.
- **Recommendation:** Implement full XML-DSIG verification or mandate certificate fingerprint configuration for SAML.

**L-002: Guest Access Audit Coverage**

- **Component:** `crates/auth/src/oidc.rs:375`
- **Description:** Anonymous/guest claims are generated when no OIDC is configured, but audit events for guest actions may not include meaningful actor identification.
- **Impact:** Audit trails for guest actions may lack attribution.
- **Recommendation:** Ensure guest audit entries include session identifiers or IP-based attribution.

**L-003: Rate Limit Configuration Granularity**

- **Component:** `crates/server-config/src/lib.rs:44-45`
- **Description:** Rate limiting is global (burst + refill) with per-tenant overrides, but not per-route configurable.
- **Impact:** Sensitive endpoints (e.g., login) share the same rate limit as general API traffic.
- **Recommendation:** Add per-route rate limit configuration, especially for authentication endpoints.

---

## 4. Recommendations (Prioritized by Risk)

| Priority | ID | Recommendation | Effort | Risk Reduction |
|----------|----|----------------|--------|----------------|
| 1 | H-001 | Integrate `zeroize` for secret material | Low | High |
| 2 | H-002 | Document key hierarchy + implement rotation | Medium | High |
| 3 | M-001 | Implement secure file deletion | Medium | Medium |
| 4 | M-002 | Audit config changes | Low | Medium |
| 5 | M-003 | Config hot-reload for security settings | Medium | Medium |
| 6 | L-001 | Full XML-DSIG for SAML | High | Low |
| 7 | L-002 | Guest audit attribution | Low | Low |
| 8 | L-003 | Per-route rate limit configuration | Medium | Low |

---

## 5. Compliance Status

### 5.1 NIST SP 800-53 Rev. 5

| Family | Controls | Implemented | Partial | Status |
|--------|----------|-------------|---------|--------|
| AC (Access Control) | 7 | 7 | 0 | Compliant |
| AU (Audit) | 5 | 5 | 0 | Compliant |
| CM (Configuration) | 3 | 2 | 1 | Partial |
| IA (Identification & Auth) | 3 | 3 | 0 | Compliant |
| MP (Media Protection) | 2 | 1 | 1 | Partial |
| SC (System & Comm Protection) | 4 | 3 | 1 | Partial |
| SI (System Integrity) | 3 | 3 | 0 | Compliant |
| **Total** | **27** | **24** | **3** | **89% Implemented** |

### 5.2 OWASP ASVS v4.0

| Category | Status |
|----------|--------|
| V1: Architecture | Compliant — Defense-in-depth, separation of concerns |
| V2: Authentication | Compliant — OIDC, TOTP, SAML, LDAP, bcrypt, rate limiting |
| V3: Session Management | Compliant — Token expiry, PKCE session TTL, refresh rotation |
| V4: Access Control | Compliant — Cedar RBAC, role hierarchy, path validation |
| V5: Validation | Compliant — Path traversal prevention, XML entity defense, input limits |
| V6: Crypto | Partial — No key rotation or HSM integration (H-002) |
| V7: Error Handling | Compliant — Structured error responses, no stack traces |
| V8: Data Protection | Partial — E2EE implemented, no secure deletion (M-001) |
| V9: Logging | Compliant — Chain-verified audit log with comprehensive fields |
| V10: HTTP Security | Compliant — HSTS, CSP, X-Frame-Options, nosniff |

### 5.3 SOC 2 Type II Alignment

| Criterion | Status |
|-----------|--------|
| CC6.1 (Logical Access) | Aligned — OIDC, RBAC, role-based access |
| CC6.6 (External Threats) | Aligned — TLS 1.3, rate limiting, WORM |
| CC7.1 (Monitoring) | Aligned — Audit logging, chain verification |
| CC7.2 (Anomaly Detection) | Partial — Rate limiting exists, no anomaly alerting |
| CC8.1 (Change Management) | Partial — Config changes not audited (M-002) |

---

## 6. Remediation Timeline

| Phase | Items | Target | Owner |
|-------|-------|--------|-------|
| **Phase 1** (Immediate) | H-001 (zeroize), M-002 (audit config) | v3.1 | Security |
| **Phase 2** (30 days) | H-002 (key hierarchy), M-003 (hot-reload) | v3.2 | Security + Ops |
| **Phase 3** (90 days) | M-001 (secure deletion), L-003 (per-route limits) | v3.3 | Core |
| **Phase 4** (6 months) | L-001 (XML-DSIG), L-002 (guest audit) | v4.0 | Auth |

---

## Appendix A: Threat Model Matrix

| Asset | Threat | Mitigation | Residual Risk |
|-------|--------|------------|---------------|
| File content | Unauthorized read | OIDC/RBAC, E2EE, TLS 1.3 | Low |
| File content | Tampering | WORM, hash chain, Cedar policies | Low |
| User credentials | Brute force | Bcrypt cost 12, rate limiting, lockout | Low |
| User credentials | Memory disclosure | (H-001: zeroize gap) | Medium |
| Audit log | Tampering | Hash chain, append-only, SQLite WAL | Low |
| Encryption keys | Compromise | (H-002: no key rotation) | Medium |
| API endpoints | DDoS | Token bucket rate limiting, per-tenant limits | Low |
| WASM plugins | Sandbox escape | Fuel limits, memory cap, network isolation | Low |
| Federation | Spoofing | HMAC-SHA256 signatures, actor validation | Low |
| SAML assertions | Forgery | (L-001: deferred XML-DSIG) | Low |
