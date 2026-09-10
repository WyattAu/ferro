# STRIDE Threat Model — Ferro System

**Document ID:** FERRO-SEC-TM-001
**Date:** 2026-04-18
**Status:** Draft
**Methodology:** STRIDE (Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege)

---

## System Boundary

The threat model covers the following components:

| Component | Description |
|-----------|-------------|
| COMP-AUTH-001 | OIDC Validator (JWT/JWKS) |
| COMP-CEDAR-002 | Cedar Authorizer (Policy Engine) |
| COMP-SESSION-003 | Session Manager |
| COMP-POLICY-004 | Policy Store |
| COMP-WEBDAV-001 | WebDAV Handler (Axum) |
| COMP-LOCK-002 | Lock Manager |
| COMP-XML-004 | XML Codec (quick-xml) |
| COMP-STORAGE-001 | Storage Engine (object_store facade) |
| COMP-CAS-002 | Content-Addressable Store (SHA-256) |
| COMP-PRESIGN-003 | Pre-signed URL Generator |
| COMP-META-004 | Metadata Store (SQLx/Postgres) |
| COMP-WASM-* | WASM Plugin System (Wasmtime/WASI) |
| Interface Layer | TLS termination, CORS, rate limiting |

---

## Spoofing

### TH-SPOOF-001: Forged JWT Tokens

| Field | Value |
|-------|-------|
| **ID** | TH-SPOOF-001 |
| **Description** | An attacker crafts a JWT with arbitrary claims (sub, groups, roles) to impersonate any user or escalate to admin. This includes signing tokens with fabricated keys, manipulating header parameters (kid, alg), or using the `none` algorithm. |
| **Affected Component** | COMP-AUTH-001 (OidcValidator) |
| **Severity** | Critical |
| **Mitigation** | 1. JWS signature verification against trusted JWKS (ALG-OIDC-VALIDATE-001 steps 2-3). 2. Algorithm allowlist restricted to RS256, RS384, RS512, ES256, ES384, ES512 (DC-AUTH-ALG-001). 3. `none` algorithm explicitly forbidden. 4. JWKS keys fetched over HTTPS from verified issuer (DC-AUTH-JWKS-002). |
| **Detection Method** | Audit log records all token validation failures with IP, token header (kid, alg), and failure reason. Alert on algorithm substitution attempts. Monitor for spikes in 401 responses. |

### TH-SPOOF-002: Fake OIDC Provider

| Field | Value |
|-------|-------|
| **ID** | TH-SPOOF-002 |
| **Description** | An attacker deploys a malicious OIDC provider and tricks Ferro into trusting it, either by DNS spoofing, SSRF on the JWKS URI, or configuration injection. The fake provider issues valid-looking tokens signed by attacker-controlled keys. |
| **Affected Component** | COMP-AUTH-001 (OidcValidator), Deployment |
| **Severity** | Critical |
| **Mitigation** | 1. Strict issuer validation: exact string match of `iss` claim against configured issuer (ALG-OIDC-VALIDATE-001 step 5). 2. OIDC Discovery document validated against expected issuer. 3. JWKS URI must use HTTPS (DC-AUTH-JWKS-002). 4. DNS pinning or certificate transparency monitoring (operational). 5. Configuration stored in env vars, not user-modifiable files. |
| **Detection Method** | Alert on issuer mismatch. Monitor JWKS key rotation frequency (unexpected rotations indicate compromise). Log all JWKS fetch operations with resolved IP. |

### TH-SPOOF-003: Pre-signed URL Forgery

