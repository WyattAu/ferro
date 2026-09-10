# Ferro Applicable Standards — Multi-Standard Compliance Matrix

**Document ID:** FERRO-ST-001
**Date:** 2026-04-18
**Status:** Draft

---

## 1. Compliance Matrix Overview

| Standard | Type | Priority | Scope |
|----------|------|----------|-------|
| ISO/IEC 27001 | Mandatory | P0 | Whole system |
| ISO/IEC 27034 | Mandatory | P1 | Web server, frontend |
| NIST SP 800-53 | Mandatory | P0 | Auth, authorization, audit |
| RFC 4918 (WebDAV) | Mandatory | P0 | Interface layer |
| RFC 3253 (WebDAV Versioning/Locking) | Mandatory | P0 | Office integration |
| RFC 3744 (WebDAV ACL) | Mandatory | P1 | Authorization layer |
| WOPI Specification | Mandatory | P1 | Collaborative editing |
| OpenID Connect Core 1.0 | Mandatory | P0 | Authentication |
| Cedar Specification | Mandatory | P0 | Authorization engine |
| WASM/WASI Specification | Mandatory | P1 | Plugin system |
| GDPR / CCPA | Mandatory | P0 | Data protection |
| OWASP Top 10 | Mandatory | P0 | Web application security |
| IEC 62443 | Recommended | P2 | Industrial/specialized use |

---

## 2. Detailed Standard Mapping

### 2.1 ISO/IEC 27001 — Information Security Management

**Applicability:** Whole system
**Priority:** P0 (Mandatory)

| Clause | Requirement | Subsystem | Verification |
|--------|------------|-----------|-------------|
| A.5.1 | Information security policies | All | Policy document review |
| A.5.9 | Inventory of information and associated assets | Core Server, Metadata | Asset registry; automated dependency scanning (cargo-audit) |
| A.5.10 | Acceptable use of information | Web Frontend, Desktop | Terms of service enforcement in UI |
| A.5.12 | Classification of information | Core Server | File tagging/metadata system via Cedar attributes |
| A.5.14 | Information transfer | Core Server, Interface Layer | TLS enforcement; pre-signed URL audit logging |
| A.5.33 | Protection of records | Enterprise (Audit Log) | Immutable append-only log verification |
| A.6.1 | Internal organizational roles | Identity & Auth | Cedar principal hierarchy |
| A.7.2 | Physical and environmental security | N/A (software-only) | Documented deployment requirements |
| A.8.1 | User endpoint devices | Desktop (Tauri) | Credential storage via OS keychain; auto-update mechanism |
| A.8.2 | Privileged access rights | Identity & Auth | Role-based Cedar policies; admin audit trail |
| A.8.3 | Information access restriction | Identity & Auth | Cedar policy enforcement on every API call |
| A.8.5 | Secure authentication | Identity & Auth | OIDC with MFA enforcement; session management |
| A.8.8 | Management of technical vulnerabilities | Core Server | Automated dependency updates; cargo-audit in CI |
| A.8.12 | Data leakage prevention | Enterprise | Audit log monitoring; API rate limiting |
| A.9.1 | Access control | Identity & Auth | Cedar ABAC policies on all endpoints |
| A.9.2 | Identity management | Identity & Auth | OIDC federation; SCIM provisioning (future) |
| A.9.4 | System and access control monitoring | Enterprise | gRPC audit log streaming; alerting integration |
| A.12.4 | Logging and monitoring | Enterprise | Structured JSON audit log; tamper-evident (hash chain) |
| A.14.1 | Information security in development | All | Secure SDLC; code review; cargo-clippy; dependency pinning |

---

### 2.2 ISO/IEC 27034 — Application Security

**Applicability:** Web server, web frontend, API
**Priority:** P1 (Mandatory)

| Clause | Requirement | Subsystem | Verification |
|--------|------------|-----------|-------------|
| 6.1.2 | Application security roles | All | Defined security responsibilities per crate |
| 7.1 | Normative application security controls | Server, Frontend | Input validation; output encoding; CORS configuration |
| 7.2 | Application security validation | All | Automated security testing in CI (cargo-audit, SAST) |
| 8.1 | Application security requirements | All | Threat model per subsystem (STRIDE) |
| 8.3 | Application security architecture | Server | Defense-in-depth; least-privilege service accounts |

---

### 2.3 NIST SP 800-53 — Security and Privacy Controls

