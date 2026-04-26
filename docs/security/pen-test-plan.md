# Penetration Test Plan — Ferro v1.0.0-beta.1

**Last updated**: 2026-04-23
**Target**: Ferro storage server
**Scope**: All API endpoints, WebDAV, WOPI, WASM upload, shares

---

## Scope

### In Scope
- Ferro server (all HTTP API endpoints)
- WebDAV operations (PROPFIND, PUT, GET, DELETE, MKCOL, COPY, MOVE, LOCK, UNLOCK, PROPPATCH)
- WOPI endpoints (discovery, checkFileInfo, get/putFile, token issuance)
- API endpoints (auth, shares, workers, policies, audit, search, snapshots)
- Share link access (`/s/:token`)
- Static file serving (`/ui/`)

### Out of Scope
- Tauri desktop application
- Reverse proxy configuration (nginx/Caddy)
- Operating system hardening
- OIDC provider (Keycloak, Auth0, etc.)

### Test Environment
- Docker deployment with simple auth (`--admin-user admin --admin-password testpass`)
- In-memory storage engine (default)
- Rate limiting enabled (default: 10k req/min)

---

## Test Categories

### 1. Authentication Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| AUTH-001 | Brute force Basic auth | Hydra/ffuf with 10k passwords | Blocked by rate limiter after 10k | High |
| AUTH-002 | No auth on protected endpoint | `curl http://host/api/audit` | 401 + `WWW-Authenticate: Basic realm="Ferro"` | Critical |
| AUTH-003 | Invalid base64 in auth header | `curl -H "Authorization: Basic !!!"` | 401 | High |
| AUTH-004 | Empty password accepted | `base64("admin:")` as credentials | 401 (if admin password is not empty) | High |
| AUTH-005 | SQL injection in credentials | `admin' OR '1'='1` as password | 401 (not SQL-based) | Medium |
| AUTH-006 | Path-based auth bypass | `GET /api/test/../../api/config` | 401 (path normalized before auth check) | Critical |
| AUTH-007 | Auth on public paths | `GET /.well-known/ferro` (no auth) | 200 | Medium |
| AUTH-008 | Extra colons in credentials | `base64("admin:pass:extra")` | 401 (split on first `:` only) | Low |
| AUTH-009 | Case-sensitive credentials | `base64("Admin:secret")` | 401 (case-sensitive match) | Low |

### 2. Path Traversal Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| PATH-001 | Upload to `../../etc/passwd` | `PUT /../../etc/cron.d/malicious` | 400 or 403 (sanitize_path rejects `..`) | Critical |
| PATH-002 | PROPFIND with `..` | `PROPFIND /files/../../etc` | 400 or normalized path only | Critical |
| PATH-003 | Null byte injection | `PUT /test%00.txt` | 400 (null byte rejected) | Critical |
| PATH-004 | Unicode normalization | `PUT /test\u202e.exe` | Accepted or sanitized | Medium |
| PATH-005 | Double encoding | `PUT /%252e%252e/test.txt` | 400 or safe path | High |
| PATH-006 | Backslash traversal (Windows) | `PUT /..\..\etc\passwd` | 400 | High |
| PATH-007 | Legitimate dots in filenames | `PUT /file..txt` | 201 Created (allowed) | Medium |
| PATH-008 | Multiple slashes | `PUT ///test///file.txt` | 201 Created (normalized) | Low |
| PATH-009 | Path in Destination header | `COPY /file.txt` with `Destination: http://host/../../etc/evil` | 400 (destination validated) | Critical |

