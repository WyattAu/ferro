# Ferro Attack Scenarios — Red Team Exercise Document

**Document:** Comprehensive Attack Scenarios  
**Version:** 1.0.0  
**Date:** 2026-07-12  
**Status:** Draft  
**Framework:** OWASP Top 10 2021, CWE/SANS Top 25

---

## OWASP A01:2021 — Broken Access Control

### AS-AC-001: Horizontal Privilege Escalation via IDOR

**Scenario ID:** AS-AC-001  
**Attack Description:** Attacker accesses another user's files by manipulating file path parameters or API identifiers.  
**Prerequisites:** Authenticated as non-admin user.  
**Expected Ferro Behavior:** Cedar policy engine enforces user-scoped access; path normalization prevents traversal; session-bound file access.  
**Detection Mechanism:** Audit middleware logs unauthorized access attempts; Cedar policy denies cross-user access.  
**Severity:** Critical  
**Remediation:** Ensure Cedar policies are evaluated on every file operation; add integration tests for cross-user access attempts.

### AS-AC-002: Vertical Privilege Escalation via Admin API

**Scenario ID:** AS-AC-002  
**Attack Description:** Regular user accesses `/api/admin/*` endpoints to perform admin operations (create users, delete data, modify configs).  
**Prerequisites:** Authenticated as regular user.  
**Expected Ferro Behavior:** Admin middleware rejects non-admin roles; Cedar policy blocks admin operations.  
**Detection Mechanism:** Admin API audit logging; 403 response with request ID.  
**Severity:** Critical  
**Remediation:** Verify admin role check is applied to all `/api/admin/*` routes; no bypass via HTTP method override.

### AS-AC-003: Guest Token Privilege Escalation

**Scenario ID:** AS-AC-003  
**Attack Description:** Guest token is reused or escalated to access user-level or admin-level endpoints.  
**Prerequisites:** Valid guest token from share link.  
**Expected Ferro Behavior:** Guest middleware restricts access to share-specific endpoints only.  
**Detection Mechanism:** Guest middleware audit logging; 403 on unauthorized endpoints.  
**Severity:** High  
**Remediation:** Validate guest token scope on every request; restrict to share path only.

### AS-AC-004: Path Traversal to Public Endpoints

**Scenario ID:** AS-AC-004  
**Attack Description:** Attacker uses path traversal (`../../`) to reach public endpoints (metrics, health, shares) without authentication.  
**Prerequisites:** None.  
**Expected Ferro Behavior:** `is_public_auth_path()` boundary rejects traversal; path normalization collapses `..` before routing.  
**Detection Mechanism:** Request smuggling middleware; path normalization logs.  
**Severity:** High  
**Remediation:** Test path normalization against all traversal encoding (URL-encoded, double-encoded, null bytes).

### AS-AC-005: Cedar Policy Bypass via Malformed Input

**Scenario ID:** AS-AC-005  
**Attack Description:** Attacker crafts Cedar policy input that exploits parsing differences to bypass authorization.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Cedar policy engine rejects malformed policies; strict input validation on policy context.  
**Detection Mechanism:** Cedar policy evaluation audit; error logging on malformed input.  
**Severity:** Critical  
**Remediation:** Fuzz Cedar policy engine with malformed inputs; verify policy evaluation is consistent.

---

## OWASP A02:2021 — Cryptographic Failures

### AS-CF-001: JWT Algorithm Confusion

**Scenario ID:** AS-CF-001  
**Attack Description:** Attacker forges JWT using `none` algorithm or swaps RS256→HS256 to bypass signature verification.  
**Prerequisites:** Knowledge of JWT structure.  
**Expected Ferro Behavior:** `OidcValidator` rejects `none` algorithm; enforces configured algorithm; JWKS keys used for verification.  
**Detection Mechanism:** Token validation audit; 401 on invalid token.  
**Severity:** Critical  
**Remediation:** Verify algorithm whitelist in OIDC validator; test with `none`, HS256, RS256 confusion.

### AS-CF-002: JWKS Cache Poisoning

