# Ferro Security Controls Matrix

**Document:** Security Controls Mapping to OWASP ASVS v4.0 & NIST SP 800-53  
**Version:** 1.0.0  
**Date:** 2026-07-12  
**Status:** Draft  

---

## Overview

This document maps Ferro's security controls to industry standards:
- **OWASP ASVS v4.0** — Application Security Verification Standard
- **NIST SP 800-53 Rev. 5** — Security and Privacy Controls

### Coverage Summary

| Domain | ASVS Sections | NIST Families | Ferro Controls | Coverage |
|--------|---------------|---------------|----------------|----------|
| Authentication | 2.1–2.6 | IA (Identification & Authentication) | OIDC, Basic, TOTP, WebAuthn, SAML, API Keys | High |
| Authorization | 4.1–4.3 | AC (Access Control) | Cedar policy engine, role-based access | High |
| Cryptography | 6.1–6.6 | SC (System & Communications Protection) | AES-GCM, X25519, age, TLS 1.3, Argon2 | High |
| Data Protection | 7.1–7.4 | MP (Media Protection) | Encryption at rest, E2EE, secure deletion | High |
| Error Handling | 7.2–7.3 | SI (System & Information Integrity) | Panic handler, sanitized errors | Medium |
| Logging & Monitoring | 7.1–7.2 | AU (Audit & Accountability) | Audit middleware, chain hash verification | High |
| Input Validation | 5.1–5.5 | SI (System & Information Integrity) | Path normalization, XML parsing, body limits | High |
| Configuration | 12.1–12.4 | CM (Configuration Management) | Secure defaults, TLS config, CORS | High |
| Communication | 6.2–6.3 | SC (System & Communications Protection) | TLS 1.3, HSTS, secure WebSocket | High |
| File Operations | 5.2–5.3 | MP (Media Protection) | Path traversal prevention, filename sanitization | High |

---

## 1. Authentication (ASVS 2.x, NIST IA)

### ASVS 2.1 — Password Security

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Password hashing | 2.1.1 | IA-5(1) | Argon2 with cost factor 12 (~250ms) | Implemented |
| Password complexity | 2.1.2 | IA-5(1) | Enforced at creation (admin API) | Implemented |
| Password storage | 2.1.3 | IA-5(1) | Argon2 hash, not plaintext; SQLite storage | Implemented |
| Default password | 2.1.4 | IA-5(1) | `default_password_layer` forces change | Implemented |
| Password change | 2.1.5 | IA-5(1) | `/api/auth/change-password` endpoint | Implemented |

**Evidence:** `SECURITY.md` — "Bcrypt cost factor 12, ~250ms, resists GPU brute-force"; `penetration_test_scope.md` — AUTH-19.

### ASVS 2.2 — General Authentication

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Credential storage | 2.2.1 | IA-5(1) | Argon2 hashes in SQLite | Implemented |
| Credential transit | 2.2.2 | IA-5(1) | TLS 1.3 for all transport | Implemented |
| Credential comparison | 2.2.3 | IA-5(1) | Constant-time string comparison | Implemented |
| Brute force protection | 2.2.4 | IA-5(1) | Token bucket rate limiter per IP | Implemented |
| Lockout mechanism | 2.2.5 | IA-5(1) | Per-IP rate limiting (not per-account) | Partial |

**Evidence:** `SECURITY.md` — "Constant-time string comparison for secrets"; `penetration_test_scope.md` — AUTH-06.

### ASVS 2.3 — OTP / MFA

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| TOTP enrollment | 2.3.1 | IA-2(1) | `/api/auth/totp/setup`, `/api/auth/totp/enable` | Implemented |
| TOTP verification | 2.3.2 | IA-2(1) | Counter-based validation, single-use | Implemented |
| TOTP secret storage | 2.3.3 | IA-2(1) | Encrypted storage (age-based encryption) | Implemented |
| TOTP revocation | 2.3.4 | IA-2(1) | `/api/auth/totp/disable` endpoint | Implemented |

