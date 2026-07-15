# NIST SP 800-53 Rev. 5 Control Mapping

**Project:** Ferro File Server
**Version:** 3.x
**Date:** 2026-07-15
**Status:** Living Document

This document maps Ferro's security controls to NIST SP 800-53 Rev. 5 control families. For each control, implementation status, evidence, gaps, and remediation plans are documented.

---

## AC — Access Control

### AC-2: Account Management

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| OIDC accounts | `crates/auth/src/oidc.rs:48` — `OidcValidator` manages OIDC sessions, PKCE flow, token refresh |
| TOTP enrollment | `crates/auth/src/totp.rs:36` — TOTP code generation/verification with RFC 6238 compliance |
| SAML provisioning | `crates/auth/src/saml.rs:168` — `parse_saml_response` extracts NameID, groups, attributes from IdP |
| Local user store | `crates/auth/src/users.rs:39` — `User` struct with `role`, `status`, `storage_quota` fields |
| User CRUD | `crates/auth/src/users.rs:312` — `insert_or_replace_user`, `list_users`, `update_user` |

**Gaps:** None.

---

### AC-3: Access Enforcement

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Cedar policy engine | `crates/auth/src/rbac.rs:66` — `generate_role_policies` produces Cedar policy set from role assignments |
| RBAC presets | `crates/auth/src/rbac.rs:29` — `system_roles()` defines Admin, User, ReadOnly presets |
| Auth middleware | `crates/auth/src/oidc.rs:347` — `auth_middleware` validates Bearer tokens, enforces public/private path split |
| Simple auth guard | `crates/auth/src/simple_auth.rs:54` — Role-based permission mapping for API key access |

**Gaps:** None.

---

### AC-4: Information Flow Enforcement

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Path validation | `crates/common/src/path.rs:67` — `validate_path` rejects `..` traversal, empty strings, whitespace |
| WORM enforcement | `crates/server-compliance/src/worm.rs:206` — `is_worm_protected` blocks mutation of protected paths |
| WebDAV handlers | `crates/server-webdav-core/src/handlers/put.rs:45` — `validate_path` called before every PUT/MOVE/COPY |

**Gaps:** None.

---

### AC-6: Least Privilege

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Role hierarchy | `crates/auth/src/users.rs:87` — `is_admin()`, `can_read_write()` methods enforce role boundaries |
| Cedar policies | `crates/auth/src/rbac.rs:99` — Admin gets `*`, User gets read+write, ReadOnly gets read-only |
| Admin-only endpoints | `crates/audit-log/src/audit_log.rs:139` — `record` requires prior authorization; admin API gates audit access |
| Guest limitation | `crates/auth/src/oidc.rs:375` — Anonymous claims when no OIDC configured; no admin privileges |

**Gaps:** None.

---

### AC-7: Unsuccessful Login Attempts

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Auth attempt tracker | `crates/server-security/src/security.rs:498` — `auth_attempt_tracker().is_locked_out()` checked before auth |
| Failure recording | `crates/server-security/src/security.rs:533` — `record_failure` called on failed login |
| Success recording | `crates/server-security/src/security.rs:535` — `record_success` called after successful login |
| Login rate limiter | `crates/server-security/src/security.rs:518` — `login_rate_limiter().check()` enforces per-IP rate limit |
| Tenant rate limits | `crates/server/src/tenant_rate_limit_api.rs:31` — Per-tenant rate limit configuration and enforcement |

**Gaps:** None.

---

### AC-11: Session Lock

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| OIDC token expiry | `crates/auth/src/oidc.rs:219` — JWT validation checks `exp` claim via `Validation::new()` |
| PKCE session TTL | `crates/auth/src/oidc.rs:72` — PKCE sessions expire after 10 minutes (`Duration::from_mins(10)`) |
| JWKS cache TTL | `crates/auth/src/oidc.rs:58` — JWKS cache expires after 24 hours (`Duration::from_hours(24)`) |
| Refresh token rotation | `crates/auth/src/oidc.rs:144` — `refresh_access_token` supports token rotation on refresh |

**Gaps:** None.

---