**Scenario ID:** AS-CF-002  
**Attack Description:** Attacker controls OIDC issuer and serves malicious JWKS keys to Ferro's JWKS cache.  
**Prerequisites:** Attacker-controlled OIDC provider.  
**Expected Ferro Behavior:** JWKS fetched from trusted issuer only; key rotation validated; cache TTL enforced.  
**Detection Mechanism:** JWKS refresh audit; key ID validation.  
**Severity:** Critical  
**Remediation:** Validate JWKS issuer matches configured OIDC issuer; pin expected key IDs.

### AS-CF-003: API Key Timing Attack

**Scenario ID:** AS-CF-003  
**Attack Description:** Attacker uses timing side-channel to guess API key characters by measuring response times.  
**Prerequisites:** Network access to API.  
**Expected Ferro Behavior:** Constant-time string comparison for API key validation.  
**Detection Mechanism:** Rate limiting on auth failures; timing analysis of responses.  
**Severity:** Medium  
**Remediation:** Verify constant-time comparison is used; benchmark timing variance.

### AS-CF-004: E2EE Key Management Weakness

**Scenario ID:** AS-CF-004  
**Attack Description:** Attacker extracts E2EE keys from memory, config, or backup files.  
**Prerequisites:** Access to server filesystem or memory dump.  
**Expected Ferro Behavior:** age-based E2E encryption (X25519, ChaCha20-Poly1305); keys not stored in plaintext; zeroize on drop.  
**Detection Mechanism:** Memory protection; file permission checks.  
**Severity:** Critical  
**Remediation:** Verify key zeroize-on-drop; audit key storage locations; test memory dump extraction.

### AS-CF-005: TLS Downgrade Attack

**Scenario ID:** AS-CF-005  
**Attack Description:** Attacker forces TLS connection to use weak cipher suite or older protocol version.  
**Prerequisites:** Network position (MITM).  
**Expected Ferro Behavior:** rustls enforces TLS 1.3; rejects weak ciphers; HSTS header prevents downgrade.  
**Detection Mechanism:** TLS handshake logs; HSTS enforcement.  
**Severity:** High  
**Remediation:** Verify TLS 1.3 only; test with TLS 1.2/1.1/1.0 connections; verify HSTS max-age.

### AS-CF-006: Argon2 Parameter Weakness

**Scenario ID:** AS-CF-006  
**Attack Description:** Password hashing uses insufficient memory/time cost, enabling GPU brute-force.  
**Prerequisites:** Access to password hash database.  
**Expected Ferro Behavior:** Argon2 with cost factor 12 (~250ms); sufficient memory parameter.  
**Detection Mechanism:** Hash parameter audit; benchmark cracking speed.  
**Severity:** Medium  
**Remediation:** Verify Argon2 parameters (memory: 64MB+, time: 4+, parallelism: 1+); benchmark against GPU attacks.

---

## OWASP A03:2021 — Injection

### AS-INJ-001: SQL Injection via Query Parameters

**Scenario ID:** AS-INJ-001  
**Attack Description:** Attacker injects SQL via search, sort, or filter parameters in list endpoints.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** SQLite queries use parameterized statements; no string concatenation in SQL.  
**Detection Mechanism:** SQL error logging; query parameterization audit.  
**Severity:** Critical  
**Remediation:** Grep for string concatenation in SQL; fuzz all query parameters with SQL payloads.

### AS-INJ-002: Path Traversal in File Operations

**Scenario ID:** AS-INJ-002  
**Attack Description:** Attacker uses `../../etc/passwd` or URL-encoded variants to access files outside storage root.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Path normalization collapses `..`; canonical path check against storage root; null byte rejection.  
**Detection Mechanism:** Path normalization audit; 403 on traversal attempts.  
**Severity:** Critical  
**Remediation:** Test all traversal variants (URL-encoded, double-encoded, null bytes, Unicode normalization).

### AS-INJ-003: XXE via WebDAV PROPFIND