**Evidence:** `penetration_test_scope.md` — AUTH-12, AS-SC-002.

### ASVS 2.5 — Credential Storage

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Credential uniqueness | 2.5.1 | IA-5(1) | Unique username constraint | Implemented |
| Credential lifecycle | 2.5.2 | IA-5(1) | Admin-managed user lifecycle | Implemented |

### ASVS 2.6 — Password Quality

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Password strength | 2.6.1 | IA-5(1) | Minimum length enforced | Implemented |
| Password dictionary | 2.6.2 | IA-5(1) | Not implemented | Not Implemented |
| Password history | 2.6.3 | IA-5(1) | Not implemented | Not Implemented |

---

## 2. Authorization (ASVS 4.x, NIST AC)

### ASVS 4.1 — Authorization

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Access control policy | 4.1.1 | AC-2 | Cedar policy engine | Implemented |
| Role-based access | 4.1.2 | AC-2, AC-6 | Admin/user roles via Cedar | Implemented |
| Least privilege | 4.1.3 | AC-6 | Cedar policies enforce minimal access | Implemented |
| Deny by default | 4.1.4 | AC-6 | Cedar default deny | Implemented |
| Administrative access | 4.1.5 | AC-6(1) | Admin middleware on `/api/admin/*` | Implemented |
| Access control review | 4.1.6 | AC-2(7) | Admin user management endpoints | Implemented |

**Evidence:** `SECURITY.md` — "Cedar policy engine"; `penetration_test_scope.md` — AUTH-07, AUTH-08, AUTH-15.

### ASVS 4.2 — Operation Level Access Control

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| File access control | 4.2.1 | AC-3 | Cedar policies on file operations | Implemented |
| Directory access control | 4.2.2 | AC-3 | Path normalization + Cedar | Implemented |
| Share access control | 4.2.3 | AC-3 | Token-based share authorization | Implemented |
| Admin operation control | 4.2.4 | AC-6 | Admin middleware + Cedar | Implemented |
| Guest access control | 4.2.5 | AC-6 | Guest middleware (limited scope) | Implemented |
| Federation access control | 4.2.6 | AC-3 | HTTP Signature validation | Implemented |

**Evidence:** `penetration_test_scope.md` — AUTH-08, AUTH-16, AUTH-18.

### ASVS 4.3 — User Interface Access Control

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| UI-level access control | 4.3.1 | AC-3 | Server-side enforcement (not client-side only) | Implemented |
| UI content filtering | 4.3.2 | AC-3 | File list filtering via Cedar policies | Implemented |

---

## 3. Cryptography (ASVS 6.x, NIST SC)

### ASVS 6.1 — Data Classification

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Data classification | 6.1.1 | SC-16 | File-level encryption, user-scoped access | Implemented |
| Sensitive data identification | 6.1.2 | SC-16 | E2EE for sensitive files | Implemented |

### ASVS 6.2 — Algorithms

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Strong algorithms | 6.2.1 | SC-13 | AES-GCM, X25519, ChaCha20-Poly1305 | Implemented |
| Algorithm randomness | 6.2.2 | SC-13 | CSPRNG for key generation | Implemented |
| Algorithm configuration | 6.2.3 | SC-13 | Rust crypto libraries (ring, aes-gcm) | Implemented |
| Key length | 6.2.4 | SC-13 | AES-256, X25519 (256-bit) | Implemented |
| Algorithm agility | 6.2.5 | SC-13 | age-based encryption (algorithm negotiation) | Implemented |

**Evidence:** `SECURITY.md` — "age-based E2E file encryption (X25519, ChaCha20-Poly1305)"; `penetration_test_scope.md` — CRYPTO-02.

