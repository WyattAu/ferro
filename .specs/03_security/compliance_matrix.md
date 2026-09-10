# Compliance Matrix — Ferro Security Controls

**Document ID:** FERRO-SEC-CM-001
**Date:** 2026-04-18
**Status:** Draft
**Traceability:** Maps security controls to OWASP Top 10 (2021), NIST SP 800-53 Rev 5, ISO/IEC 27001:2022, GDPR

---

## Security Control Mapping

### Authentication & Identity

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| OIDC Authentication (Authorization Code + PKCE) | A07:2021 | IA-2, IA-3, IA-5 | A.9.2, A.9.4 | Art. 32 | REQ-AUTH-001 | Planned |
| JWKS Signature Verification | A07:2021 | IA-2, IA-5, SC-13 | A.9.2 | Art. 32 | REQ-AUTH-001 | Planned |
| Algorithm Allowlist (no `none`, no HS256) | A07:2021 | IA-5, SC-13 | A.9.2 | Art. 32 | REQ-AUTH-001 | Planned |
| Session Management (idle timeout, absolute lifetime) | A07:2021 | SC-23, SA-10, AC-12 | A.9.4 | Art. 32 | REQ-AUTH-004 | Planned |
| Refresh Token Rotation | A07:2021 | IA-5 | A.9.4 | Art. 32 | REQ-AUTH-004 | Planned |
| Session Revocation (immediate logout) | A07:2021 | AC-12, SC-23 | A.9.4 | Art. 32 | REQ-AUTH-004 | Planned |
| MFA Enforcement | A07:2021 | IA-2(1), IA-3 | A.9.4 | Art. 32 | TH-ELEVATE-002 | Planned |
| Nonce Validation (replay prevention) | A07:2021 | IA-2, IA-5 | A.9.4 | Art. 32 | REQ-AUTH-001 | Planned |

### Authorization & Access Control

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| Cedar ABAC Policy Engine | A01:2021 | AC-3, AC-4, AC-6 | A.9.1 | Art. 25 | REQ-AUTH-002, REQ-AUTH-003 | Planned |
| Deny-by-Default Semantics | A01:2021 | AC-3, AC-6 | A.9.1 | Art. 25 | REQ-AUTH-002 | Planned |
| Forbid-Overrides-Permit | A01:2021 | AC-3 | A.9.1 | Art. 25 | REQ-AUTH-002 | Planned |
| Policy Schema Validation | A01:2021 | AC-3, AC-4 | A.9.1 | Art. 25 | REQ-AUTH-002 | Planned |
| Policy Hot-Reload (atomic swap) | A01:2021 | AC-3 | A.9.1 | Art. 25 | REQ-AUTH-002 | Planned |
| Per-Field Metadata Authorization | A01:2021 | AC-3, AC-4 | A.9.1, A.8.3 | Art. 25 | REQ-WEBDAV-003 | Planned |
| Admin Role Separation | A01:2021 | AC-2, AC-6 | A.8.2 | Art. 32 | TH-ELEVATE-002 | Planned |

### Audit & Logging

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| Immutable Append-Only Audit Log | A09:2021 | AU-2, AU-3, AU-4, AU-9 | A.12.4 | Art. 30 | REQ-ENT-001 | Planned |
| Tamper-Evident Hash Chaining | A09:2021 | AU-9 | A.12.4, A.5.33 | Art. 30 | REQ-ENT-001 | Planned |
| gRPC Audit Log Streaming | A09:2021 | AU-6, AU-9, SI-4 | A.12.4 | Art. 30 | REQ-ENT-002 | Planned |
| Authorization Decision Logging | A09:2021 | AU-2, AU-3 | A.12.4 | Art. 30 | REQ-ENT-001 | Planned |
| Policy Change Audit Trail | A09:2021 | AU-2, AU-3, AC-2 | A.12.4 | Art. 30 | TH-REPUD-002 | Planned |
| Token Validation Failure Logging | A09:2021 | AU-2, AU-6, SI-4 | A.12.4 | Art. 32 | TH-SPOOF-001 | Planned |

