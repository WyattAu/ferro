# STRIDE Threat Model — Ferro Storage Server

**Last updated**: 2026-05-14
**Ferro version**: v2.5.1
**Author**: Security audit preparation

---

## Threat Actors

| Actor | Capability | Motivation | Likelihood |
|-------|-----------|------------|------------|
| External attacker | Network access only | Data theft, ransomware, disruption | High |
| Authenticated user | Valid credentials | Unauthorized access to shared files | Medium |
| Malicious insider | Server access | Data manipulation, privilege escalation | Low |
| Supply chain | Dependency injection | Backdoor, data exfiltration | Low |

## Asset Inventory

| Asset | Sensitivity | Location | Notes |
|-------|------------|----------|-------|
| User files | High | Storage backend (in-memory / S3 / filesystem) | Primary target |
| Metadata (file info, hashes) | Medium | SQLite / in-memory | Path and ownership data |
| Auth tokens (OIDC Bearer) | High | In-memory / browser localStorage | Short-lived |
| Admin credentials | Critical | CLI args / env vars | Single admin pair |
| WASM modules | High | In-memory / workers directory | Custom code execution |
| Cedar policies | High | Policy file / in-memory | Defines all access control |
| WOPI token secret | Critical | CLI args / env vars | Signs all WOPI tokens |
| Share links | Medium | In-memory | UUID tokens with optional passwords |
| Audit log | Medium | In-memory | Request history |

---

## STRIDE Analysis per Component

### Authentication — `simple_auth.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | Brute force on Basic auth | Unauthorized access | Rate limiting (10k req/min) | Mitigated | Medium |
| **Tampering** | Credentials in base64 (not encrypted) | Credential interception | TLS required at reverse proxy | Partial | High |
| **Repudiation** | Attacker denies auth attempts | Cannot trace abuse | Audit log records auth failures | Mitigated | Low |
| **Info Disclosure** | 401 reveals Basic auth is configured | Reconnaissance | Acceptable — standard behavior | Accepted | Low |
| **Denial of Service** | Auth check on every request | CPU exhaustion | Constant-time string compare; CPU bounded | Mitigated | Low |
| **Elevation of Privilege** | No role system yet | Admin-only, no escalation surface | Single admin model | Mitigated | Medium |

### Authentication — `auth/oidc.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | CSRF on login redirect | Session hijacking | State parameter prevents CSRF | Mitigated | Low |
| **Tampering** | Authorization code interception | Token theft | PKCE prevents code interception | Mitigated | Low |
| **Repudiation** | Token issuance not logged | Cannot trace token grants | Token validation logged via request_logging | Mitigated | Low |
| **Info Disclosure** | Token in localStorage | XSS token theft | CSP header prevents script injection | Partial | Medium |
| **Denial of Service** | OIDC provider down | No login possible | Fallback to simple auth | Mitigated | Low |
| **Elevation of Privilege** | Token forgery | Full access | RS256 signature validation via JWKS | Mitigated | Low |

### File Storage — `webdav.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | WebDAV without auth | Unauthorized file access | Auth middleware enforced on all paths | Mitigated | Low |
| **Tampering** | Direct PUT to any path | Unauthorized writes | Cedar policy enforcement | Mitigated | High (if no Cedar) |
| **Repudiation** | File modifications untraceable | Cannot audit changes | Audit log records all operations | Mitigated | Low |
| **Info Disclosure** | PROPFIND lists all files | Data enumeration | Auth required; Cedar can restrict list | Mitigated | Medium |
| **Denial of Service** | Large file uploads | Storage exhaustion | Body size limit (`max_body_size`) | Mitigated | Medium |
| **Elevation of Privilege** | Path traversal in file names | Read/write arbitrary files | `sanitize_path` rejects `..`, null bytes | Mitigated | High |