**Applicability:** Authentication, authorization, audit logging
**Priority:** P0 (Mandatory)

| Control | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| AC-1 | Access control policy | Identity & Auth | Cedar policy documentation |
| AC-2 | Account management | Identity & Auth | OIDC user provisioning/deprovisioning |
| AC-3 | Access enforcement | Identity & Auth | Cedar evaluation on every request; deny-by-default |
| AC-4 | Information flow enforcement | Identity & Auth | Cedar attribute-based flow controls |
| AC-6 | Least privilege | Identity & Auth | Minimal Cedar permissions per principal |
| AC-7 | Unsuccessful logon attempts | Identity & Auth | OIDC rate limiting; account lockout via IdP |
| AC-17 | Remote access | Interface Layer | TLS-only; pre-signed URL scoping |
| AU-1 | Audit and accountability policy | Enterprise | Audit log policy documentation |
| AU-2 | Audit events | Enterprise | Log all CRUD operations; WebDAV/WOPI/API requests |
| AU-3 | Content of audit records | Enterprise | Structured log: timestamp, principal, action, resource, result |
| AU-4 | Audit log storage | Enterprise | Append-only; tamper-evident (hash chain); retention policy |
| AU-5 | Response to audit processing failures | Enterprise | Alert on log write failures; fail-closed for audit-critical ops |
| AU-6 | Audit review/analysis/reporting | Enterprise | gRPC streaming to SIEM; queryable log API |
| AU-9 | Protection of audit information | Enterprise | Write-only log service; separate from main DB |
| IA-2 | User identification and authentication | Identity & Auth | OIDC with PKCE; token validation |
| IA-5 | Authenticator management | Identity & Auth | OIDC token lifecycle; refresh rotation |
| SC-8 | Transmission confidentiality | All | TLS 1.3 enforcement |
| SC-12 | Cryptographic key management | Core Server | Pre-signed URL key rotation; OIDC key verification (JWKS) |
| SC-13 | Cryptographic protection | Core Server | SHA-256 for CAS; AES-256-GCM for encryption at rest (future) |
| SI-4 | System monitoring | Enterprise | Health checks; metrics export (Prometheus) |

---

### 2.4 RFC 4918 — WebDAV (HTTP Extensions for Distributed Authoring)

**Applicability:** Interface layer — core protocol
**Priority:** P0 (Mandatory)

| Section | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| 8.1 | PROPFIND method | Interface Layer | Unit tests; rclone compatibility test |
| 8.2 | PROPPATCH method | Interface Layer | Unit tests; metadata update verification |
| 8.3 | MKCOL method | Interface Layer | Directory creation tests |
| 8.4 | GET/HEAD/POST | Interface Layer | Standard HTTP compliance |
| 8.5 | DELETE method | Interface Layer | Recursive delete; CAS reference counting |
| 8.6 | PUT method | Interface Layer | File upload; chunked transfer; CAS integration |
| 8.7 | COPY method | Interface Layer | Server-side copy; backend-native copy where available |
| 8.8 | MOVE method | Interface Layer | Atomic rename; metadata update |
| 8.9 | LOCK method | Interface Layer | Exclusive/shared locks; lock token management |
| 8.10 | UNLOCK method | Interface Layer | Lock release; stale lock cleanup |
| 9 | XML request/response bodies | Interface Layer | Schema validation; encoding tests (UTF-8) |
| 10 | Status codes (207, 423, 424, 507) | Interface Layer | Correct multi-status responses; lock error codes |
| 11 | XML property model | Interface Layer | DAV: namespace compliance |
| 13 | Conditional requests (If header) | Interface Layer | ETag support; lock token matching |
| 14 | Compliance classes | Interface Layer | Class 1 (basic), Class 2 (locking), Class 3 (ACL) |

---

### 2.5 RFC 3253 — WebDAV Versioning Extensions

**Applicability:** Office integration, WOPI prerequisites
**Priority:** P0 (Mandatory)

| Section | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| 3 | REPORT method | Interface Layer | DeltaV reports for version history |
| 4 | VERSION-CONTROL | Interface Layer | Version-controlled resource creation |
| 6 | CHECKOUT/CHECKIN | Interface Layer | Locking semantics for Office |
| 7 | UNCHECKOUT | Interface Layer | Revert on cancel |
| 8 | UPDATE/MERGE | Interface Layer | Version merging |
| 12 | LOCK and versioning interaction | Interface Layer | Office edit session isolation |
| 18 | Auto-versioning | Interface Layer | Automatic version creation on PUT |