| Field | Value |
|-------|-------|
| **ID** | TH-SPOOF-003 |
| **Description** | An attacker forges a pre-signed URL to access objects they are not authorized to read or write, either by manipulating URL parameters, extending the expiration, or modifying the scope (path/method). |
| **Affected Component** | COMP-PRESIGN-003 (PresignedUrlGenerator), Cloud Backends |
| **Severity** | High |
| **Mitigation** | 1. HMAC signature validation by the cloud backend (object_store Signer trait). 2. Short TTL clamped to [1s, 3600s] (DC-PRESIGN-001). 3. URL scoped to specific object path and HTTP method (INV-PRESIGN-001). 4. Staging paths use UUIDv4 prefix to prevent path guessing (INV-PRESIGN-002). 5. Cedar authorization checked before URL generation (PRE-PRESIGN-001). |
| **Detection Method** | Audit log entry created for every URL generation (POST-PRESIGN-003). Monitor for failed pre-signed URL usage (cloud backend 403s). Rate limit pre-signed URL generation (ERR-PRESIGN-003). |

---

## Tampering

### TH-TAMPER-001: Metadata Tampering

| Field | Value |
|-------|-------|
| **ID** | TH-TAMPER-001 |
| **Description** | An attacker modifies file metadata (path-to-hash mappings, ownership, timestamps) to redirect reads to different content, claim ownership of others' files, or hide evidence of unauthorized access. This includes direct database manipulation or exploiting SQL injection in metadata queries. |
| **Affected Component** | COMP-META-004 (MetadataStore), Metadata Database |
| **Severity** | Critical |
| **Mitigation** | 1. Immutable append-only audit log with hash chaining (REQ-ENT-001, NIST AU-9). 2. Metadata versioning with optimistic concurrency control. 3. SQLx compile-time query checking prevents SQL injection (OWASP A03). 4. Database access restricted to application service account. 5. Metadata mutations logged with before/after state. |
| **Detection Method** | Hash chain verification detects log tampering. Audit log anomaly detection for unexpected metadata mutations. Integrity check: periodic verification that path-to-hash mappings match physical content (SHA-256 spot checks). |

### TH-TAMPER-002: Content Replacement in CAS

| Field | Value |
|-------|-------|
| **ID** | TH-TAMPER-002 |
| **Description** | An attacker replaces the physical content at a CAS path with different bytes, causing all users referencing that content to receive corrupted or malicious data. This could be achieved by direct backend access, exploiting backend misconfiguration, or a compromised cloud account. |
| **Affected Component** | COMP-CAS-002 (CasStore), Cloud Backends |
| **Severity** | Critical |
| **Mitigation** | 1. SHA-256 verification on every read (DC-INTEGRITY-001, PROP-CAS-002). 2. Mismatch triggers `IntegrityFailure` error and audit log entry. 3. Atomic puts prevent partial writes (PROP-CAS-003, object_store PutMode::Create). 4. ETag validation on cloud backends. 5. Cloud backend SSE (server-side encryption) recommended (GDPR Art. 32). |
| **Detection Method** | Every `get_content` computes and compares SHA-256. IntegrityFailure events are audit-logged with hash, path, and timestamp. Alert on any IntegrityFailure — indicates either backend compromise or data corruption. |

### TH-TAMPER-003: Cedar Policy Injection

| Field | Value |
|-------|-------|
| **ID** | TH-TAMPER-003 |
| **Description** | An attacker injects or modifies Cedar policies to grant themselves unauthorized access. This includes exploiting the policy management API, writing malicious policies to the file-backed policy store, or manipulating the database directly. |
| **Affected Component** | COMP-POLICY-004 (PolicyStore), COMP-CEDAR-002 (CedarAuthorizer) |
| **Severity** | Critical |
| **Mitigation** | 1. Policy schema validation on every load (DC-AUTH-CEDAR-VALIDATE-001). 2. Admin-only write access enforced by Cedar itself (circular bootstrap). 3. Policy size limit 256 KiB (DC-AUTH-CEDAR-SIZE-001). 4. Policy count limit 10,000 (DC-AUTH-CEDAR-MAX-001). 5. Policy changes audit-logged with admin identity and full diff. 6. Database write access restricted to application service account. |
| **Detection Method** | Policy change audit log with admin signature. Alert on policy modifications outside business hours. Policy validation rejects syntactically/semantically invalid policies. Periodic policy review by security team. |

### TH-TAMPER-004: XML Injection in WebDAV

