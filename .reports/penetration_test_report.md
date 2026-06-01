# Ferro Security Audit & Penetration Test Report

**Date:** 2026-06-01  
**Scope:** Full codebase at `/home/wyatt/dev/src/github.com/WyattAu/ferro/`  
**Auditor:** Automated security audit  
**Version:** 3.0.0

---

## Executive Summary

The Ferro codebase demonstrates solid security fundamentals: parameterized SQL queries, bcrypt password hashing, constant-time comparisons for sensitive operations, content-type magic byte verification, path traversal protection in WebDAV handlers, security headers middleware, and CSRF/account lockout mechanisms. However, several security issues were identified ranging from Critical to Low severity.

**Key findings:**
- 2 Critical: Path traversal in REST API (`normalize_api_path` does not strip `..`), SQL injection in test code
- 2 High: Default permissive Cedar policy, API key comparison not timing-safe, share passwords stored in plaintext
- 3 Medium: CORS wildcard default, unbounded request body read, OIDC token algorithm acceptance from header
- 4 Low: Constant-time eq length mismatch, E2EE envelope key derivation weakness, `Validate_path` completeness, WebAuthn prototype mode

---

## Findings

### F001 — Path Traversal in REST API via `normalize_api_path` (CRITICAL)

**Category:** Path Traversal  
**Severity:** Critical  
**Affected Code:** `crates/server/src/api.rs:1171-1178`  

**Description:** The `normalize_api_path` function used by all REST API endpoints (`put_file`, `get_file`, `delete_file`, `mkdir`, `copy_file`, `move_file_rest`) only strips leading/trailing slashes. It does NOT resolve `..` path traversal components. This means a request to `PUT /api/v1/files/../../../etc/passwd` will result in the path `/../../../etc/passwd` being passed to the storage engine.

In contrast, the WebDAV handler (`crates/server/src/webdav.rs:22-47`) properly checks for `..` and `.` components via `sanitize_path()` before passing to `normalize_path()`.

**Impact:** An attacker can read/write/delete files outside the intended storage directory.

**PoC:**
```
PUT /api/v1/files/../../etc/cron.d/evil HTTP/1.1
Authorization: Basic YWRtaW46c2VjcmV0
Content-Type: text/plain

* * * * * * * * * * * * * *
```

**Recommended Fix:** Apply the same `sanitize_path()` logic (checking for `..` and `.` components) to `normalize_api_path`, or replace it with `common::path::normalize_path`.

---

### F002 — SQL Injection in Database Test Code (CRITICAL)

**Category:** SQL Injection  
**Severity:** Critical  
**Affected Code:** `crates/server/src/db.rs:135-136`  

**Description:** A test function constructs a SQL query using string formatting:
```rust
.query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| row.get(0))
```

The `table` variable comes from a hardcoded array of trusted table names, so this is not exploitable in the current test context. However, if this pattern were copied into production code with user-controlled input, it would be a direct SQL injection vector.

**Impact:** Currently test-only — no exploitable production path. But the pattern itself is dangerous.

**Recommended Fix:** Since the table names are from a trusted constant array, this is acceptable in test code. No immediate action needed, but enforce via CI that `format!` is never used in SQL outside tests.

---

### F003 — Default Cedar Policy is Permissive (HIGH)

**Category:** Authorization Bypass  
**Severity:** High  
**Affected Code:** `crates/auth/src/cedar.rs:28-37`  

**Description:** The default Cedar policy when no custom policies are loaded permits ALL actions (`read`, `write`, `delete`, `list`, `admin`) for ALL principals on ALL resources. This means that if an administrator deploys Ferro without configuring Cedar policies and without enabling OIDC authentication, the default configuration allows unrestricted anonymous access to everything.

**Impact:** Users who deploy without Cedar configuration and without OIDC get no access control. The `auth_middleware` in `crates/auth/src/oidc.rs:357-358` already sets `Claims::anonymous()` when OIDC is not configured, and the Cedar middleware passes through all requests if not configured (`cedar.rs:251-254`). The WebDAV middleware (`webdav.rs`) and REST API handlers do not check authorization independently.

**Recommended Fix:** The default policy should either deny all actions by default, or issue a startup warning when running without authentication and authorization. At minimum, log a WARNING at startup if both OIDC and Cedar are unconfigured.

---

### F004 — API Key Hash Comparison is Not Timing-Safe (HIGH)

**Category:** Timing Attack  
**Severity:** High  
**Affected Code:** `crates/auth/src/api_keys.rs:459-465`  