### 3. WebDAV Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| DAV-001 | PROPFIND without auth | `PROPFIND / Depth:1` | 401 | High |
| DAV-002 | PUT large file (>max-body-size) | `PUT /large.bin` (1GB) | 413 Payload Too Large | Medium |
| DAV-003 | DELETE without auth | `DELETE /file.txt` | 401 | High |
| DAV-004 | MKCOL on existing path | `MKCOL /existing-folder` | 409 Conflict (AlreadyExists) | Low |
| DAV-005 | LOCK/UNLOCK support | `LOCK /file.txt` | 200 OK | Medium |
| DAV-006 | LOCK refresh via If header | `LOCK /file.txt` with `If: (<token>)` | 200 OK | Medium |
| DAV-007 | PUT with If-Match (conditional) | `PUT /file.txt` with valid ETag | 204 No Content | Low |
| DAV-008 | PUT with If-None-Match `*` on existing | `PUT /file.txt` with `If-None-Match: *` | 412 Precondition Failed | Low |
| DAV-009 | PROPFIND Depth:infinity DoS | `PROPFIND / Depth:infinity` on deep tree | Bounded by MAX_PROPFIND_DEPTH (100) | Medium |
| DAV-010 | PROPPATCH without auth | `PROPPATCH /file.txt` | 401 | High |
| DAV-011 | MOVE without Destination | `MOVE /file.txt` (no Destination header) | 400 | Low |

### 4. WASM Runtime Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| WASM-001 | Upload non-WASM as worker | `POST /api/workers/upload` with ELF binary | 400 (magic byte check fails) | High |
| WASM-002 | WASM infinite loop | Worker with `loop {}` | Timeout + error (fuel limit) | High |
| WASM-003 | WASM memory bomb | Worker allocating max memory | OOM or fuel limit reached | Medium |
| WASM-004 | WASM accessing host FS | Worker with WASI `fd_read` | Sandbox blocks (no host functions) | High |
| WASM-005 | Upload WASM with path traversal filename | Filename: `../../etc/evil.wasm` | 400 (filename validation) | Critical |
| WASM-006 | Upload WASM with no extension | Filename: `worker` | 400 (extension validation) | Medium |
| WASM-007 | Delete WASM with path traversal | `DELETE /api/workers/modules/../../../etc/passwd` | 400 (filename validation) | Critical |

### 5. Input Validation Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| INJ-001 | XSS in file name | `PUT /"<script>alert(1)</script>.txt"` | 201 (stored safely, escaped in XML) | Medium |
| INJ-002 | XML injection in PROPFIND | Custom PROPFIND with XML entities | Parsed safely (quick-xml, no XXE) | High |
| INJ-003 | CRLF in headers | PUT with CRLF in filename | 400 or sanitized | Medium |
| INJ-004 | Oversized JSON body | `POST /api/shares` with 100MB JSON | 413 | Medium |
| INJ-005 | Malformed JSON body | `POST /api/shares` with `{invalid` | 400 Bad Request | Low |
| INJ-006 | Negative expiry in share | `POST /api/shares` with `expires_in_hours: -1` | 200 (immediate expiry) or 400 | Low |

### 6. Rate Limiting Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| RATE-001 | Normal usage | 100 requests/sec for 10s | All succeed | Medium |
| RATE-002 | Burst attack | 15k requests in 1 second | 429 after 10k limit | High |
| RATE-003 | Rate limit bypass via X-Forwarded-For | Spoof different IPs in header | Still rate limited (uses first IP) | High |
| RATE-004 | Rate limit recovery | Wait 60s after limit | Requests succeed again | Low |
| RATE-005 | Rate limit on public paths | Rapid requests to `/.well-known/ferro` | Rate limited (applies to all) | Medium |

### 7. OIDC Tests (if configured)

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| OIDC-001 | CSRF via state reuse | Reuse old state parameter | Token exchange fails | Critical |
| OIDC-002 | Code interception without PKCE | Intercept authorization code | PKCE challenge fails | Critical |
| OIDC-003 | Malicious redirect_uri | `redirect_uri=https://evil.com` | Server validates against `external_url` | Critical |
| OIDC-004 | Expired token usage | Use token past `exp` claim | 401 Unauthorized | High |
| OIDC-005 | Missing Bearer token | Request without Authorization header | 401 | Medium |
| OIDC-006 | Invalid audience in token | Token with wrong `aud` | 401 (audience validation) | High |