| Field | Value |
|-------|-------|
| **ID** | TH-TAMPER-004 |
| **Description** | An attacker sends crafted XML payloads in WebDAV requests (PROPFIND, PROPPATCH, LOCK) to inject malicious content, exploit XML entity expansion (billion laughs attack), or manipulate the XML parser into reading local files (XXE). |
| **Affected Component** | COMP-XML-004 (XmlCodec), COMP-WEBDAV-001 |
| **Severity** | High |
| **Mitigation** | 1. `quick-xml` in streaming mode with entity expansion disabled (configurable entity limits). 2. No DTD processing (prevents XXE). 3. Request body size limit 1 MiB (DC-XML-002). 4. Input sanitization on all XML text content. 5. XML output uses proper encoding and escaping. |
| **Detection Method** | Request size limit middleware rejects oversized payloads (413). XML parse errors logged with payload hash. Monitor for spikes in XML parse failures. Rate limiting on WebDAV endpoints. |

---

## Repudiation

### TH-REPUD-001: Denial of File Operations

| Field | Value |
|-------|-------|
| **ID** | TH-REPUD-001 |
| **Description** | A user performs unauthorized file operations (delete, modify, read sensitive files) and denies having done so. Without proper audit trails, it is impossible to prove or disprove the claim. |
| **Affected Component** | Enterprise (Audit Log), All API endpoints |
| **Severity** | High |
| **Mitigation** | 1. Immutable append-only audit log with tamper-evident hash chaining (REQ-ENT-001). 2. Every authenticated request logged with: timestamp, principal (sub), action, resource path, result (allow/deny), client IP, user-agent. 3. Audit log stored separately from application database (NIST AU-9). 4. Write-only log service prevents retroactive modification. 5. gRPC streaming for real-time SIEM integration (REQ-ENT-002). |
| **Detection Method** | Audit log query API for forensic analysis. Hash chain integrity verification. SIEM correlation with other security events. Automated anomaly detection (unusual access patterns, bulk deletions). |

### TH-REPUD-002: Denial of Policy Changes

| Field | Value |
|-------|-------|
| **ID** | TH-REPUD-002 |
| **Description** | An administrator modifies Cedar policies (granting unauthorized access, removing security controls) and denies responsibility. Without proper policy change tracking, it is impossible to determine who made the change and when. |
| **Affected Component** | COMP-POLICY-004 (PolicyStore) |
| **Severity** | Medium |
| **Mitigation** | 1. Policy change logging with admin identity, timestamp, full policy diff (before/after). 2. Policy version history stored in database (optimistic concurrency via `version` column). 3. Admin session binding — policy changes require active admin session. 4. Policy rollback capability from version history. 5. MFA required for policy modification (TH-ELEVATE-002). |
| **Detection Method** | Policy change audit log with admin identity. Alert on policy changes. Policy diff review workflow. Version history enables point-in-time reconstruction. |

---

## Information Disclosure

### TH-DISCLOSE-001: Unauthorized File Access

| Field | Value |
|-------|-------|
| **ID** | TH-DISCLOSE-001 |
| **Description** | An attacker accesses files they are not authorized to read, either by exploiting path traversal in the WebDAV handler, bypassing Cedar authorization, or manipulating user paths to resolve to other users' content. |
| **Affected Component** | COMP-CEDAR-002 (CedarAuthorizer), COMP-WEBDAV-001, COMP-STORAGE-001 |
| **Severity** | Critical |
| **Mitigation** | 1. Cedar deny-by-default semantics (PROP-AUTH-002, THM-CEDAR-001). 2. Path traversal prevention: `..` rejection, path normalization (DC-PATH-003, DC-PATH-004, PRE-STORAGE-001). 3. UserPath validation against traversal patterns (TV-CAS-018). 4. Per-request Cedar evaluation for every API call. 5. No routes exist outside the middleware chain (BP-AUTH-MIDDLEWARE-001 §7). |
| **Detection Method** | Cedar Deny decisions logged for every request (DC-AUTH-LOG-001). Alert on spikes in 403 responses. Audit log analysis for path traversal attempts. Monitor for access patterns inconsistent with user roles. |