---

### 2.6 RFC 3744 — WebDAV Access Control Protocol

**Applicability:** Authorization layer
**Priority:** P1 (Mandatory)

| Section | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| 3 | ACL method | Interface Layer | ACL read/write |
| 4 | ACE (Access Control Entry) model | Identity & Auth | Cedar <-> ACL translation layer |
| 5 | Principal resources | Identity & Auth | OIDC principal mapping |
| 6 | Privilege model | Identity & Auth | Read/write/admin privilege definitions |
| 7 | Inheritance | Identity & Auth | Directory-level policy inheritance |
| 8 | Aggregate privileges | Identity & Auth | Composite Cedar actions |

---

### 2.7 WOPI Specification (Microsoft)

**Applicability:** Collaborative editing integration
**Priority:** P1 (Mandatory)

| Clause | Requirement | Subsystem | Verification |
|--------|------------|-----------|-------------|
| 3.1 | WOPI endpoints (CheckFileInfo, GetFile, PutFile, Lock/Unlock) | Interface Layer | Endpoint implementation; WOPI client test |
| 3.2 | CheckFileInfo response schema | Interface Layer | Schema validation against spec |
| 3.3 | File locking for co-authoring | Interface Layer | Concurrent edit lock management |
| 3.4 | WOPI host authentication | Identity & Auth | Token validation; access token generation |
| 3.5 | WOPI discovery | Interface Layer | Discovery XML endpoint |
| 6 | File conversion (GetFile with conversion) | Interface Layer | Format conversion endpoints |

---

### 2.8 OpenID Connect Core 1.0

**Applicability:** Authentication subsystem
**Priority:** P0 (Mandatory)

| Section | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| 3 | Authentication request | Identity & Auth | Authorization code flow with PKCE |
| 3.1.2.1 | Authentication Error Response | Identity & Auth | Error handling tests |
| 3.1.3.6 | Nonce validation | Identity & Auth | Replay attack prevention |
| 5 | Token request/response | Identity & Auth | Code exchange; token validation |
| 6 | UserInfo endpoint | Identity & Auth | Claims extraction for Cedar attributes |
| 7 | ID Token validation | Identity & Auth | Signature verification (RS256/ES256); clock skew handling |
| 9 | RP-Initiated Logout | Identity & Auth | Session cleanup |
| 12 | PKCE (Proof Key for Code Exchange) | Identity & Auth | S256 challenge method mandatory |
| 15 | Keys management (JWKS) | Identity & Auth | JWKS rotation; key caching |
| 16 | Session management | Identity & Auth | Session timeout; refresh token rotation |

---

### 2.9 Cedar Policy Language Specification

**Applicability:** Authorization engine
**Priority:** P0 (Mandatory)

| Section | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| 1 | Policy syntax and structure | Identity & Auth | Policy parser tests |
| 2 | Authorization model (principal, action, resource) | Identity & Auth | Entity hierarchy definition |
| 3 | Policy evaluation semantics | Identity & Auth | Correctness tests against Cedar test suite |
| 4 | Attribute access and slot expressions | Identity & Auth | Attribute resolution from OIDC claims |
| 5 | Partial evaluation / validation | Identity & Auth | Policy validation on upload (catch errors early) |
| 6 | Cedar built-in functions | Identity & Auth | Custom function extensions |
| 7 | Cedar standard hierarchy | Identity & Auth | Principal/resource type definitions |

---

### 2.10 WASM/WASI Specification

**Applicability:** Plugin system (Active FS)
**Priority:** P1 (Mandatory)

| Section | Requirement | Subsystem | Verification |
|---------|------------|-----------|-------------|
| WASI Preview 2 | Capability-based security | Active FS | WASI capability restrictions enforced |
| WASI: filesystem | File system access | Active FS | Scoped to authorized files only |
| WASI: clocks | Time access | Active FS | Monotonic clock; no wall-clock leakage |
| WASI: random | Random number generation | Active FS | Deterministic seeding for reproducibility |
| WASI: io | I/O streams | Active FS | Stdin/stdout/stderr capture; log forwarding |
| WASI: http | HTTP client (preview) | Active FS | Outbound webhook capability (opt-in) |
| Resource limits | Memory, CPU, time | Active FS | Fuel metering; memory limits; timeout enforcement |