### Data Integrity & Cryptography

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| SHA-256 Content Integrity (CAS) | A08:2021 | SC-8, SC-13 | A.8.3, A.12.2 | Art. 5(1)(f) | REQ-STOR-002 | Planned |
| SHA-256 Verification on Read | A08:2021 | SC-13 | A.8.3, A.12.2 | Art. 5(1)(f) | REQ-STOR-002 | Planned |
| Atomic Write Operations | A08:2021 | SC-13 | A.8.8 | Art. 5(1)(f) | REQ-STOR-005 | Planned |
| Pre-signed URL HMAC Signing | A10:2021 | SC-12 | A.5.14 | Art. 32 | REQ-STOR-004 | Planned |
| TLS 1.3 Enforcement | A02:2021 | SC-8, SC-13 | A.10 | Art. 32 | REQ-AUTH-001 | Planned |
| Server-Side Encryption (SSE) | A02:2021 | SC-8, SC-28 | A.8.24 | Art. 32 | TH-TAMPER-002 | Planned |

### Input Validation & Injection Prevention

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| Path Traversal Prevention | A03:2021 | SI-10 | A.8.25 | Art. 32 | REQ-STOR-001, DC-PATH-003 | Planned |
| Streaming XML Parser (no DTD/entities) | A03:2021 | SI-10 | A.8.25 | Art. 32 | REQ-WEBDAV-006 | Planned |
| XML Entity Expansion Limit | A03:2021 | SI-10 | A.8.25 | Art. 32 | REQ-WEBDAV-006 | Planned |
| SQL Injection Prevention (SQLx compile-time) | A03:2021 | SI-10, SI-16 | A.8.24 | Art. 25 | BP-STORAGE-001 | Planned |
| Request Body Size Limit (1 MiB) | A03:2021 | SI-10 | A.8.25 | Art. 32 | DC-XML-002 | Planned |
| Policy Input Validation (Cedar schema) | A03:2021 | SI-10 | A.8.25 | Art. 25 | REQ-AUTH-002 | Planned |

### Denial of Service Prevention

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| Rate Limiting (per-IP) | A04:2021 | SC-5, SC-6 | A.12.4 | Art. 32 | DC-SEC-002 | Planned |
| Upload Size Limits (5 TB) | A04:2021 | SC-5 | A.12.4 | Art. 32 | DC-STORAGE-001 | Planned |
| PROPFIND Depth Limit (100) | A04:2021 | SC-5 | A.12.4 | Art. 32 | DC-PATH-005 | Planned |
| Lock Timeout Enforcement | A04:2021 | SC-5 | A.12.4 | — | DC-LOCK-001..003 | Planned |
| Max Locks Per Principal (1000) | A04:2021 | SC-5 | A.12.4 | — | DC-LOCK-005 | Planned |
| WASM Fuel Metering | A04:2021 | SC-7 | A.15.1 | Art. 32 | REQ-WASM-004 | Planned |
| WASM Memory Limits | A04:2021 | SC-7 | A.15.1 | Art. 32 | REQ-WASM-004 | Planned |
| WASM Time Limits | A04:2021 | SC-7 | A.15.1 | Art. 32 | REQ-WASM-004 | Planned |
| Connection Pool Limits | A04:2021 | SC-5, SC-6 | A.12.4 | Art. 32 | TH-DOS-005 | Planned |

### Session & Transmission Security

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| Secure Cookie Flags (HttpOnly, Secure, SameSite) | A07:2021 | SC-23 | A.9.4 | Art. 32 | REQ-AUTH-004 | Planned |
| Short Token Lifetime (600s max) | A07:2021 | SC-23, AC-12 | A.9.4 | Art. 32 | DC-AUTH-TOKEN-TTL-001 | Planned |
| Pre-signed URL Short TTL (60s default, 3600s max) | A10:2021 | AC-17 | A.5.14 | Art. 32 | DC-PRESIGN-001 | Planned |
| HTTPS-Only for All Endpoints | A02:2021 | SC-8, SC-13 | A.10 | Art. 32 | REQ-AUTH-001 | Planned |

### WASM Plugin Security

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| WASI Capability-Based Security | A04:2021 | SC-7, AC-3 | A.15.1 | Art. 32 | REQ-WASM-003 | Planned |
| WASM Sandbox Isolation | A04:2021 | SC-7 | A.15.1 | Art. 32 | REQ-WASM-003 | Planned |
| No Raw Syscalls from WASM | A04:2021 | SC-7 | A.15.1 | Art. 32 | REQ-WASM-003 | Planned |
| Plugin Termination Isolation | A04:2021 | SC-7 | A.15.1 | Art. 32 | REQ-WASM-003, REQ-WASM-004 | Planned |