**Scenario ID:** AS-INJ-003  
**Attack Description:** Attacker sends XML with external entity references to read local files or cause SSRF.  
**Prerequisites:** Authenticated user with WebDAV access.  
**Expected Ferro Behavior:** XML parser disables external entities; entity expansion limit enforced.  
**Detection Mechanism:** XML parsing error logs; entity expansion metrics.  
**Severity:** Critical  
**Remediation:** Verify XML parser configuration (no DTD, no external entities); test billion laughs attack.

### AS-INJ-004: SSRF via Federation Proxy

**Scenario ID:** AS-INJ-004  
**Attack Description:** Attacker crafts federation activity with internal network URLs to scan or access internal services.  
**Prerequisites:** Federation enabled.  
**Expected Ferro Behavior:** Federation proxy validates actor URLs; blocks internal IP ranges; rate limits outbound requests.  
**Detection Mechanism:** Federation proxy audit; outbound request logging.  
**Severity:** High  
**Remediation:** Implement SSRF protection (block RFC 1918, link-local); test with internal IP targets.

### AS-INJ-005: SSRF via Remote Mount Proxy

**Scenario ID:** AS-INJ-005  
**Attack Description:** Attacker creates remote mount pointing to internal services (metadata endpoints, databases).  
**Prerequisites:** Admin access (mount creation is admin-only).  
**Expected Ferro Behavior:** Remote mount proxy validates URLs; blocks internal network ranges; requires admin authorization.  
**Detection Mechanism:** Mount creation audit; outbound request logging.  
**Severity:** High  
**Remediation:** Block internal IP ranges in mount URL validation; test with `169.254.169.254`, `10.x`, `192.168.x`.

### AS-INJ-006: GraphQL Injection / Deep Nesting DoS

**Scenario ID:** AS-INJ-006  
**Attack Description:** Attacker sends deeply nested GraphQL queries to exhaust CPU/memory.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Query depth limit; query complexity analysis; timeout enforcement.  
**Detection Mechanism:** Query depth logging; timeout metrics.  
**Severity:** Medium  
**Remediation:** Verify query depth limit (max 10-15); test with 100+ nesting levels.

### AS-INJ-007: LDAP Injection

**Scenario ID:** AS-INJ-007  
**Attack Description:** Attacker injects LDAP filter metacharacters in username/password to bypass authentication.  
**Prerequisites:** LDAP auth enabled.  
**Expected Ferro Behavior:** Parameterized LDAP queries via `ldap3` crate; input sanitization on bind DN.  
**Detection Mechanism:** LDAP error logging; failed bind audit.  
**Severity:** High  
**Remediation:** Verify parameterized queries; test with `*)(uid=*))(|(uid=*` payloads.

### AS-INJ-008: Chat Message Stored XSS

**Scenario ID:** AS-INJ-008  
**Attack Description:** Attacker sends chat message containing JavaScript that executes in other users' browsers.  
**Prerequisites:** Authenticated user with chat access.  
**Expected Ferro Behavior:** Message content escaped in HTML response; Content-Security-Policy blocks inline scripts.  
**Detection Mechanism:** CSP violation reports; message sanitization audit.  
**Severity:** Medium  
**Remediation:** Verify HTML escaping in chat responses; test CSP enforcement.

### AS-INJ-009: CalDAV/CardDAV vCard/iCal Injection

**Scenario ID:** AS-INJ-009  
**Attack Description:** Attacker uploads malformed vCard/iCal containing malicious payloads (XSS, command injection).  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** vCard/iCal parser rejects malformed input; sanitizes display values; Content-Type validation.  
**Detection Mechanism:** Parser error logs; Content-Type mismatch logging.  
**Severity:** Medium  
**Remediation:** Fuzz vCard/iCal parsers with malformed payloads; verify Content-Type enforcement.

### AS-INJ-010: Filename Path Traversal in Uploads