### AC-17: Remote Access

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| TLS 1.3 | `SECURITY.md:77` — TLS 1.3 enforced via rustls for all transport |
| API key auth | `crates/auth/src/simple_auth.rs:54` — API keys with permission-level role mapping |
| Rate limiting | `crates/server-config/src/lib.rs:44` — `rate_limit_burst` and `rate_limit_refill` configurable per-deployment |

**Gaps:** None.

---

## AU — Audit and Accountability

### AU-2: Audit Events

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Audit action types | `crates/audit-log/src/audit_log.rs:14` — `AuditAction` enum: Login, Logout, FileCreate, FileRead, FileUpdate, FileDelete, FileShare, FileDownload, UserCreate, UserUpdate, UserDelete, PermissionChange, ConfigChange, Custom |
| Resource types | `crates/audit-log/src/audit_log.rs:40` — `ResourceType` enum: User, File, Folder, Share, Permission, Config, ApiKey, Custom |
| Middleware integration | `SECURITY.md:298` — Chain hash verification on `GET /api/admin/audit-chain` |

**Gaps:** None.

---

### AU-3: Content of Audit Records

**Status:** Implemented

| Field | Evidence |
|-------|----------|
| Timestamp | `crates/audit-log/src/audit_log.rs:55` — `timestamp: DateTime<Utc>` (RFC 3339) |
| Action | `crates/audit-log/src/audit_log.rs:56` — `action: AuditAction` |
| Actor ID | `crates/audit-log/src/audit_log.rs:57` — `actor_id: String` |
| Resource | `crates/audit-log/src/audit_log.rs:58-60` — `resource_type`, `resource_id` |
| Details | `crates/audit-log/src/audit_log.rs:61` — `details: HashMap<String, serde_json::Value>` |
| IP Address | `crates/audit-log/src/audit_log.rs:62` — `ip_address: Option<String>` |
| User Agent | `crates/audit-log/src/audit_log.rs:63` — `user_agent: Option<String>` |
| Chain hash | `crates/audit-log/src/audit_log.rs:64-65` — `previous_hash`, `hash` for tamper detection |

**Gaps:** None.

---

### AU-6: Audit Record Review

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Query API | `crates/audit-log/src/audit_log.rs:176` — `query` supports filtering by action, actor, resource, time range |
| Export formats | `crates/audit-log/src/audit_log.rs:279` — `export` supports JSON and CSV formats |
| Chain verification | `crates/audit-log/src/audit_log.rs:244` — `verify_chain` validates hash chain integrity |
| Admin API | `SECURITY.md:298` — `GET /api/admin/audit-chain` endpoint for chain verification |

**Gaps:** None.

---

### AU-9: Protection of Audit Information

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Hash chain | `crates/audit-log/src/audit_log.rs:150-151` — Each entry links to previous hash, computed via `chain::compute_hash` |
| Append-only design | `crates/audit-log/src/audit_log.rs:139` — `record` only appends; no update/delete API exposed |
| SQLite WAL mode | `SECURITY.md:60` — ADR-005: WAL mode for concurrent reads and crash recovery |

**Gaps:** None.

---

### AU-12: Audit Record Generation

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Automatic logging | `crates/audit-log/src/audit_log.rs:139` — `record` called from middleware and request handlers |
| Middleware chain | `SECURITY.md:298` — Audit middleware intercepts all authenticated requests |
| Configurable retention | `crates/audit-log/src/audit_log.rs:290` — `prune` applies `RetentionPolicy` |

**Gaps:** None.

---

## CM — Configuration Management

### CM-2: Baseline Configuration

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| ferro.toml | `crates/server-config/src/lib.rs:44-45` — `rate_limit_burst`, `rate_limit_refill` in config struct |
| CLI overrides | `crates/server-config/src/lib.rs:803-811` — CLI flags override file config via `was_set` checks |
| Config merging | `crates/server-config/src/lib.rs:622-623` — Override struct merges base and file configuration |

**Gaps:** None.

---

### CM-3: Configuration Change Control

**Status:** Partial

| Component | Evidence |
|-----------|----------|
| Config validation | Referenced in requirements as `--validate-config` flag |
| Runtime config reload | `crates/admin/src/pages/settings.rs:99` — Admin UI saves settings (requires restart) |