### TH-DISCLOSE-002: Pre-signed URL Leakage

| Field | Value |
|-------|-------|
| **ID** | TH-DISCLOSE-002 |
| **Description** | A pre-signed URL is leaked (via logs, referer headers, shared links, browser history) allowing unintended parties to access the target object. The URL may be further distributed or cached. |
| **Affected Component** | COMP-PRESIGN-003 (PresignedUrlGenerator) |
| **Severity** | High |
| **Mitigation** | 1. Short TTL (default 60s, max 3600s, DC-PRESIGN-001). 2. Single-use token option (consume on first use, track in metadata). 3. Audit log entry for every URL generation and consumption. 4. Cedar authorization required before URL generation (PRE-PRESIGN-001). 5. UUIDv4 staging paths prevent URL guessing (INV-PRESIGN-002). 6. Rate limiting on URL generation. |
| **Detection Method** | Audit log correlation: URL generation vs. consumption. Alert on same URL consumed from different IPs. Monitor for URLs consumed after expiry. Rate limit alerts on URL generation. |

### TH-DISCLOSE-003: Metadata Exposure

| Field | Value |
|-------|-------|
| **ID** | TH-DISCLOSE-003 |
| **Description** | Sensitive metadata (file ownership, access patterns, custom attributes) is exposed to unauthorized users through API responses, PROPFIND properties, or audit log access. This may violate GDPR data minimization principles. |
| **Affected Component** | COMP-WEBDAV-001, COMP-PROP-003, Enterprise (Audit Log) |
| **Severity** | Medium |
| **Mitigation** | 1. Per-field authorization: Cedar policies control which metadata fields are visible per principal. 2. WebDAV dead properties respect Cedar ACL (REQ-WEBDAV-003). 3. Audit log access restricted to admin principals (NIST AU-9). 4. GDPR compliance: only collect necessary metadata (GDPR Art. 5, Art. 30). 5. Error responses do not leak internal details (DC-AUTH-ERROR-001). |
| **Detection Method** | Audit log for metadata access patterns. Review PROPFIND responses for unintended property exposure. GDPR compliance audit of collected metadata fields. |

### TH-DISCLOSE-004: OIDC Token Leakage

| Field | Value |
|-------|-------|
| **ID** | TH-DISCLOSE-004 |
| **Description** | OIDC tokens (access tokens, ID tokens, refresh tokens) are leaked via insecure storage, transmission, or logging, allowing session hijacking or identity theft. |
| **Affected Component** | COMP-AUTH-001, COMP-SESSION-003 |
| **Severity** | Critical |
| **Mitigation** | 1. Secure cookie flags: HttpOnly, Secure, SameSite=Strict (BP-AUTH-MIDDLEWARE-001 §10). 2. Short token lifetime (max 600s, DC-AUTH-TOKEN-TTL-001). 3. Refresh token rotation: new refresh token issued on each use, old one invalidated. 4. Tokens encrypted at rest in session store. 5. Tokens never logged or included in error responses. 6. HTTPS-only for all OIDC endpoints. |
| **Detection Method** | Monitor for token reuse (refresh token replay). Alert on sessions from unusual IPs/locations. Session anomaly detection. Audit log for session creation/destruction. |

---

## Denial of Service

### TH-DOS-001: Large File Upload Exhaustion

| Field | Value |
|-------|-------|
| **ID** | TH-DOS-001 |
| **Description** | An attacker uploads extremely large files or many concurrent uploads to exhaust server memory, disk, bandwidth, or cloud storage costs. This includes exploiting multipart upload to create many uncommitted parts. |
| **Affected Component** | COMP-STORAGE-001, COMP-CAS-002, Cloud Backends |
| **Severity** | High |
| **Mitigation** | 1. Upload size limit: 5 TB max per object (DC-STORAGE-001). 2. Multipart max parts enforced by object_store. 3. Configurable size threshold for pre-signed URL offload (REQ-STOR-004). 4. Rate limiting on upload endpoints. 5. Per-user storage quotas (configurable). 6. Streaming SHA-256 computation avoids buffering entire content. |
| **Detection Method** | Monitor storage consumption per user. Alert on unusual upload patterns (sustained high throughput, many concurrent uploads). Cloud backend cost monitoring. Rate limit hit counters. |