**Scenario ID:** AS-INJ-010  
**Attack Description:** Attacker uploads file with filename `../../etc/cron.d/malicious` to write to arbitrary paths.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Filename sanitization strips `..`, `/`, null bytes; safe filename generated.  
**Detection Mechanism:** Filename sanitization audit; path validation logs.  
**Severity:** High  
**Remediation:** Verify filename sanitization handles all traversal variants; test with null bytes, Unicode.

---

## OWASP A04:2021 — Insecure Design

### AS-ID-001: Missing Rate Limiting on Auth Endpoints

**Scenario ID:** AS-ID-001  
**Attack Description:** Attacker brute-forces login credentials without rate limiting.  
**Prerequisites:** Network access.  
**Expected Ferro Behavior:** Token bucket rate limiter per IP; separate rate limit for auth endpoints.  
**Detection Mechanism:** Rate limit metrics; 429 responses.  
**Severity:** High  
**Remediation:** Verify per-IP rate limiting on `/api/auth/*`; test with 1000+ requests/minute.

### AS-ID-002: Share Token Brute Force

**Scenario ID:** AS-ID-002  
**Attack Description:** Attacker generates random share tokens to access private shares.  
**Prerequisites:** Knowledge of share endpoint pattern.  
**Expected Ferro Behavior:** UUID v4 tokens (122-bit entropy); per-token lockout (10 fails / 5 min); expiration.  
**Detection Mechanism:** Share access audit; lockout metrics.  
**Severity:** High  
**Remediation:** Verify token entropy (UUID v4); test brute-force with 1M+ token attempts.

### AS-ID-003: Missing Input Validation on Batch Operations

**Scenario ID:** AS-ID-003  
**Attack Description:** Attacker sends malformed batch delete/copy/move requests to corrupt data.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Batch operations validate all inputs; atomic execution; rollback on partial failure.  
**Detection Mechanism:** Batch operation audit; error logging.  
**Severity:** Medium  
**Remediation:** Test batch operations with malformed paths, mixed valid/invalid inputs.

---

## OWASP A05:2021 — Security Misconfiguration

### AS-SC-001: Default Admin Password in Production

**Scenario ID:** AS-SC-001  
**Attack Description:** Attacker uses default admin credentials to access the server.  
**Prerequisites:** Default configuration.  
**Expected Ferro Behavior:** `default_password_layer` forces password change on first login; blocks admin operations until changed.  
**Detection Mechanism:** Default password audit; forced change logging.  
**Severity:** High  
**Remediation:** Verify forced password change; test admin operations with default password.

### AS-SC-002: CORS Misconfiguration

**Scenario ID:** AS-SC-002  
**Attack Description:** Attacker uses `Access-Control-Allow-Origin: *` with credentials to steal tokens.  
**Prerequisites:** Auth session.  
**Expected Ferro Behavior:** CORS logs warning when `*` used with auth; configurable allowed origins; credentials mode validated.  
**Detection Mechanism:** CORS header audit; warning logs.  
**Severity:** High  
**Remediation:** Verify CORS rejects `*` with credentials; test with spoofed Origin headers.

### AS-SC-003: Missing Security Headers

**Scenario ID:** AS-SC-003  
**Attack Description:** Attacker exploits missing HSTS, CSP, or X-Frame-Options headers.  
**Prerequisites:** Network access.  
**Expected Ferro Behavior:** CSP, HSTS (HTTPS only), X-Frame-Options: DENY, X-Content-Type-Options: nosniff, Referrer-Policy, Permissions-Policy.  
**Detection Mechanism:** Security header audit; header presence checks.  
**Severity:** Medium  
**Remediation:** Verify all security headers present; test with header removal attempts.

### AS-SC-004: Debug Mode in Production

**Scenario ID:** AS-SC-004  
**Attack Description:** Attacker accesses debug endpoints or verbose error messages in production.  
**Prerequisites:** Network access.  
**Expected Ferro Behavior:** Debug endpoints disabled in release builds; error messages sanitized; panic handler logs to server only.  
**Detection Mechanism:** Error message audit; debug endpoint checks.  
**Severity:** Medium  
**Remediation:** Verify `#[cfg(debug_assertions)]` guards; test error responses for internal paths.