### Data Protection (GDPR-Specific)

| Control | GDPR Article | OWASP | NIST 800-53 | ISO 27001 | Requirement | Status |
|---------|-------------|-------|-------------|-----------|-------------|--------|
| Data Minimization (metadata) | Art. 5(1)(c) | — | — | A.5.10 | REQ-STOR-002, TH-DISCLOSE-003 | Planned |
| Right to Erasure (CAS ref counting) | Art. 17 | — | — | A.5.33 | REQ-STOR-003, REQ-ENT-004 | Planned |
| Right to Data Portability | Art. 20 | — | — | A.5.34 | REQ-ENT-004 | Planned |
| Processing Records (audit log) | Art. 30 | A09:2021 | AU-2, AU-3 | A.12.4 | REQ-ENT-001 | Planned |
| Security of Processing | Art. 32 | A02, A05 | SC-8, SC-13 | A.8.3, A.10 | Multiple | Planned |
| Privacy by Design | Art. 25 | A01:2021 | AC-3 | A.8.3 | REQ-AUTH-003, TH-DISCLOSE-003 | Planned |
| Breach Notification | Art. 33 | A09:2021 | AU-6, SI-4 | A.12.4 | REQ-ENT-002 | Planned |
| Cookie/Tracking Consent | CCPA 1798.120 | A05:2021 | — | A.5.10 | REQ-WEB-003 | Planned |

### Dependency & Configuration Security

| Control | OWASP | NIST 800-53 | ISO 27001 | GDPR | Requirement | Status |
|---------|-------|-------------|-----------|------|-------------|--------|
| Dependency Scanning (cargo-audit) | A06:2021 | RA-5, SI-2 | A.8.8 | Art. 32 | applicable_standards §2.12 | Planned |
| Dependency Pinning | A06:2021 | SA-10, SI-2 | A.8.8 | — | applicable_standards §2.12 | Planned |
| No Default Credentials | A05:2021 | AC-2, IA-5 | A.8.5 | Art. 32 | applicable_standards §2.12 | Planned |
| Minimal CORS Configuration | A05:2021 | AC-3, SC-7 | A.8.25 | Art. 32 | applicable_standards §2.12 | Planned |
| Secure HTTP Headers (CSP, HSTS, X-Frame-Options) | A05:2021 | SC-8, SC-23 | A.10 | Art. 32 | applicable_standards §2.12 | Planned |
| Error Response Sanitization | A05:2021 | SI-11 | A.8.12 | Art. 32 | DC-AUTH-ERROR-001 | Planned |

---

## Coverage Summary by Standard

### OWASP Top 10 (2021) Coverage

| Category | Controls Addressed | Status |
|----------|-------------------|--------|
| A01: Broken Access Control | Cedar ABAC, deny-by-default, path traversal, per-field auth | Planned |
| A02: Cryptographic Failures | TLS 1.3, SHA-256, SSE, no weak crypto | Planned |
| A03: Injection | SQLx compile-time, XML streaming, path validation, body limits | Planned |
| A04: Insecure Design | STRIDE threat model per subsystem | Planned |
| A05: Security Misconfiguration | No default creds, minimal CORS, secure headers, error sanitization | Planned |
| A06: Vulnerable Components | cargo-audit, dependency pinning | Planned |
| A07: Authentication Failures | OIDC+PKCE, MFA, session management, algorithm allowlist | Planned |
| A08: Data Integrity Failures | SHA-256 CAS, atomic writes, audit hash chain | Planned |
| A09: Logging Failures | Immutable audit log, hash chain, gRPC streaming | Planned |
| A10: SSRF | Pre-signed URL scoping, WASM HTTP capability control | Planned |

### NIST SP 800-53 Coverage

| Control Family | Controls Covered | Notable Gaps |
|----------------|-----------------|--------------|
| AC (Access Control) | AC-2, AC-3, AC-4, AC-6, AC-7, AC-12, AC-17 | AC-10 (concurrent session control — partial via session store) |
| AU (Audit) | AU-1, AU-2, AU-3, AU-4, AU-5, AU-6, AU-9 | — |
| IA (Identification) | IA-2, IA-3, IA-5 | IA-8 (non-organizational users — delegated to OIDC provider) |
| SC (System & Communications) | SC-5, SC-6, SC-7, SC-8, SC-12, SC-13, SC-23, SC-28 | SC-39 (process isolation — WASM sandbox partial) |
| SI (System Integrity) | SI-2, SI-4, SI-7, SI-10, SI-11, SI-16 | — |
| RA (Risk Assessment) | RA-5 | — |
| SA (System & Services Acquisition) | SA-10 | — |