---

### 2.11 GDPR / CCPA — Data Protection

**Applicability:** Data handling, audit logs, dedup metadata
**Priority:** P0 (Mandatory)

| Article/Section | Requirement | Subsystem | Verification |
|----------------|------------|-----------|-------------|
| GDPR Art. 5 | Data minimization | All | Collect only necessary metadata; document data flows |
| GDPR Art. 15 | Right of access (data portability) | Core Server, Interface | User data export API |
| GDPR Art. 17 | Right to erasure | Core Server, Metadata | CAS file deletion with ref counting; metadata purge |
| GDPR Art. 20 | Data portability | Core Server, Interface | Standard export format (ZIP + metadata JSON) |
| GDPR Art. 25 | Privacy by design | All | Privacy impact assessment per subsystem |
| GDPR Art. 30 | Records of processing | Enterprise | Audit log as processing record |
| GDPR Art. 32 | Security of processing | All | Encryption at rest/transit; access controls |
| GDPR Art. 33 | Breach notification | Enterprise | Incident response procedure; audit log alerts |
| CCPA 1798.100 | Right to know | Enterprise | User data access API |
| CCPA 1798.105 | Right to delete | Core Server | Same as GDPR Art. 17 |
| CCPA 1798.120 | Right to opt-out | Web Frontend | Cookie/tracking consent mechanism |

**Dedup-specific consideration:** CAS dedup means deleting a user's copy may not delete the underlying bytes if other users reference them. The metadata layer must track per-user ownership to satisfy erasure requests.

---

### 2.12 OWASP Top 10 (2021)

**Applicability:** Web application security
**Priority:** P0 (Mandatory)

| Category | Requirement | Subsystem | Verification |
|----------|------------|-----------|-------------|
| A01: Broken Access Control | Every endpoint must enforce Cedar policies | All | Authorization test suite; deny-by-default |
| A02: Cryptographic Failures | TLS 1.3; SHA-256 CAS; no weak crypto | Core Server, Interface | TLS configuration audit; crypto audit |
| A03: Injection | Parameterized queries (SQLx); XML entity expansion prevention | Server, Interface | SQLx compile-time query checking; XML parser hardening |
| A04: Insecure Design | Threat modeling per subsystem | All | STRIDE threat models |
| A05: Security Misconfiguration | No default credentials; minimal CORS; secure headers | Server, Frontend | Configuration audit; header check (CSP, HSTS) |
| A06: Vulnerable Components | cargo-audit; pinned dependencies | All | CI dependency scanning |
| A07: Auth Failures | OIDC with PKCE; MFA; session management | Identity & Auth | Auth test suite |
| A08: Data Integrity Failures | SBOM; CI provenance; audit log integrity | All | cargo-lock parsing; hash chain verification |
| A09: Logging Failures | Structured audit logs for all auth decisions | Enterprise | Log coverage tests |
| A10: SSRF | Restrict pre-signed URL targets; WASM outbound HTTP | Core Server, Active FS | URL allowlisting; WASI HTTP capability control |

---

### 2.13 IEC 62443 — Industrial Automation Security

**Applicability:** Specialized industrial/specialized deployments
**Priority:** P2 (Recommended)

| Clause | Requirement | Subsystem | Verification |
|--------|------------|-----------|-------------|
| 4.1 | Security level assessment | All | Documented security level per deployment |
| 4.2 | Security requirements | All | Formal security requirements document |
| 4.3 | Security architecture | Server | Network segmentation documentation |
| 3.3 | System security | Server | Hardened configuration guide |

**Note:** IEC 62443 is only relevant if Ferro is deployed in OT/ICS environments. Mark as future consideration.

---

## 3. Standards Priority Summary

### P0 — Mandatory (Must Comply)
- ISO/IEC 27001 (Information Security)
- NIST SP 800-53 (Security Controls)
- RFC 4918 (WebDAV)
- RFC 3253 (WebDAV Versioning)
- OpenID Connect Core 1.0
- Cedar Specification
- GDPR / CCPA
- OWASP Top 10

### P1 — Mandatory (Must Comply, Lower Priority)
- ISO/IEC 27034 (Application Security)
- RFC 3744 (WebDAV ACL)
- WOPI Specification
- WASM/WASI Specification

### P2 — Recommended
- IEC 62443 (Industrial Security) — only for specialized deployments