### AS-SC-005: Metrics Endpoint Information Disclosure

**Scenario ID:** AS-SC-005  
**Attack Description:** Attacker reads `/metrics` to discover internal state, user counts, or performance bottlenecks.  
**Prerequisites:** Network access.  
**Expected Ferro Behavior:** Metrics endpoint public but limited to non-sensitive counters; no user data in metrics.  
**Detection Mechanism:** Metrics content audit; sensitive data checks.  
**Severity:** Low  
**Remediation:** Audit metrics output for PII; restrict metrics to internal network if sensitive.

---

## OWASP A06:2021 — Vulnerable and Outdated Components

### AS-VC-001: Known Vulnerability in Dependencies

**Scenario ID:** AS-VC-001  
**Attack Description:** Attacker exploits known CVE in Ferro's dependency tree.  
**Prerequisites:** Vulnerability exists in dependency.  
**Expected Ferro Behavior:** `cargo audit` in CI; dependency policy (no critical CVEs, maintained dependencies); `Cargo.lock` pinning.  
**Detection Mechanism:** `cargo audit` CI pipeline; dependency review process.  
**Severity:** High  
**Remediation:** Run `cargo audit`; review transitive dependencies (bincode, paste, proc-macro-error); update or replace.

### AS-VC-002: Supply Chain Attack via Malicious Crate

**Scenario ID:** AS-VC-002  
**Attack Description:** Attacker publishes malicious crate that Ferro depends on (typosquatting, maintainer compromise).  
**Prerequisites:** Compromised crate in dependency tree.  
**Expected Ferro Behavior:** Prefer pure-Rust implementations; minimize dependency depth; manual review of new dependencies.  
**Detection Mechanism:** `cargo audit`; dependency review; crate reputation checks.  
**Severity:** Medium  
**Remediation:** Audit dependency tree; verify crate publishers; use `cargo-vet` for supply chain verification.

---

## OWASP A07:2021 — Identification and Authentication Failures

### AS-AUTH-001: OIDC State Parameter Reuse

**Scenario ID:** AS-AUTH-001  
**Attack Description:** Attacker reuses OIDC state parameter to hijack session.  
**Prerequisites:** Valid OIDC session.  
**Expected Ferro Behavior:** State parameter single-use; session fixation prevention; PKCE code_verifier validation.  
**Detection Mechanism:** OIDC callback audit; state reuse detection.  
**Severity:** High  
**Remediation:** Verify state parameter is single-use; test replay with same state value.

### AS-AUTH-002: TOTP Replay Attack

**Scenario ID:** AS-AUTH-002  
**Attack Description:** Attacker replays captured TOTP code within validity window.  
**Prerequisites:** Captured TOTP code.  
**Expected Ferro Behavior:** TOTP codes single-use; counter-based validation; short validity window (30s).  
**Detection Mechanism:** TOTP validation audit; replay detection.  
**Severity:** High  
**Remediation:** Verify TOTP counter tracking; test replay within and outside validity window.

### AS-AUTH-003: Password Reset Token Prediction

**Scenario ID:** AS-AUTH-003  
**Attack Description:** Attacker predicts or brute-forces password reset token.  
**Prerequisites:** Knowledge of reset endpoint.  
**Expected Ferro Behavior:** Cryptographically random reset token; short expiration; single-use.  
**Detection Mechanism:** Reset token audit; failed attempt logging.  
**Severity:** High  
**Remediation:** Verify reset token entropy (UUID v4); test with token prediction attempts.

### AS-AUTH-004: WebAuthn Attestation Bypass

**Scenario ID:** AS-AUTH-004  
**Attack Description:** Attacker bypasses WebAuthn attestation to register unauthorized authenticators.  
**Prerequisites:** WebAuthn feature enabled.  
**Expected Ferro Behavior:** Proper attestation validation; origin verification; challenge-response validation.  
**Detection Mechanism:** WebAuthn registration audit; attestation validation logs.  
**Severity:** High  
**Remediation:** Verify attestation validation; test with forged attestation statements.