### TH-DOS-002: PROPFIND Depth:infinity on Huge Tree

| Field | Value |
|-------|-------|
| **ID** | TH-DOS-002 |
| **Description** | An attacker sends a PROPFIND with Depth:infinity against a directory containing millions of files, causing the server to enumerate the entire tree, consume memory, and exhaust CPU generating the XML response. |
| **Affected Component** | COMP-WEBDAV-001, COMP-XML-004, COMP-STORAGE-001 |
| **Severity** | High |
| **Mitigation** | 1. Max depth limit (100, DC-PATH-005, config `max_depth_limit`). 2. Streaming XML response: O(1) memory per element (ADR-002). 3. Configurable directory listing pagination. 4. Request timeout for long-running PROPFIND operations. 5. Rate limiting per IP (100 req/s default). |
| **Detection Method** | Monitor PROPFIND response times and sizes. Alert on depth-limited responses (502). Track per-IP request rates. Monitor memory usage during directory enumeration. |

### TH-DOS-003: WASM Resource Exhaustion

| Field | Value |
|-------|-------|
| **ID** | TH-DOS-003 |
| **Description** | A malicious or buggy WASM plugin consumes excessive CPU, memory, or wall-clock time, starving other plugins and the Ferro server itself. This includes infinite loops, unbounded memory allocation, or recursive computation. |
| **Affected Component** | WASM Plugin System (Wasmtime/WASI), REQ-WASM-003, REQ-WASM-004 |
| **Severity** | High |
| **Mitigation** | 1. Wasmtime fuel metering: configurable CPU fuel limit per execution (REQ-WASM-004). 2. Memory limit: configurable max memory allocation per plugin instance. 3. Wall-clock timeout: configurable execution time limit with enforced termination. 4. Plugin termination does not affect server or other plugins (isolation). 5. WASI capability restrictions prevent filesystem/network access (REQ-WASM-003). |
| **Detection Method** | Plugin execution metrics (fuel consumed, memory peak, wall-clock time). Alert on plugins hitting resource limits. Monitor server responsiveness during plugin execution. Audit log for plugin terminations. |

### TH-DOS-004: Lock Starvation

| Field | Value |
|-------|-------|
| **ID** | TH-DOS-004 |
| **Description** | An attacker acquires many locks (exclusive or shared) to prevent other users from accessing resources, or holds locks indefinitely by continuously refreshing them. |
| **Affected Component** | COMP-LOCK-002 (LockManager) |
| **Severity** | Medium |
| **Mitigation** | 1. Lock timeout enforcement (default 60s, max 3600s, DC-LOCK-001..003). 2. Max locks per principal (1000, config `max_locks_per_principal`). 3. Lazy expiry cleanup on access. 4. WAL journal for lock state recovery (no orphaned locks after restart). 5. Admin unlock capability for stale locks. |
| **Detection Method** | Monitor lock count per principal. Alert on principals approaching lock limit. Track lock acquisition rate. Monitor for lock timeout expirations. |

### TH-DOS-005: Connection Pool Exhaustion

| Field | Value |
|-------|-------|
| **ID** | TH-DOS-005 |
| **Description** | An attacker opens many concurrent HTTP connections or WebSocket connections to exhaust the server's connection pool, preventing legitimate users from connecting. This includes slow-loris style attacks that hold connections open without completing requests. |
| **Affected Component** | Axum HTTP Server, Database Connection Pool |
| **Severity** | Medium |
| **Mitigation** | 1. Max concurrent connections configurable at Axum level. 2. Connection timeout for idle connections. 3. Request timeout for slow/incomplete requests. 4. Database connection pool sizing with backpressure. 5. Rate limiting per IP (tower-governor). 6. Cloud load balancer with connection draining. |
| **Detection Method** | Monitor active connection count. Alert on connection pool saturation. Track per-IP connection rates. Monitor request queue depth. |