### ASVS 6.3 — Key Management

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Key generation | 6.3.1 | SC-12 | Cryptographically secure key generation | Implemented |
| Key storage | 6.3.2 | SC-12, SC-28 | Encrypted key storage; zeroize on drop | Implemented |
| Key rotation | 6.3.3 | SC-12 | Federation key rotation support | Implemented |
| Key revocation | 6.3.4 | SC-12 | Device revocation endpoints | Implemented |
| Key backup | 6.3.5 | SC-12 | Admin backup/restore functionality | Implemented |
| Key destruction | 6.3.6 | SC-28 | Secure deletion via trash purge | Partial |

**Evidence:** `SECURITY.md` — "age for E2E encryption, modern, audited, passphrase-based"; `penetration_test_scope.md` — CRYPTO-05, CRYPTO-08.

### ASVS 6.4 — Random Number Generation

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Random number generation | 6.4.1 | SC-13 | CSPRNG for tokens, keys, nonces | Implemented |
| Entropy estimation | 6.4.2 | SC-13 | UUID v4 for share tokens (122-bit entropy) | Implemented |
| Entropy source | 6.4.3 | SC-13 | OS CSPRNG (`/dev/urandom`, `getrandom`) | Implemented |

**Evidence:** `SECURITY.md` — "UUID v4 tokens (122-bit entropy), per-token lockout"; `penetration_test_scope.md` — CRYPTO-09.

### ASVS 6.5 — Secrets Management

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Secret storage | 6.5.1 | SC-12 | Environment variables, config files | Implemented |
| Secret rotation | 6.5.2 | SC-12 | Federation secret rotation | Partial |
| Secret access control | 6.5.3 | AC-6 | File permissions on config | Implemented |
| Secret in transit | 6.5.4 | SC-8 | TLS for all secret transmission | Implemented |

### ASVS 6.6 — TLS

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| TLS configuration | 6.6.1 | SC-8 | TLS 1.3 via rustls | Implemented |
| Certificate validation | 6.6.2 | SC-17 | Full certificate chain validation | Implemented |
| HSTS | 6.6.3 | SC-8 | HSTS header (HTTPS only) | Implemented |
| Certificate pinning | 6.6.4 | SC-17 | Not implemented (optional) | Not Implemented |

**Evidence:** `SECURITY.md` — "TLS 1.3 for all transport (rustls)"; `penetration_test_scope.md` — CRYPTO-01.

---

## 4. Data Protection (ASVS 7.x, NIST MP)

### ASVS 7.1 — Data Classification and Protection

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Data at rest encryption | 7.1.1 | MP-5, SC-28 | E2EE via age encryption | Implemented |
| Data in transit encryption | 7.1.2 | SC-8 | TLS 1.3 | Implemented |
| Data retention | 7.1.3 | MP-4, SC-28 | Retention policies (admin API) | Implemented |
| Data disposal | 7.1.4 | MP-6 | Trash purge, secure deletion | Implemented |
| Data backup | 7.1.5 | CP-9 | Admin backup/restore functionality | Implemented |

**Evidence:** `penetration_test_scope.md` — CRYPTO-10, API endpoints for backup/restore.

### ASVS 7.2 — Privacy

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Privacy by design | 7.2.1 | SC-16 | Self-hosted, user-controlled data | Implemented |
| Data minimization | 7.2.2 | SC-16 | Minimal data collection | Implemented |
| GDPR compliance | 7.2.3 | SC-16 | GDPR export/erase endpoints | Implemented |
| Consent management | 7.2.4 | SC-16 | User-managed sharing controls | Implemented |

**Evidence:** `penetration_test_scope.md` — GDPR endpoints (`/api/admin/gdpr`, export, erase).

### ASVS 7.3 — Data Protection

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Sensitive data exposure | 7.3.1 | SC-8 | E2EE, access controls | Implemented |
| Data masking | 7.3.2 | SC-28 | Password masking in logs | Implemented |
| Data integrity | 7.3.3 | SC-16 | File integrity verification | Implemented |
| Data availability | 7.3.4 | CP-9 | Backup, disaster recovery | Implemented |