**Gaps:**
- No hot-reload of configuration without server restart.
- No versioned config change audit trail.

**Remediation:**
1. Implement config change events in audit log (AU-2 aligned).
2. Add file-watcher based config hot-reload for non-critical settings.

---

### CM-6: Configuration Settings

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Documented settings | `SECURITY.md:62-93` — All security settings documented in Security Architecture section |
| Secure defaults | `crates/server-routes/src/lib.rs:29-30` — Default rate limits: burst 10,000, refill 166/sec |
| Admin UI | `crates/admin/src/pages/settings.rs:16-99` — Web UI for runtime configuration |

**Gaps:** None.

---

## IA — Identification and Authentication

### IA-2: Identification and Authentication

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| OIDC SSO | `crates/auth/src/oidc.rs:48` — Full OIDC implementation with PKCE, JWKS validation, token refresh |
| TOTP MFA | `crates/auth/src/totp.rs:36` — RFC 6238 TOTP with clock drift tolerance |
| SAML SSO | `crates/auth/src/saml.rs:168` — SAML 2.0 Web Browser SSO Profile with assertion validation |
| WebAuthn | Referenced in SECURITY.md as supported authentication method |
| LDAP | Referenced in SECURITY.md with parameterized query defense against injection |

**Gaps:** None.

---

### IA-5: Authenticator Management

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Password hashing | `SECURITY.md:57` — Bcrypt cost factor 12 (~250ms, GPU brute-force resistant) |
| API key management | `crates/auth/src/simple_auth.rs:54` — API keys with permission-level role mapping |
| TOTP secrets | `crates/auth/src/totp.rs:112` — `generate_secret` produces 20-byte cryptographically random secrets |
| Constant-time comparison | `SECURITY.md:80` — Constant-time string comparison for secrets |

**Gaps:** None.

---

### IA-8: Identification and Authentication (Non-Organizational)

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Guest/anonymous access | `crates/auth/src/oidc.rs:375` — `Claims::anonymous()` when no OIDC configured |
| Share links | `SECURITY.md:296` — UUID v4 tokens (122-bit entropy) with per-token lockout and expiration |
| Public paths | `crates/auth/src/oidc.rs:354` — `is_public_auth_path` whitelists login, callback, healthz, metrics |

**Gaps:** None.

---

## MP — Media Protection

### MP-6: Media Sanitization

**Status:** Partial

| Component | Evidence |
|-----------|----------|
| Key destruction | `SECURITY.md:80` — Constant-time comparison prevents key material leakage |
| Secure deletion | No explicit secure-delete implementation found |

**Gaps:**
- No overwriting/freeing of in-memory key material after use (zeroize).
- No secure file deletion (overwrite before unlink).

**Remediation:**
1. Integrate `zeroize` crate for all secret/key material in memory.
2. Implement secure file deletion with overwrite-before-unlink for sensitive data.

---

### MP-7: Media Use Limitations

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| WORM storage | `crates/server-compliance/src/worm.rs:206` — `is_worm_protected` prevents mutation of protected paths |
| Retention policies | `crates/audit-log/src/audit_log.rs:290` — `prune` applies `RetentionPolicy` to audit entries |
| Storage quotas | `crates/auth/src/users.rs:39` — `storage_quota_bytes` and `storage_used_bytes` per user |

**Gaps:** None.

---

## SC — System and Communications Protection

### SC-8: Confidentiality of Communications

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| TLS 1.3 | `SECURITY.md:77` — TLS 1.3 enforced via rustls for all transport |
| HSTS | `SECURITY.md:89` — `Strict-Transport-Security` header set |
| No HTTP fallback | `SECURITY.md:77` — All communication encrypted; no plaintext fallback |

**Gaps:** None.

---

### SC-12: Cryptographic Key Management

**Status:** Partial

| Component | Evidence |
|-----------|----------|
| Age encryption keys | `crates/server-security/src/encryption.rs:12` — Passphrase-based key derivation via age (scrypt) |
| HMAC keys | `SECURITY.md:58` — HMAC-SHA256 for HTTP signatures |
| Key hierarchy | Referenced as "in progress" in requirements |