---

## Elevation of Privilege

### TH-ELEVATE-001: Cedar Policy Bypass

| Field | Value |
|-------|-------|
| **ID** | TH-ELEVATE-001 |
| **Description** | An attacker bypasses Cedar authorization to perform actions they are not permitted to do. This includes exploiting policy evaluation bugs, crafting requests that match unintended policy patterns, or finding routes that skip the Cedar middleware layer. |
| **Affected Component** | COMP-CEDAR-002 (CedarAuthorizer), Middleware Chain |
| **Severity** | Critical |
| **Mitigation** | 1. Cedar deny-by-default guaranteed by evaluation semantics (PROP-AUTH-002, THM-CEDAR-001). 2. Forbid-overrides-permit semantics (deny always wins). 3. No routes exist outside the middleware chain — all paths require auth. 4. Cedar is formally verified in Lean 4 (differential testing with spec). 5. Policy schema validation catches semantic errors at load time. 6. Authorization decision logged for every request (DC-AUTH-LOG-001). |
| **Detection Method** | All Cedar decisions (Allow/Deny) audit-logged. Alert on unexpected Allow decisions. Policy evaluation error monitoring (errors default to Deny). Regular authorization test suite execution. |

### TH-ELEVATE-002: Admin Account Compromise

| Field | Value |
|-------|-------|
| **ID** | TH-ELEVATE-002 |
| **Description** | An attacker gains access to an admin account (via credential theft, session hijacking, or social engineering) and uses admin privileges to modify policies, access all data, or disrupt the system. |
| **Affected Component** | COMP-POLICY-004, Admin Dashboard, OIDC Provider |
| **Severity** | Critical |
| **Mitigation** | 1. MFA enforcement at OIDC provider level. 2. Limited admin sessions: shorter idle timeout, shorter absolute lifetime. 3. Admin actions require re-authentication for sensitive operations (policy changes, user management). 4. Policy change audit log with admin identity. 5. Principle of least privilege: admin role split into sub-roles (policy admin, user admin, system admin). 6. Session anomaly detection (unusual IP, location, device). |
| **Detection Method** | Admin action audit log. Alert on admin login from new IP/location. Session anomaly detection. Policy change review workflow. MFA bypass attempt monitoring. |

### TH-ELEVATE-003: WASM Sandbox Escape

| Field | Value |
|-------|-------|
| **ID** | TH-ELEVATE-003 |
| **Description** | A malicious WASM plugin escapes the Wasmtime sandbox to access the host filesystem, network, environment variables, or other system resources, potentially gaining control of the Ferro server process. |
| **Affected Component** | WASM Plugin System (Wasmtime/WASI), REQ-WASM-003 |
| **Severity** | Critical |
| **Mitigation** | 1. Capability-based security: WASI capabilities explicitly granted per plugin (no default access). 2. No raw syscalls from WASM — all host access through WASI API. 3. Wasmtime sandbox provides memory isolation between plugin and host. 4. No filesystem, network, or environment variable access unless explicitly granted. 5. Plugin runs in separate Wasmtime instance (no shared state with host). 6. Fuel metering and memory limits prevent resource-based escape vectors. 7. Regular Wasmtime security updates (cargo-audit). |
| **Detection Method** | Wasmtime trap monitoring (plugin attempted unauthorized operation). Audit log for all WASI capability violations. Server health monitoring during/after plugin execution. Periodic security review of plugin code. Monitor Wasmtime CVEs. |

---

## Threat Summary by Severity

