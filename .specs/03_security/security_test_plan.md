# Security Test Plan — Ferro System

**Document ID:** FERRO-SEC-TP-001
**Date:** 2026-04-18
**Status:** Draft
**Traceability:** Links to STRIDE threat model (FERRO-SEC-TM-001) and requirements (FERRO-REQ-001)

---

## Test Execution Framework

| Aspect | Specification |
|--------|--------------|
| **Test Runner** | `cargo test` with integration test feature flags |
| **Test Isolation** | InMemory object_store backend; in-memory session store; ephemeral Cedar policy set |
| **Test Data** | Deterministic fixtures: known JWTs, known SHA-256 hashes, pre-crafted XML payloads |
| **Classification** | Each test tagged with severity (Critical/High/Medium) and threat ID cross-reference |
| **CI Integration** | Security test suite runs on every PR; full suite nightly |

---

## 1. Authentication Tests

| Test ID | Description | Input | Expected Outcome | Severity | Threat Ref |
|---------|-------------|-------|-----------------|----------|------------|
| SEC-AUTH-001 | Valid token acceptance | JWT signed by trusted JWKS key, valid claims (sub, iss, aud, exp, iat, nonce) | 200 OK, Claims extracted, principal set in request extensions | Critical | TH-SPOOF-001 |
| SEC-AUTH-002 | Expired token rejection | JWT with `exp` in the past (beyond clock skew) | 401 Unauthorized, `TokenExpired` error, audit log entry | Critical | TH-SPOOF-001 |
| SEC-AUTH-003 | Invalid signature rejection | JWT signed by unknown key (not in JWKS) or corrupted signature bytes | 401 Unauthorized, `SignatureInvalid` error, JWKS refresh attempted | Critical | TH-SPOOF-001 |
| SEC-AUTH-004 | Algorithm confusion attack (`none`) | JWT with `alg: none` header, unsigned payload | 401 Unauthorized, `AlgorithmNotAllowed("none")` error | Critical | TH-SPOOF-001 |
| SEC-AUTH-005 | Algorithm confusion attack (HS256→RS256) | JWT with `alg: HS256` but payload signed with RSA public key as HMAC secret | 401 Unauthorized, `AlgorithmNotAllowed` (HS256 not in allowlist) | Critical | TH-SPOOF-001 |
| SEC-AUTH-006 | Token replay attack | Previously valid token reused after nonce consumed by session | 401 Unauthorized, `NonceMismatch` error | High | TH-SPOOF-001, TH-DISCLOSE-004 |
| SEC-AUTH-007 | Missing `sub` claim | JWT with valid signature but no `sub` claim | 401 Unauthorized, `InvalidSubject` error | High | TH-SPOOF-001 |
| SEC-AUTH-008 | Missing `iss` claim | JWT with valid signature but no `iss` claim | 401 Unauthorized, `InvalidIssuer` error | Critical | TH-SPOOF-002 |
| SEC-AUTH-009 | Wrong `iss` claim | JWT with `iss: "https://evil.com"` instead of configured issuer | 401 Unauthorized, `InvalidIssuer { expected, actual }` | Critical | TH-SPOOF-002 |
| SEC-AUTH-010 | Wrong `aud` claim | JWT with `aud: "wrong-client"` | 401 Unauthorized, `InvalidAudience` error | High | TH-SPOOF-001 |
| SEC-AUTH-011 | Token TTL exceeded | JWT with `iat` far in the past (TTL > max_token_ttl_seconds) | 401 Unauthorized, `TokenTtlExceeded` error | High | TH-SPOOF-001 |
| SEC-AUTH-012 | Token not yet valid | JWT with `nbf` in the future (beyond clock skew) | 401 Unauthorized, `TokenNotYetValid` error | Medium | TH-SPOOF-001 |
| SEC-AUTH-013 | Unknown `kid` triggers JWKS refresh | JWT with `kid: "unknown-key-id"` | OidcValidator calls `refresh_jwks()`, then validates against new key | High | TH-SPOOF-001 |
| SEC-AUTH-014 | Refresh token rotation | Use refresh token once → success; reuse same token → rejection | First use: new token pair issued; second use: old token invalidated, 401 returned | Critical | TH-DISCLOSE-004 |
| SEC-AUTH-015 | Session revocation on logout | Create session → logout → use session ID | 401 Unauthorized, session not found (revoked) | High | TH-DISCLOSE-004 |

---

## 2. Authorization Tests