**Gaps:**
- No formal key hierarchy (master key → derived keys).
- No key rotation mechanism documented.
- No HSM/KMS integration.

**Remediation:**
1. Document key hierarchy in ADR.
2. Implement key rotation for HMAC signing keys.
3. Add optional HSM/KMS backend via PKCS#11.

---

### SC-13: Cryptographic Protection

**Status:** Implemented

| Algorithm | Evidence |
|-----------|----------|
| SHA-256 | `crates/audit-log/src/audit_log.rs:151` — Hash chain uses SHA-256 via `chain::compute_hash` |
| AES-GCM / ChaCha20-Poly1305 | `crates/server-security/src/encryption.rs:14` — age encryption uses X25519 + ChaCha20-Poly1305 |
| ECDSA | Referenced in SECURITY.md for federation signatures |
| HMAC-SHA256 | `SECURITY.md:58` — HTTP signature verification |
| bcrypt | `SECURITY.md:79` — Password hashing with cost factor 12 |

**Gaps:** None.

---

### SC-28: Protection of Information at Rest

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| E2E file encryption | `crates/server-security/src/encryption.rs:12` — `encrypt_content` / `decrypt_content` using age |
| Encrypted file detection | `crates/server-security/src/encryption.rs:70` — `is_age_encrypted` checks for AGE header |
| WORM immutability | `crates/server-compliance/src/worm.rs:206` — Protected paths cannot be modified/deleted |

**Gaps:** None.

---

## SI — System and Information Integrity

### SI-2: Flaw Remediation

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| cargo-deny | `deny.toml:2` — cargo-deny configuration for dependency auditing |
| cargo audit | `SECURITY.md:271` — Weekly automated `cargo audit` in CI |
| Dependency policy | `SECURITY.md:275-280` — No critical CVEs, maintainer required, version policy |

**Gaps:** None.

---

### SI-7: Software Integrity

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| WASM validation | `crates/wasm-host/src/sandbox.rs:66-71` — WASM magic bytes verification (`0x00 0x61 0x73 0x6d`) |
| WASM size limits | `crates/wasm-host/src/sandbox.rs:57` — Maximum WASM module size: 100MB |
| Content hashing | `crates/server-versioning/src/lib.rs:161` — `compute_content_hash` for file deduplication and integrity |
| Hash chain | `crates/audit-log/src/audit_log.rs:151` — Audit log integrity via hash chain |

**Gaps:** None.

---

### SI-10: Information Input Validation

**Status:** Implemented

| Component | Evidence |
|-----------|----------|
| Path validation | `crates/common/src/path.rs:67` — `validate_path` rejects traversal, empty, whitespace |
| WebDAV XML defense | `SECURITY.md:87` — XML entity expansion prevention |
| Content-Type validation | `SECURITY.md:85` — Magic bytes verification on uploads |
| Request body limits | `SECURITY.md:86` — Configurable request body size limits |
| Fuzzing | `fuzz/fuzz_targets/fuzz_path_normalize.rs:27` — Property-based fuzz testing of `validate_path` |

**Gaps:** None.

---

## Summary

| Family | Controls Mapped | Implemented | Partial | Gaps |
|--------|----------------|-------------|---------|------|
| AC (Access Control) | 7 | 7 | 0 | 0 |
| AU (Audit) | 5 | 5 | 0 | 0 |
| CM (Configuration) | 3 | 2 | 1 | 2 |
| IA (Identification & Auth) | 3 | 3 | 0 | 0 |
| MP (Media Protection) | 2 | 1 | 1 | 2 |
| SC (System & Comm Protection) | 4 | 3 | 1 | 3 |
| SI (System Integrity) | 3 | 3 | 0 | 0 |
| **Total** | **27** | **24** | **3** | **7** |

### Priority Remediation Items

1. **SC-12**: Document key hierarchy and implement key rotation (High)
2. **MP-6**: Integrate `zeroize` for secret material and secure file deletion (High)
3. **CM-3**: Add config change audit trail and hot-reload capability (Medium)