### ISO 27001:2022 Coverage

| Clause | Controls Addressed | Notable Gaps |
|--------|-------------------|--------------|
| A.5 (Organizational) | A.5.10, A.5.12, A.5.14, A.5.33 | A.5.1 (policy document — operational) |
| A.8 (People) | A.8.1, A.8.2, A.8.3, A.8.5, A.8.8, A.8.12, A.8.24, A.8.25 | A.8.15 (incident response — procedural) |
| A.9 (Technical) | A.9.1, A.9.2, A.9.4 | — |
| A.10 (Communications) | A.10 | — |
| A.12 (Operations) | A.12.2, A.12.4 | A.12.6 (capacity management — partial via rate limiting) |
| A.15 (Supplier) | A.15.1 | — |

### GDPR Coverage

| Article | Controls Addressed | Notable Gaps |
|---------|-------------------|--------------|
| Art. 5 (Principles) | Data minimization, integrity, confidentiality | Art. 5(1)(b) (purpose limitation — procedural) |
| Art. 15 (Right of access) | User data export API | Planned (Phase 5) |
| Art. 17 (Right to erasure) | CAS ref counting + GC | Planned |
| Art. 20 (Data portability) | Standard export format | Planned (Phase 5) |
| Art. 25 (Privacy by design) | Cedar ABAC, per-field authorization | Planned |
| Art. 30 (Processing records) | Audit log as processing record | Planned |
| Art. 32 (Security of processing) | TLS, encryption, access controls | Planned |
| Art. 33 (Breach notification) | Audit log alerts, incident response | Planned (Phase 2) |

---

## Compliance Gaps Identified

| Gap ID | Standard | Control | Description | Priority | Remediation Phase |
|--------|----------|---------|-------------|----------|-------------------|
| GAP-001 | NIST 800-53 | AC-10 | Concurrent session control (limit sessions per user) not explicitly implemented | Medium | Phase 2 (COMP-SESSION-003) |
| GAP-002 | NIST 800-53 | SC-39 | Process isolation for WASM plugins relies on Wasmtime sandbox only; no OS-level isolation | Low | Phase 5 (future: containerized plugins) |
| GAP-003 | ISO 27001 | A.5.1 | Information security policies document not yet created | Medium | Pre-deployment |
| GAP-004 | ISO 27001 | A.8.15 | Formal incident response procedure not yet defined | Medium | Pre-deployment |
| GAP-005 | GDPR | Art. 15 | User data access/export API not yet implemented | High | Phase 5 (REQ-SEARCH area) |
| GAP-006 | GDPR | Art. 20 | Data portability export format not yet defined | Medium | Phase 5 |
| GAP-007 | GDPR | Art. 33 | Automated breach notification pipeline not yet implemented | High | Phase 2 (REQ-ENT-002 + SIEM integration) |
| GAP-008 | OWASP A05 | Content Security Policy (CSP) headers not yet defined for web frontend | Medium | Phase 3 (REQ-WEB-001) |
| GAP-009 | OWASP A06 | Automated SBOM generation and supply chain provenance not yet configured | Medium | CI/CD setup |
| GAP-010 | NIST 800-53 | IA-8 | Non-organizational user (external collaborator) identity management not addressed | Low | Future consideration |
| GAP-011 | ISO 27001 | A.12.6 | Capacity management monitoring (storage, bandwidth) not automated | Low | Phase 5 (metrics integration) |
| GAP-012 | GDPR | CCPA 1798.120 | Cookie/tracking consent mechanism not yet implemented for web frontend | Medium | Phase 3 (REQ-WEB-003) |

---

## Implementation Priority

| Priority | Gaps | Target Phase |
|----------|------|-------------|
| **P0 (Before any release)** | GAP-003, GAP-004 | Pre-deployment |
| **P1 (Phase 2 release)** | GAP-001, GAP-007 | Phase 2 |
| **P2 (Phase 3 release)** | GAP-008, GAP-012 | Phase 3 |
| **P3 (Phase 5 release)** | GAP-005, GAP-006, GAP-009, GAP-011 | Phase 5 |
| **P4 (Future)** | GAP-002, GAP-010 | Post-launch |