| Test ID | Description | Input | Expected Outcome | Severity | Threat Ref |
|---------|-------------|-------|-----------------|----------|------------|
| SEC-AUTHZ-001 | Deny-by-default verification | Empty policy set, any principal/action/resource tuple | Deny decision | Critical | TH-ELEVATE-001 |
| SEC-AUTHZ-002 | Permit policy matching | Policy: `permit(principal, ==User::"alice", action, ==Action::"FileRead", resource)`; request from User::"alice" | Allow decision | Critical | TH-ELEVATE-001 |
| SEC-AUTHZ-003 | Deny policy override | Permit policy for alice + Forbid policy for alice on same action/resource | Deny decision (forbid wins) | Critical | TH-ELEVATE-001 |
| SEC-AUTHZ-004 | Complex condition evaluation | Policy with attribute conditions: `resource.tag == "public" && principal.groups.contains("engineering")` | Allow/Deny based on correct condition evaluation | High | TH-ELEVATE-001 |
| SEC-AUTHZ-005 | Policy hot-reload without downtime | Load policy set v1 → evaluate → load policy set v2 (atomic swap) → evaluate with v2 rules | Zero-downtime transition; requests during swap see consistent policy set | High | TH-TAMPER-003 |
| SEC-AUTHZ-006 | Invalid policy rejected | Policy with syntax error or schema violation | `PolicyLoadError`; old policy set remains active | High | TH-TAMPER-003 |
| SEC-AUTHZ-007 | Policy size limit enforcement | Policy exceeding 256 KiB (DC-AUTH-CEDAR-SIZE-001) | `PolicyLoadError` with size limit message | Medium | TH-TAMPER-003 |
| SEC-AUTHZ-008 | Policy count limit enforcement | 10,001 policies loaded | `PolicyLoadError` with count limit message | Medium | TH-TAMPER-003 |
| SEC-AUTHZ-009 | Attribute-based access control | Policy: `permit(principal, action, resource) when { resource.owner == principal }` | Owner can access own files; non-owner denied | Critical | TH-DISCLOSE-001 |
| SEC-AUTHZ-010 | Authorization decision audit logging | Make authorized and unauthorized requests | Every decision logged with principal, action, resource, decision, determining policies | High | TH-REPUD-001, TH-ELEVATE-001 |
| SEC-AUTHZ-011 | No route bypasses middleware | Request to arbitrary path without auth header | 401 Unauthorized (middleware applies to all routes) | Critical | TH-ELEVATE-001 |

---

## 3. WebDAV Security Tests

| Test ID | Description | Input | Expected Outcome | Severity | Threat Ref |
|---------|-------------|-------|-----------------|----------|------------|
| SEC-WEBDAV-001 | Path traversal prevention (`../`) | PUT to `/files/../../../etc/passwd` | 400 Bad Request, `InvalidPath` error | Critical | TH-DISCLOSE-001 |
| SEC-WEBDAV-002 | Path traversal prevention (encoded) | PUT to `/files/%2e%2e/%2e%2e/etc/passwd` | 400 Bad Request, `InvalidPath` error | Critical | TH-DISCLOSE-001 |
| SEC-WEBDAV-003 | Path traversal prevention (double encoding) | PUT to `/files/%252e%252e/etc/passwd` | 400 Bad Request, `InvalidPath` error | Critical | TH-DISCLOSE-001 |
| SEC-WEBDAV-004 | XML entity expansion (billion laughs) | PROPFIND with deeply nested XML entity expansion | 413 Payload Too Large or 400 Bad Request (entity limit exceeded) | High | TH-TAMPER-004 |
| SEC-WEBDAV-005 | XXE prevention | PROPFIND with external entity reference (`<!ENTITY xxe SYSTEM "/etc/passwd">`) | 400 Bad Request (DTD/entity processing disabled) | High | TH-TAMPER-004 |
| SEC-WEBDAV-006 | Large request body rejection | PROPFIND with body exceeding 1 MiB (DC-XML-002) | 413 Payload Too Large | Medium | TH-TAMPER-004, TH-DOS-002 |
| SEC-WEBDAV-007 | Lock timeout enforcement | LOCK with Timeout: Second-5, wait 6 seconds, attempt write | Lock expired, write succeeds (no 423) | Medium | TH-DOS-004 |
| SEC-WEBDAV-008 | Concurrent lock conflict resolution | Acquire exclusive lock → second client attempts lock on same resource | Second client receives 423 Locked | High | TH-DOS-004 |
| SEC-WEBDAV-009 | Max locks per principal | Principal acquires `max_locks_per_principal` + 1 locks | Lock attempt fails with error when limit exceeded | Medium | TH-DOS-004 |
| SEC-WEBDAV-010 | Depth:infinity on huge tree | PROPFIND Depth:infinity on directory with 10,000 items | Response completes within timeout; streaming XML; no OOM | High | TH-DOS-002 |
| SEC-WEBDAV-011 | Malformed XML rejection | PROPFIND with invalid XML (unclosed tags) | 400 Bad Request | Low | TH-TAMPER-004 |
| SEC-WEBDAV-012 | Destination header traversal (COPY/MOVE) | COPY with `Destination: /files/../../../etc/` | 400 Bad Request, `InvalidPath` | Critical | TH-DISCLOSE-001 |