### ASVS 7.4 — Files and Resources

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| File upload validation | 7.4.1 | SC-28 | Content-Type validation, magic bytes | Implemented |
| File download control | 7.4.2 | AC-3 | Access-controlled download endpoints | Implemented |
| File integrity verification | 7.4.3 | SC-16 | Hash verification for uploads | Implemented |
| Temporary file handling | 7.4.4 | SC-28 | Chunked upload cleanup | Implemented |

---

## 5. Error Handling (ASVS 7.x, NIST SI)

### ASVS 7.2 — Error Handling

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Error handling | 7.2.1 | SI-10 | Panic handler logs to server; generic 500 response | Implemented |
| Error logging | 7.2.2 | AU-3 | Structured error logging with request context | Implemented |
| Error information | 7.2.3 | SI-10 | No stack traces in production responses | Implemented |
| Error recovery | 7.2.4 | SI-10 | Graceful degradation; timeout handling | Implemented |

**Evidence:** `penetration_test_scope.md` — API-06, API-07; `SECURITY.md` — "Panic Handler: Logs request context on 500 errors."

### ASVS 7.3 — Logging

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Security event logging | 7.3.1 | AU-2, AU-3 | Audit middleware logs auth events, admin ops | Implemented |
| Log integrity | 7.3.2 | AU-9 | Chain hash verification for audit logs | Implemented |
| Log protection | 7.3.3 | AU-9 | Append-only audit design; SQLite WAL | Implemented |
| Log retention | 7.3.4 | AU-11 | Configurable log retention | Implemented |
| Log review | 7.3.5 | AU-6 | Admin audit chain endpoint | Implemented |

**Evidence:** `SECURITY.md` — "Chain hash verification (GET /api/admin/audit-chain), append-only design"; `penetration_test_scope.md` — API-11, INFRA-07.

---

## 6. Input Validation (ASVS 5.x, NIST SI)

### ASVS 5.1 — Input Validation

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Input validation | 5.1.1 | SI-10 | Request body validation, Content-Type checks | Implemented |
| Input encoding | 5.1.2 | SI-10 | URL decoding, path normalization | Implemented |
| Input length limits | 5.1.3 | SI-10 | Configurable `max_body_size` | Implemented |
| Input type validation | 5.1.4 | SI-10 | Rust type system enforcement | Implemented |
| Input sanitization | 5.1.5 | SI-10 | Filename sanitization, XML entity prevention | Implemented |

**Evidence:** `penetration_test_scope.md` — INPUT-01 through INPUT-21; `SECURITY.md` — "Path traversal prevention (normalized paths, `..` rejection)."

### ASVS 5.2 — Sanitization and Encoding

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Output encoding | 5.2.1 | SI-10 | HTML escaping in responses | Implemented |
| Input sanitization | 5.2.2 | SI-10 | Filename sanitization, null byte rejection | Implemented |
| Safe APIs | 5.2.3 | SI-10 | Parameterized SQL queries; Rust type safety | Implemented |
| Context-aware encoding | 5.2.4 | SI-10 | Content-Type specific handling | Implemented |

### ASVS 5.3 — Error Handling and Logging

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Error messages | 5.3.1 | SI-10 | Generic error messages in production | Implemented |
| Error logging | 5.3.2 | AU-3 | Structured error logging | Implemented |
| Attack detection | 5.3.3 | SI-4 | Rate limiting, audit logging | Implemented |

### ASVS 5.4 — XML and JSON

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| XML parsing | 5.4.1 | SI-10 | XML parser with entity expansion limits | Implemented |
| XML external entities | 5.4.2 | SI-10 | External entity disabling | Implemented |
| JSON parsing | 5.4.3 | SI-10 | serde-based JSON parsing | Implemented |
| Schema validation | 5.4.4 | SI-10 | Request body schema validation | Implemented |