---

## OWASP A08:2021 — Software and Data Integrity Failures

### AS-DI-001: WASM Module Tampering

**Scenario ID:** AS-DI-001  
**Attack Description:** Attacker uploads malicious WASM module that escapes sandbox or exfiltrates data.  
**Prerequisites:** Authenticated user with WASM upload access.  
**Expected Ferro Behavior:** WASM magic bytes verification; fuel metering (1B units); memory cap (64MB); timeout (30s); no network/filesystem access.  
**Detection Mechanism:** WASM upload audit; resource limit metrics; sandbox escape attempts.  
**Severity:** Critical  
**Remediation:** Verify WASM sandbox isolation; test with malicious WASM binaries; fuzz wasmtime parser.

### AS-DI-002: Backup File Tampering

**Scenario ID:** AS-DI-002  
**Attack Description:** Attacker modifies backup files to inject malicious data or corrupt restoration.  
**Prerequisites:** Access to backup storage.  
**Expected Ferro Behavior:** Backup integrity verification (hash/checksum); encrypted backups; access controls on backup endpoint.  
**Detection Mechanism:** Backup integrity audit; hash verification on restore.  
**Severity:** High  
**Remediation:** Verify backup integrity checks; test with tampered backup files.

### AS-DI-003: Audit Log Tampering

**Scenario ID:** AS-DI-003  
**Attack Description:** Attacker modifies audit logs to cover tracks.  
**Prerequisites:** Access to audit database.  
**Expected Ferro Behavior:** Chain hash verification (`GET /api/admin/audit-chain`); append-only design; SQLite WAL mode.  
**Detection Mechanism:** Audit chain verification; hash mismatch detection.  
**Severity:** High  
**Remediation:** Verify chain hash implementation; test with log modification attempts.

---

## OWASP A09:2021 — Security Logging and Monitoring Failures

### AS-LOG-001: Missing Audit Logging for Sensitive Operations

**Scenario ID:** AS-LOG-001  
**Attack Description:** Attacker performs sensitive operations without audit trail (user creation, password changes, data export).  
**Prerequisites:** Admin access.  
**Expected Ferro Behavior:** Audit middleware logs all admin operations; request ID tracking; timestamp and user context.  
**Detection Mechanism:** Audit log completeness check; missing entry detection.  
**Severity:** Medium  
**Remediation:** Audit logging for all admin endpoints; verify log completeness.

### AS-LOG-002: Log Injection / Log Forging

**Scenario ID:** AS-LOG-002  
**Attack Description:** Attacker injects malicious content into logs to forge entries or exploit log viewers.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Log content sanitized; newline/控制字符 stripped; structured logging (JSON).  
**Detection Mechanism:** Log injection attempts; structured log validation.  
**Severity:** Medium  
**Remediation:** Verify log sanitization; test with newline injection payloads.

### AS-LOG-003: Insufficient Error Context in 500 Responses

**Scenario ID:** AS-LOG-003  
**Attack Description:** Attacker triggers 500 errors to extract internal paths, stack traces, or configuration details.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Panic handler logs request context to server only; 500 responses return generic error; no stack traces in production.  
**Detection Mechanism:** Error response audit; internal path checks.  
**Severity:** Medium  
**Remediation:** Verify 500 responses are generic; test error responses for internal information leakage.

---

## OWASP A10:2021 — Server-Side Request Forgery (SSRF)

### AS-SSRF-001: SSRF via OIDC Discovery

**Scenario ID:** AS-SSRF-001  
**Attack Description:** Attacker configures OIDC issuer to point to internal services, causing Ferro to make requests to internal endpoints.  
**Prerequisites:** Admin access (OIDC configuration).  
**Expected Ferro Behavior:** OIDC discovery URL validated; internal IP ranges blocked; TLS verification enforced.  
**Detection Mechanism:** OIDC discovery audit; outbound request logging.  
**Severity:** High  
**Remediation:** Block internal IP ranges in OIDC discovery; verify TLS certificate validation.