---

## 4. Storage Security Tests

| Test ID | Description | Input | Expected Outcome | Severity | Threat Ref |
|---------|-------------|-------|-----------------|----------|------------|
| SEC-STOR-001 | Content integrity verification (SHA-256) | PUT content → GET content → verify SHA-256 matches | `get_content` returns Ok only if SHA-256 matches stored hash | Critical | TH-TAMPER-002 |
| SEC-STOR-002 | Corrupted content detection | PUT content → directly modify backend bytes → GET | `Err(IntegrityFailure)` with audit log entry | Critical | TH-TAMPER-002 |
| SEC-STOR-003 | Pre-signed URL expiration | Generate URL with 1s TTL → wait 2s → use URL | Cloud backend returns 403 (expired) | High | TH-SPOOF-003, TH-DISCLOSE-002 |
| SEC-STOR-004 | Pre-signed URL method scoping | Generate PUT URL → attempt GET | Cloud backend returns 405 or 403 | High | TH-SPOOF-003 |
| SEC-STOR-005 | Pre-signed URL path scoping | Generate URL for `/files/doc.txt` → attempt access to `/files/secret.txt` | Cloud backend returns 403 (wrong path) | High | TH-SPOOF-003 |
| SEC-STOR-006 | Pre-signed URL single-use | Use URL once → attempt reuse | Second use rejected (if single-use mode enabled) | High | TH-DISCLOSE-002 |
| SEC-STOR-007 | Concurrent dedup race condition | 10 concurrent PUTs of identical content | Exactly one physical copy stored; all paths resolve to same hash | High | TH-TAMPER-002 |
| SEC-STOR-008 | Unauthorized pre-signed URL generation | Request pre-signed URL without FileRead/FileWrite permission | 401/403 Unauthorized; no URL generated | Critical | TH-SPOOF-003, TH-DISCLOSE-001 |
| SEC-STOR-009 | Atomic write under failure | PUT content → simulate network failure mid-upload | No partial file exists; previous state unchanged | Critical | TH-TAMPER-002 |
| SEC-STOR-010 | Path traversal in storage API | `put("/../../../etc/passwd", content)` | `Err(InvalidPath)` — PRE-STORAGE-001 | Critical | TH-DISCLOSE-001 |

---

## 5. WASM Security Tests (Phase 5)

| Test ID | Description | Input | Expected Outcome | Severity | Threat Ref |
|---------|-------------|-------|-----------------|----------|------------|
| SEC-WASM-001 | Sandbox escape: filesystem access | WASM plugin attempts `fd_read` on `/etc/passwd` without granted capability | WASI error: capability not granted; plugin receives error; server unaffected | Critical | TH-ELEVATE-003 |
| SEC-WASM-002 | Sandbox escape: network access | WASM plugin attempts outbound HTTP connection without granted capability | WASI error: capability not granted; connection refused | Critical | TH-ELEVATE-003 |
| SEC-WASM-003 | Sandbox escape: environment variables | WASM plugin attempts `environ_get` without granted capability | WASI error: capability not granted | High | TH-ELEVATE-003 |
| SEC-WASM-004 | Resource exhaustion: CPU | WASM plugin with infinite loop | Plugin terminated after fuel limit; server remains responsive | High | TH-DOS-003 |
| SEC-WASM-005 | Resource exhaustion: memory | WASM plugin allocates large arrays exceeding memory limit | Plugin terminated at memory limit; server remains responsive | High | TH-DOS-003 |
| SEC-WASM-006 | Resource exhaustion: time | WASM plugin with sleep/blocking operation | Plugin terminated after wall-clock timeout; server processes other requests | High | TH-DOS-003 |
| SEC-WASM-007 | Unauthorized file access (granted subset) | Plugin granted access to `/data/public/*` → attempts access to `/data/private/secret.txt` | Access denied; capability scope enforced | Critical | TH-ELEVATE-003 |
| SEC-WASM-008 | Server stability after plugin crash | Load plugin that intentionally crashes (trap) | Plugin terminated; server continues processing other requests; new plugin instances can be loaded | High | TH-DOS-003 |
| SEC-WASM-009 | Concurrent plugin isolation | Two plugins running simultaneously; one crashes or exhausts resources | Other plugin unaffected; independent execution | High | TH-DOS-003 |
| SEC-WASM-010 | Plugin input validation | Plugin receives malformed file metadata | Plugin receives validated, sanitized input; no injection from metadata | Medium | TH-TAMPER-004 |