**Evidence:** `SECURITY.md` — "XML entity expansion prevention in WebDAV"; `penetration_test_scope.md` — INPUT-04, INPUT-05.

### ASVS 5.5 — Deserialization

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Deserialization safety | 5.5.1 | SI-10 | serde with strict type checking | Implemented |
| Untrusted data | 5.5.2 | SI-10 | No `unsafe deserialization` from untrusted sources | Implemented |

---

## 7. Configuration (ASVS 12.x, NIST CM)

### ASVS 12.1 — Secure Configuration

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Secure defaults | 12.1.1 | CM-6 | Secure default configuration; default password enforcement | Implemented |
| Configuration review | 12.1.2 | CM-2 | ferro.toml configuration file | Implemented |
| Configuration validation | 12.1.3 | CM-2 | Config parsing with validation | Implemented |
| Configuration access control | 12.1.4 | AC-3 | Admin-only configuration endpoints | Implemented |
| Configuration documentation | 12.1.5 | CM-6 | Configuration documentation | Implemented |

**Evidence:** `penetration_test_scope.md` — AUTH-19, SC-004.

### ASVS 12.2 — Dependency Management

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Dependency inventory | 12.2.1 | SA-12 | Cargo.lock with dependency tree | Implemented |
| Vulnerability scanning | 12.2.2 | SA-11 | `cargo audit` in CI | Implemented |
| Dependency policy | 12.2.3 | SA-12 | No critical CVEs, maintained dependencies | Implemented |
| Supply chain verification | 12.2.4 | SA-12 | Prefer pure-Rust implementations | Implemented |

**Evidence:** `SECURITY.md` — "Dependency Security" section; `penetration_test_scope.md` — INFRA-03, INFRA-04.

### ASVS 12.3 — Automated Build and Deployment

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Build security | 12.3.1 | SA-11 | CI/CD pipeline with security checks | Implemented |
| Deployment security | 12.3.2 | CM-2 | Docker and bare metal deployment | Implemented |
| Release integrity | 12.3.3 | CM-2 | Signed releases, version tagging | Implemented |

---

## 8. Communication Security (ASVS 6.x, NIST SC)

### ASVS 6.2 — Communications Security

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Transport encryption | 6.2.1 | SC-8 | TLS 1.3 for all transport | Implemented |
| Certificate management | 6.2.2 | SC-17 | rustls certificate validation | Implemented |
| Secure WebSocket | 6.2.3 | SC-8 | WebSocket over TLS (WSS) | Implemented |
| CORS configuration | 6.2.4 | SC-8 | Configurable allowed origins | Implemented |
| HSTS | 6.2.5 | SC-8 | HSTS header (HTTPS only) | Implemented |

**Evidence:** `SECURITY.md` — "Strict-Transport-Security (HSTS)"; `penetration_test_scope.md` — CRYPTO-01.

### ASVS 6.3 — Session Management

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Session token generation | 6.3.1 | AC-3 | UUID v4 for tokens | Implemented |
| Session token storage | 6.3.2 | AC-3 | Secure cookie / bearer token | Implemented |
| Session token expiration | 6.3.3 | AC-3 | Configurable token TTL | Implemented |
| Session token revocation | 6.3.4 | AC-3 | Token revocation endpoints | Implemented |
| Session fixation prevention | 6.3.5 | AC-3 | State parameter in OIDC flow | Implemented |

---

## 9. File Operations Security

### ASVS 5.2 — File Operations

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Path traversal prevention | 5.2.1 | SC-4 | Path normalization, `..` rejection | Implemented |
| Filename sanitization | 5.2.2 | SC-4 | Null byte, special char stripping | Implemented |
| File upload validation | 5.2.3 | SC-4 | Content-Type, magic bytes verification | Implemented |
| File size limits | 5.2.4 | SC-4 | Configurable `max_body_size` | Implemented |
| Chunked upload security | 5.2.5 | SC-4 | Chunk reassembly validation | Implemented |
| File overwrite protection | 5.2.6 | SC-4 | Atomic file operations | Implemented |
| Symlink protection | 5.2.7 | SC-4 | Canonical path resolution | Implemented |