### 8. WOPI Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| WOPI-001 | Access without token | `GET /wopi/files/test.txt` (no access_token) | 401 | High |
| WOPI-002 | Expired token | Token with past expiry | 401 | High |
| WOPI-003 | Tampered token payload | Modify payload, keep signature | 401 (signature mismatch) | High |
| WOPI-004 | Wrong signing secret | Token signed with different secret | 401 | High |
| WOPI-005 | Token reuse across files | Use token for file A to access file B | 401 (path mismatch) | Medium |
| WOPI-006 | Discovery endpoint access | `GET /hosting/discovery` | 200 (public) | Low |

### 9. Share Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| SHARE-001 | Access share without password | Share with password set, no `?password=` | 401 | Medium |
| SHARE-002 | Access expired share | Share past `expires_at` | 410 Gone | Medium |
| SHARE-003 | Exceed download limit | More downloads than `max_downloads` | 410 Gone | Low |
| SHARE-004 | Guess share token | Try random UUIDs | 404 (122-bit entropy) | Low |
| SHARE-005 | Access non-existent share | `GET /s/nonexistent` | 404 | Low |
| SHARE-006 | Create share without auth | `POST /api/shares` (no auth) | 401 | High |

### 10. Security Headers Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| HDR-001 | X-Content-Type-Options present | `curl -I http://host/` | `X-Content-Type-Options: nosniff` | Medium |
| HDR-002 | X-Frame-Options present | `curl -I http://host/` | `X-Frame-Options: DENY` | Medium |
| HDR-003 | Content-Security-Policy present | `curl -I http://host/` | CSP with `default-src 'self'` | High |
| HDR-004 | Referrer-Policy present | `curl -I http://host/` | `Referrer-Policy: strict-origin-when-cross-origin` | Low |
| HDR-005 | Permissions-Policy present | `curl -I http://host/` | Camera/microphone/geolocation blocked | Medium |
| HDR-006 | HSTS absent on HTTP | `curl -I http://host/` | No `Strict-Transport-Security` header | Medium |
| HDR-007 | CORS headers only on cross-origin | `curl -I` without Origin | No `Access-Control-Allow-Origin` | Medium |

### 11. Denial of Service Tests

| ID | Test | Method | Expected | Priority |
|----|------|--------|----------|----------|
| DOS-001 | Slow upload (Slowloris) | Send headers slowly | Server timeout or request rejection | Medium |
| DOS-002 | Many concurrent connections | 10k simultaneous connections | Server remains responsive | Medium |
| DOS-003 | Large PROPFIND depth | `PROPFIND Depth:infinity` | Depth capped at 100 | Low |
| DOS-004 | Rapid share creation | POST /api/shares in loop | Rate limited after 10k | Low |
| DOS-005 | WASM fuel exhaustion | WASM module consuming max fuel | Execution terminated gracefully | Low |

---

## Tools Required

| Tool | Purpose | Notes |
|------|---------|-------|
| `curl` / `httpie` | Manual request testing | Available in most environments |
| `ffuf` | Path/header fuzzing | `ffuf -w wordlist.txt -u http://host/FUZZ` |
| `hydra` | Credential brute force | `hydra -l admin -P wordlist.txt host http-get` |
| Burp Suite Community | Intercepting proxy, repeater | For manual exploration |
| `wasm-tools` | WASM module analysis | Validate and inspect WASM binaries |
| `cargo audit` | Dependency vulnerability check | `cargo audit` |
| `nmap` | Port scanning | Verify only expected ports are open |

## Reporting Format

For each finding, provide:

1. **Finding ID**: Match to test case ID (e.g., AUTH-002)
2. **Severity**: Critical / High / Medium / Low / Informational
3. **Description**: What was found
4. **Evidence**: Request/response pairs, screenshots
5. **Impact**: What an attacker could do
6. **Remediation**: How to fix
7. **References**: OWASP category, CWE ID

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Penetration Tester | | | |
| Security Reviewer | | | |
| Ferro Maintainer | | | |