| Severity | Count | Threat IDs |
|----------|-------|------------|
| **Critical** | 10 | TH-SPOOF-001, TH-SPOOF-002, TH-TAMPER-001, TH-TAMPER-002, TH-TAMPER-003, TH-DISCLOSE-001, TH-DISCLOSE-004, TH-ELEVATE-001, TH-ELEVATE-002, TH-ELEVATE-003 |
| **High** | 7 | TH-SPOOF-003, TH-TAMPER-004, TH-REPUD-001, TH-DISCLOSE-002, TH-DOS-001, TH-DOS-002, TH-DOS-003 |
| **Medium** | 5 | TH-REPUD-002, TH-DISCLOSE-003, TH-DOS-004, TH-DOS-005 |
| **Total** | **22** | |

## Component Threat Density

| Component | Threat Count | Highest Severity |
|-----------|-------------|-----------------|
| COMP-AUTH-001 (OidcValidator) | 3 | Critical |
| COMP-CEDAR-002 (CedarAuthorizer) | 2 | Critical |
| COMP-POLICY-004 (PolicyStore) | 2 | Critical |
| COMP-STORAGE-001 / COMP-CAS-002 | 3 | Critical |
| COMP-PRESIGN-003 | 2 | High |
| COMP-WEBDAV-001 / COMP-XML-004 | 3 | High |
| COMP-LOCK-002 | 1 | Medium |
| COMP-META-004 | 1 | Critical |
| WASM Plugin System | 2 | Critical |
| Enterprise (Audit Log) | 2 | High |
| COMP-SESSION-003 | 1 | Critical |

## Cross-Reference to Standards

| Threat | OWASP Top 10 | NIST 800-53 | ISO 27001 | GDPR |
|--------|-------------|-------------|-----------|------|
| TH-SPOOF-001 | A07:2021 | IA-2, IA-3, IA-5 | A.9.2, A.9.4 | Art. 32 |
| TH-SPOOF-002 | A07:2021 | IA-2, IA-5, SC-12 | A.9.2 | Art. 32 |
| TH-SPOOF-003 | A10:2021 | SC-12, AC-17 | A.5.14 | Art. 32 |
| TH-TAMPER-001 | A08:2021 | AU-9, SC-13 | A.12.4, A.8.3 | Art. 5(1)(f), Art. 30 |
| TH-TAMPER-002 | A08:2021 | SC-8, SC-13 | A.8.3, A.12.2 | Art. 5(1)(f) |
| TH-TAMPER-003 | A01:2021 | AC-3, AC-6 | A.9.1 | Art. 25 |
| TH-TAMPER-004 | A03:2021 | SI-10 | A.8.25 | Art. 32 |
| TH-REPUD-001 | A09:2021 | AU-2, AU-3, AU-9 | A.12.4 | Art. 30 |
| TH-REPUD-002 | A09:2021 | AU-2, AU-3 | A.12.4 | Art. 30 |
| TH-DISCLOSE-001 | A01:2021 | AC-3, AC-4 | A.9.1, A.8.3 | Art. 25 |
| TH-DISCLOSE-002 | A10:2021 | AC-17 | A.5.14 | Art. 32 |
| TH-DISCLOSE-003 | A01:2021 | AC-3 | A.9.1, A.8.12 | Art. 5, Art. 30 |
| TH-DISCLOSE-004 | A07:2021 | SC-23, IA-5 | A.9.4 | Art. 32 |
| TH-DOS-001 | A04:2021 | SC-5, SC-6 | A.12.4 | Art. 32 |
| TH-DOS-002 | A04:2021 | SC-5 | A.12.4 | Art. 32 |
| TH-DOS-003 | A04:2021 | SC-7 | A.15.1 | Art. 32 |
| TH-DOS-004 | A04:2021 | SC-5 | A.12.4 | — |
| TH-DOS-005 | A04:2021 | SC-5, SC-6 | A.12.4 | Art. 32 |
| TH-ELEVATE-001 | A01:2021 | AC-3, AC-6 | A.9.1 | Art. 25 |
| TH-ELEVATE-002 | A07:2021 | AC-2, AC-6 | A.8.2 | Art. 32 |
| TH-ELEVATE-003 | A04:2021 | SC-7, SI-7 | A.8.25, A.15.1 | Art. 32 |