### AS-SSRF-002: SSRF via Federation WebFinger

**Scenario ID:** AS-SSRF-002  
**Attack Description:** Attacker crafts WebFinger query to cause Ferro to fetch internal URLs.  
**Prerequisites:** Federation enabled.  
**Expected Ferro Behavior:** WebFinger resolution validates URLs; blocks internal IP ranges; rate limits outbound requests.  
**Detection Mechanism:** WebFinger audit; outbound request logging.  
**Severity:** Medium  
**Remediation:** Verify URL validation in WebFinger resolution; test with internal IP targets.

### AS-SSRF-003: SSRF via Webhook URLs

**Scenario ID:** AS-SSRF-003  
**Attack Description:** Admin creates webhook pointing to internal services, causing Ferro to POST sensitive data to internal endpoints.  
**Prerequisites:** Admin access.  
**Expected Ferro Behavior:** Webhook URL validation; internal IP ranges blocked; webhook delivery audit.  
**Detection Mechanism:** Webhook creation audit; delivery logging.  
**Severity:** High  
**Remediation:** Block internal IP ranges in webhook URL validation; test with internal targets.

---

## Additional Attack Scenarios

### AS-DO-001: Resource Exhaustion via Large Upload

**Scenario ID:** AS-DO-001  
**Attack Description:** Attacker uploads extremely large file to exhaust disk space or memory.  
**Prerequisites:** Authenticated user.  
**Expected Ferro Behavior:** Configurable `max_body_size`; disk quota enforcement; chunked upload with size limits.  
**Detection Mechanism:** Upload size metrics; disk usage monitoring.  
**Severity:** Medium  
**Remediation:** Verify body size limits; test with multi-GB uploads.

### AS-DO-002: Connection Exhaustion

**Scenario ID:** AS-DO-002  
**Attack Description:** Attacker opens thousands of concurrent connections to exhaust server resources.  
**Prerequisites:** Network access.  
**Expected Ferro Behavior:** `ConcurrencyLimitLayer` caps in-flight requests; connection timeouts; rate limiting.  
**Detection Mechanism:** Connection count metrics; concurrency limit logs.  
**Severity:** Medium  
**Remediation:** Verify concurrency limits; test with 1000+ simultaneous connections.

### AS-DO-003: XML Entity Expansion (Billion Laughs)

**Scenario ID:** AS-DO-003  
**Attack Description:** Attacker sends XML with nested entity references to exhaust memory (10^9 expansions).  
**Prerequisites:** Authenticated user with WebDAV access.  
**Expected Ferro Behavior:** XML parser disables entity expansion; entity limit enforced; memory cap.  
**Detection Mechanism:** XML parsing metrics; memory usage monitoring.  
**Severity:** High  
**Remediation:** Verify XML parser configuration; test with billion laughs payload.

### AS-FED-001: Federation Activity Spoofing

**Scenario ID:** AS-FED-001  
**Attack Description:** Attacker sends unsigned or forged activity to federation inbox.  
**Prerequisites:** Federation enabled.  
**Expected Ferro Behavior:** HTTP Signatures required on all POST to `/fed/inbox`; actor keyId validated against activity actor; HMAC-SHA256 verification.  
**Detection Mechanism:** Federation signature audit; rejected activity logging.  
**Severity:** High  
**Remediation:** Verify signature validation; test with forged/unsigned activities.

### AS-FED-002: Federation Key Rotation Attack

**Scenario ID:** AS-FED-002  
**Attack Description:** Attacker exploits key rotation window to replay old signatures.  
**Prerequisites:** Federation enabled.  
**Expected Ferro Behavior:** Key rotation with overlap window; old keys invalidated after rotation; signature timestamp validation.  
**Detection Mechanism:** Key rotation audit; replay detection.  
**Severity:** Medium  
**Remediation:** Verify key rotation implementation; test with old key signatures.

---

*Document references: `penetration_test_scope.md`, `SECURITY.md`, OWASP Top 10 2021, CWE/SANS Top 25.*