### WASM Runtime — `wasm_upload.rs`, `worker_runner.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | N/A (internal only) | — | N/A | N/A | Low |
| **Tampering** | Malicious WASM modules | Arbitrary computation | Fuel limits, memory limits, timeout | Mitigated | Medium |
| **Repudiation** | Worker execution not logged | Cannot track worker effects | Worker execution logged via tracing | Mitigated | Low |
| **Info Disclosure** | WASM reads host data | Data leakage | Sandbox boundary (no WASI fd_read by default) | Mitigated | Medium |
| **Denial of Service** | Infinite loops in WASM | CPU exhaustion | Fuel limit + execution timeout | Mitigated | Medium |
| **Elevation of Privilege** | WASM escapes sandbox | Full system compromise | No host functions exposed; fuel/memory caps | Mitigated | High |

### WOPI Integration — `wopi.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | Forged WOPI tokens | Unauthorized file editing | HMAC-SHA256 signature validation | Mitigated | Low |
| **Tampering** | Modified token payload | Privilege escalation | Signature covers full payload; expiry check | Mitigated | Low |
| **Repudiation** | WOPI operations not logged | Cannot track edits | WOPI ops logged via audit trail | Mitigated | Low |
| **Info Disclosure** | CheckFileInfo leaks metadata | File metadata exposure | Only authorized via valid token | Mitigated | Low |
| **Denial of Service** | Rapid token issuance | Resource exhaustion | Rate limiting applies | Mitigated | Low |
| **Elevation of Privilege** | Token reuse across files | Access to wrong file | Token payload includes file path | Mitigated | Low |

### Shares — `shares.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | Share token guessing | Unauthorized download | UUID v4 tokens (122 bits entropy) | Mitigated | Low |
| **Tampering** | Share password bypass | Unauthorized access | Password comparison; optional enforcement | Partial | Medium |
| **Repudiation** | Share creation not attributed | Cannot track who shared | `created_by` field recorded | Mitigated | Low |
| **Info Disclosure** | Share URL leaked | Unauthorized access | Password + expiry + download limit | Mitigated | Medium |
| **Denial of Service** | Mass share creation | Memory exhaustion | Rate limiting; shares in-memory | Mitigated | Low |
| **Elevation of Privilege** | Share bypasses directory auth | Access to files without auth | By design — shares are intentional access grants | Accepted | Medium |

### Rate Limiting — `rate_limit.rs`

| Threat | Description | Impact | Mitigation | Status | Risk |
|--------|-------------|--------|------------|--------|------|
| **Spoofing** | IP spoofing via X-Forwarded-For | Bypass rate limit | Trusts proxy-set header; must be behind trusted proxy | Partial | Medium |
| **Tampering** | N/A | — | N/A | N/A | Low |
| **Repudiation** | N/A | — | N/A | N/A | Low |
| **Info Disclosure** | Rate limit response reveals policy | Reconnaissance | Only returns retry-after, not limits | Mitigated | Low |
| **Denial of Service** | Memory exhaustion from IP entries | Server crash | Cleanup task removes stale entries | Mitigated | Low |
| **Elevation of Privilege** | N/A | — | N/A | N/A | Low |

---

## Trust Boundaries

```
┌─────────────────────────────────────────────────────┐
│                    Internet                          │
└──────────────────────┬──────────────────────────────┘
                       │ HTTPS (reverse proxy)
┌──────────────────────▼──────────────────────────────┐
│  Security Headers │ Request ID │ Request Logging     │
├─────────────────────────────────────────────────────┤
│  CORS │ Simple Auth │ OIDC Auth │ Cedar Authz        │
├─────────────────────────────────────────────────────┤
│  Rate Limiter                                     │
├─────────────────────────────────────────────────────┤
│  WebDAV │ API │ WOPI │ Shares │ WASM Upload        │
├─────────────────────────────────────────────────────┤
│  Storage Engine │ WASM Runtime │ Cedar Authorizer   │
└─────────────────────────────────────────────────────┘
```

## Assumptions

1. Ferro runs behind a trusted reverse proxy (nginx/Caddy) that terminates TLS.
2. The reverse proxy sets `X-Forwarded-For` correctly (single trusted hop).
3. Admin credentials are managed securely (not committed to VCS).
4. OIDC provider is operated by a trusted identity provider.
5. WASM modules are uploaded only by authorized administrators.
6. Storage backend (S3, filesystem) access is restricted to the Ferro process.

## Out of Scope

- Tauri desktop client security (separate threat model)
- Reverse proxy configuration
- Operating system hardening
- Physical security
- Social engineering attacks