---

## 6. Enterprise / Audit Log Tests

| Test ID | Description | Input | Expected Outcome | Severity | Threat Ref |
|---------|-------------|-------|-----------------|----------|------------|
| SEC-AUDIT-001 | Audit log completeness | Perform PUT, GET, DELETE, PROPFIND, LOCK, UNLOCK | All operations recorded with timestamp, principal, action, resource, result, client IP | Critical | TH-REPUD-001 |
| SEC-AUDIT-002 | Audit log immutability | Write audit entries → attempt to modify or delete an entry | Modification detected via hash chain; deletion rejected | Critical | TH-REPUD-001 |
| SEC-AUDIT-003 | Audit log hash chain integrity | Verify hash chain from first entry to latest | Each entry's hash matches H(prev_hash + entry_data) | High | TH-TAMPER-001 |
| SEC-AUDIT-004 | Policy change audit | Admin creates, modifies, deletes a Cedar policy | All changes logged with admin identity, timestamp, full policy diff | High | TH-REPUD-002 |
| SEC-AUDIT-005 | Token validation failure logging | Send invalid/expired/forged tokens | Every failure logged with IP, token header info (kid, alg), failure reason | High | TH-SPOOF-001 |

---

## Critical Test Cases Summary

The following tests are classified as **Critical** — they must pass before any release:

| Test ID | Category | Threat Addressed |
|---------|----------|-----------------|
| SEC-AUTH-001 | Authentication | TH-SPOOF-001 |
| SEC-AUTH-002 | Authentication | TH-SPOOF-001 |
| SEC-AUTH-003 | Authentication | TH-SPOOF-001 |
| SEC-AUTH-004 | Authentication | TH-SPOOF-001 |
| SEC-AUTH-005 | Authentication | TH-SPOOF-001 |
| SEC-AUTH-008 | Authentication | TH-SPOOF-002 |
| SEC-AUTH-014 | Authentication | TH-DISCLOSE-004 |
| SEC-AUTHZ-001 | Authorization | TH-ELEVATE-001 |
| SEC-AUTHZ-002 | Authorization | TH-ELEVATE-001 |
| SEC-AUTHZ-003 | Authorization | TH-ELEVATE-001 |
| SEC-AUTHZ-009 | Authorization | TH-DISCLOSE-001 |
| SEC-AUTHZ-011 | Authorization | TH-ELEVATE-001 |
| SEC-WEBDAV-001 | WebDAV | TH-DISCLOSE-001 |
| SEC-WEBDAV-002 | WebDAV | TH-DISCLOSE-001 |
| SEC-WEBDAV-003 | WebDAV | TH-DISCLOSE-001 |
| SEC-WEBDAV-012 | WebDAV | TH-DISCLOSE-001 |
| SEC-STOR-001 | Storage | TH-TAMPER-002 |
| SEC-STOR-002 | Storage | TH-TAMPER-002 |
| SEC-STOR-008 | Storage | TH-DISCLOSE-001 |
| SEC-STOR-009 | Storage | TH-TAMPER-002 |
| SEC-STOR-010 | Storage | TH-DISCLOSE-001 |
| SEC-WASM-001 | WASM | TH-ELEVATE-003 |
| SEC-WASM-002 | WASM | TH-ELEVATE-003 |
| SEC-WASM-007 | WASM | TH-ELEVATE-003 |
| SEC-AUDIT-001 | Audit | TH-REPUD-001 |
| SEC-AUDIT-002 | Audit | TH-REPUD-001 |

**Total Critical Tests: 26**

---

## Test Execution Priority

| Priority | Phase | Tests | Gate |
|----------|-------|-------|------|
| P0 | Phase 1 (Core) | SEC-STOR-001..010, SEC-WEBDAV-001..012 | Must pass before Phase 1 release |
| P1 | Phase 2 (Metadata/Auth) | SEC-AUTH-001..015, SEC-AUTHZ-001..011, SEC-AUDIT-001..005 | Must pass before Phase 2 release |
| P2 | Phase 5 (WASM) | SEC-WASM-001..010 | Must pass before Phase 5 release |