**Description:** API key authentication works by hashing the provided raw key with SHA-256 and then looking up the hash in a `DashMap`. The `DashMap::get()` call uses standard string comparison (Rust's default `String::eq`), which is NOT constant-time. An attacker with network access can measure response times to gradually recover the hash byte-by-byte.

```rust
let id = self
    .hash_index
    .get(&hash)  // non-constant-time lookup
    .map(|r| r.value().clone())
```

**Impact:** A sophisticated network attacker could extract API key hashes through timing analysis, given sufficient requests and low network jitter.

**Recommended Fix:** Precompute a 32-byte hex hash (which is already fixed-length) and use `subtle::ConstantTimeEq` to compare hashes. Since the hash is always 64 hex characters (256-bit), this eliminates length-based timing.

---

### F005 — Share Passwords Stored in Plaintext (HIGH)

**Category:** Insecure Data Storage  
**Severity:** High  
**Affected Code:** `crates/server/src/shares.rs:26-29`  

**Description:** Share link passwords are stored as plaintext strings in the `ShareLink` struct. While they're compared using constant-time comparison (good), they're persisted to SQLite as plaintext in the `password` column. If the database is compromised, all share passwords are immediately recoverable.

```rust
pub struct ShareLink {
    pub password: Option<String>,  // plaintext!
```

**Impact:** Database compromise exposes all share link passwords.

**Recommended Fix:** Hash share passwords with bcrypt (like user passwords) before storage. Compare hashes during authentication using `bcrypt::verify`.

---

### F006 — CORS Default is Wildcard `*` (MEDIUM)

**Category:** Security Misconfiguration  
**Severity:** Medium  
**Affected Code:** `crates/server/src/config.rs:215`  

**Description:** The default `cors_allowed_origins` is `"*"`, which allows requests from any origin. This is a common developer convenience default but is inappropriate for production deployments handling sensitive data.

**Impact:** Any website can make cross-origin requests to the Ferro API, potentially stealing data if a user is authenticated.

**Recommended Fix:** Default to `"self"` or require explicit configuration in production.

---

### F007 — Unbounded Body Read with `usize::MAX` (MEDIUM)

**Category:** Denial of Service  
**Severity:** Medium  
**Affected Code:** `crates/server/src/api.rs:180`  

**Description:** The `auth_change_password` endpoint reads the request body with `axum::body::to_bytes(body, usize::MAX)`. This allows an attacker to send an arbitrarily large request body, potentially exhausting server memory.

```rust
let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
```

**Impact:** An attacker can send a multi-gigabyte request body to exhaust memory and cause a denial of service.

**Recommended Fix:** Use `state.max_body_size` as the limit, or a reasonable maximum like `1024 * 1024` (1 MB) for password change endpoints.

---

### F008 — OIDC Token Validation Accepts Algorithm from JWT Header (MEDIUM)

**Category:** Authentication Bypass  
**Severity:** Medium  
**Affected Code:** `crates/auth/src/oidc.rs:224`  

**Description:** The OIDC token validation constructs a `Validation` using the algorithm from the JWT header: `Validation::new(header.alg)`. This is the standard behavior of the `jsonwebtoken` crate, but it means an attacker could forge a token with `alg: none` and have it parsed successfully (the claims would be extracted without signature verification). While `decode` would fail for `none` in most cases because the `DecodingKey` wouldn't match, using `RS256` headers with `HS256` key confusion attacks (CVE-2016-10555 style) could be possible.

The code does validate issuer and audience, which provides some defense. The JWKS-based key lookup also mitigates key confusion.

**Impact:** Algorithm confusion or `none` algorithm attacks could bypass JWT signature verification in certain configurations.

**Recommended Fix:** Explicitly validate that the algorithm is one of the expected algorithms (e.g., `RS256`, `ES256`) and reject `none`.

---

### F009 — Constant-Time Equality Returns `false` for Different-Length Inputs (LOW)

**Category:** Cryptography  
**Severity:** Low  
**Affected Code:** `crates/crypto/src/ring_provider.rs:99-108`  

**Description:** The `constant_time_eq` implementation returns `false` immediately if the byte slices have different lengths. This leaks length information through timing.

```rust
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;  // early return leaks length
    }
    // ... constant-time comparison for same-length data
}
```

**Impact:** Minor timing leak about the length of compared values. For password/API key comparisons where lengths are typically fixed (hashes are always the same length), this is acceptable.

**Recommended Fix:** For production, pad both inputs to the same length before comparison.

---

### F010 — E2EE Envelope Key Derivation Uses Only Public Key (LOW)

**Category:** Cryptographic Weakness  
**Severity:** Low  
**Affected Code:** `crates/e2ee/src/envelope.rs:19-28`  

**Description:** The envelope key derivation uses HKDF with only the recipient's public key as the input key material:
```rust
let hk = Hkdf::<Sha256>::new(None, recipient_public_key);
```

HKDF with `None` salt and only a public key as IKM does not provide the security properties of Diffie-Hellman key agreement. A proper hybrid encryption scheme should use ECDH to derive a shared secret from the sender's private key and recipient's public key.

**Impact:** The envelope encryption scheme is not quantum-resistant and may be weaker than expected. An attacker who knows the recipient's public key (which is public by nature) could potentially derive the envelope key.

**Recommended Fix:** Implement proper ECDH key agreement (e.g., X25519) between sender private key and recipient public key to derive the envelope key.

---

### F011 — WebAuthn Prototype Mode Without Full Verification (LOW)

**Category:** Authentication  
**Severity:** Low  
**Affected Code:** `crates/auth/src/webauthn.rs:14-21`  

**Description:** The WebAuthn module operates in "prototype mode" by default (without the `webauthn-rs` feature). In this mode, it verifies challenge and origin but does NOT verify the actual CTAP2 cryptographic signatures from the authenticator. The module documentation explicitly warns about this.

**Impact:** If deployed in production without the `webauthn-rs` feature, WebAuthn credentials can be forged by any client that knows the expected challenge and origin.

**Recommended Fix:** Add a startup warning when WebAuthn is enabled without the `webauthn-rs` feature. Consider refusing to start with WebAuthn enabled in prototype mode.

---

### F012 — Share Upload Path Not Sanitized for Traversal (MEDIUM)

**Category:** Path Traversal  
**Severity:** Medium  
**Affected Code:** `crates/server/src/shares.rs:578`  

**Description:** The share upload handler constructs the target path by joining the share's `link.path` with the sanitized filename:
```rust
let target_path = format!("{}/{}", link.path.trim_end_matches('/'), file_name);
```

While `file_name` is sanitized via `crate::shares_ext::sanitize_filename`, the `link.path` comes from the share creation request, which was not sanitized at creation time. If a share were created with `path: "../../tmp"`, uploads would be written outside the intended directory.

**Impact:** A share created with a path traversal could write files outside the storage directory.

**Recommended Fix:** Validate `link.path` at share creation time using `sanitize_path()`.

---

---

## Overall Risk Assessment

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 2 | 1 requires immediate fix (F001) |
| High | 3 | Address in next release |
| Medium | 4 | Address within quarter |
| Low | 4 | Address opportunistically |

**Overall risk level:** HIGH — primarily due to the REST API path traversal (F001) which affects all file operations through the REST interface.

---

## Remediation Priority

1. **Immediate:** Fix F001 (REST API path traversal) — replace `normalize_api_path` with path sanitization that rejects `..` and `.` components
2. **Short-term:** Fix F005 (plaintext share passwords), F004 (API key timing), F007 (unbounded body read)
3. **Medium-term:** F003 (default Cedar policy), F006 (CORS default), F008 (OIDC algorithm validation), F012 (share path traversal)
4. **Long-term:** F009 (constant-time length), F010 (E2EE key derivation), F011 (WebAuthn prototype warning)

---

## Positive Security Controls Found

The following security controls are properly implemented and should be maintained:

- All production SQL queries use parameterized statements (`rusqlite::params![]`)
- Password hashing uses bcrypt with default cost
- Basic auth comparison uses `subtle::ConstantTimeEq`
- Share password comparison uses `subtle::ConstantTimeEq`
- CSRF token generation and verification are cryptographically sound
- Content-Type magic byte verification on uploads
- WebDAV path sanitization properly blocks `..` and `.` components
- Security headers middleware (CSP, HSTS, X-Frame-Options, etc.)
- Account lockout after 10 failed attempts (15 min cooldown)
- Login rate limiting (5 attempts/minute/IP)
- Default password detection and enforcement
- File name validation (reserved names, control chars, length limits)
- Sensitive fields redacted in Debug output
- PKCE for OIDC code flow with one-time-use state
- API keys use 256-bit entropy with SHA-256 hashing
- E2EE uses AES-256-GCM with random nonces and HKDF key derivation
- WASM plugins run in sandboxed workers with capability declarations

---

## F013: Write API Key Permission Allows Admin Actions

| Field | Value |
|-------|-------|
| **Severity** | Medium |
| **Category** | Authorization |
| **File** | `crates/auth/src/api_keys.rs:76` |
| **Status** | Open |

### Description

The `ApiKeyPermission::Write` variant matches `"admin"` in `allows_action()`, granting
full administrative access to any key with write-level permissions. Users with only
write intent can perform admin operations.

### Affected Code

```rust
Self::Write => matches!(action, "read" | "write" | "delete" | "list" | "admin"),
```

### Impact

Write-permission API keys can perform admin operations (LOCK, UNLOCK, user management),
violating the principle of least privilege.

### Recommendation

Remove `"admin"` from the Write match arm:

```rust
Self::Write => matches!(action, "read" | "write" | "delete" | "list"),
```