**Evidence:** `penetration_test_scope.md` — INPUT-02, INPUT-13, INPUT-18.

---

## 10. WASM Sandbox Security

### ASVS 6.1 — WASM Isolation

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Module validation | 6.1.1 | SC-39 | WASM magic bytes verification | Implemented |
| Resource limits | 6.1.2 | SC-39 | Fuel (1B units), memory (64MB), timeout (30s) | Implemented |
| I/O isolation | 6.1.3 | SC-39 | No network, no filesystem access | Implemented |
| Memory isolation | 6.1.4 | SC-39 | Linear memory isolation, no shared memory | Implemented |
| Execution monitoring | 6.1.5 | SC-39 | Resource limit enforcement; timeout handling | Implemented |

**Evidence:** `SECURITY.md` — "WASM Worker Security Model" section; `penetration_test_scope.md` — INFRA-05.

---

## 11. Federation Security

### ASVS 6.2 — Federation

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Activity signing | 6.2.1 | SC-8 | HTTP Signatures (HMAC-SHA256) | Implemented |
| Signature verification | 6.2.2 | SC-8 | Actor keyId validation | Implemented |
| Replay prevention | 6.2.3 | SC-8 | Signature timestamp validation | Implemented |
| Key rotation | 6.2.4 | SC-12 | Federation key rotation | Implemented |
| Discovery security | 6.2.5 | SC-8 | WebFinger/NodeInfo read-only | Implemented |

**Evidence:** `SECURITY.md` — "Federation Security Model" section; `penetration_test_scope.md` — AUTH-18.

---

## 12. Monitoring and Incident Response

### ASVS 7.1 — Monitoring

| Control | ASVS Requirement | NIST Control | Ferro Implementation | Status |
|---------|------------------|--------------|---------------------|--------|
| Security event monitoring | 7.1.1 | AU-6 | Audit middleware; request logging | Implemented |
| Anomaly detection | 7.1.2 | AU-6 | Rate limit metrics; auth failure tracking | Implemented |
| Alerting | 7.1.3 | AU-6 | Webhook-based alerting (admin API) | Implemented |
| Incident response | 7.1.4 | IR-1 | Incident response documentation | Implemented |

**Evidence:** `docs/incident_response/` directory; `penetration_test_scope.md` — API-11.

---

## Coverage Gaps and Recommendations

| Gap | Recommendation | Priority |
|-----|----------------|----------|
| Password dictionary check | Implement HaveIBeenPwned API or local dictionary | Medium |
| Password history | Store password hash history to prevent reuse | Medium |
| Certificate pinning | Optional feature for high-security deployments | Low |
| WORM policy enforcement | Verify WORM policies cannot be bypassed | High |
| Content-Type magic bytes | Extend magic bytes verification to all upload types | Medium |
| Federation replay window | Define and document replay window duration | Medium |
| Log retention automation | Implement automatic log rotation/cleanup | Medium |

---

## References

| Standard | Reference | Version |
|----------|-----------|---------|
| OWASP ASVS | https://owasp.org/www-project-application-security-verification-standard/ | v4.0.3 |
| NIST SP 800-53 | https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final | Rev. 5 |
| OWASP Top 10 | https://owasp.org/www-project-top-ten/ | 2021 |
| CWE/SANS Top 25 | https://cwe.mitre.org/top25/ | 2024 |
| NIST SP 800-57 | https://csrc.nist.gov/publications/detail/sp/800-57-part-1/rev-5/final | Rev. 5 |

---

*Document references: `penetration_test_scope.md`, `SECURITY.md`, OWASP ASVS v4.0, NIST SP 800-53 Rev. 5.*
