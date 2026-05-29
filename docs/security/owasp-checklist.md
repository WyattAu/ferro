# OWASP Top 10 (2021) Compliance Checklist

**Last updated**: 2026-05-14
**Ferro version**: x.y.z

---

## A01: Broken Access Control

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | Auth middleware enforced on all non-public paths | [x] | Critical | `simple_auth.rs:8-84`, `auth/oidc.rs:276-314` |
| 2 | Public paths explicitly whitelisted (`is_public_path`) | [x] | High | `simple_auth.rs:86-94`, `auth/oidc.rs:266-271` |
| 3 | 401 returned with `WWW-Authenticate` header | [x] | Medium | `simple_auth.rs:32-41` |
| 4 | Path traversal protection | [x] | Critical | `webdav.rs:47-50` (sanitize_path), `common/src/path.rs:47-49` (validate_path) |
| 5 | Cedar policy-based authorization on every request | [x] | High | `auth/cedar.rs:189-242` |
| 6 | Share links bypass auth (by design, with password + expiry) | [~] | Medium | `shares.rs:140-199` — password is optional |
| 7 | WOPI tokens scoped to specific file paths | [x] | Medium | `wopi.rs:50-99` |
| 8 | Rate limiting on auth endpoints | [x] | Medium | `rate_limit.rs:50-77`, `lib.rs:216-243` |

**Remediation notes**:
- [A01-6] Share passwords should be mandatory for sensitive files. Consider making `password` required in `CreateShareRequest` or adding a server-wide policy.

---

## A02: Cryptographic Failures

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | OIDC uses PKCE (no client secret in browser) | [x] | High | `auth/oidc.rs:66-83` |
| 2 | WOPI tokens signed with HMAC-SHA256 | [x] | High | `wopi.rs:74-80` |
| 3 | Bearer tokens validated via JWKS (RS256) | [x] | High | `auth/oidc.rs:134-166` |
| 4 | TLS enforcement | [ ] | Critical | Requires reverse proxy (nginx/Caddy) |
| 5 | Sensitive config (admin password) passed via CLI args / env | [~] | High | `lib.rs:167-179` — env vars may appear in process listing |
| 6 | No sensitive data in logs | [x] | Medium | Tokens/credentials not logged; only user sub is logged |
| 7 | WOPI token secret configurable (not hardcoded in prod) | [~] | Medium | `lib.rs:94` — default secret exists; production checklist warns |

**Remediation notes**:
- [A02-4] TLS must be terminated at a reverse proxy. Add startup warning if `external_url` is `http://`.
- [A02-5] Consider reading secrets from files (`--admin-password-file`) to avoid env var leakage.

---

## A03: Injection

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | No SQL injection (SQLite via parameterized queries / rusqlite) | [x] | High | Storage backends use object_store / in-memory; no raw SQL |
| 2 | No command injection (no shell execution) | [x] | Critical | No `std::process::Command` or shell calls in server code |
| 3 | WebDAV XML injection (PROPFIND parsing) | [~] | Medium | `xml.rs` uses `escape_xml()` for output; `quick-xml` for parsing |
| 4 | Path traversal via file names | [x] | Critical | `webdav.rs` sanitize_path + `common/src/path.rs` validate_path |
| 5 | XSS in file names (stored) | [x] | Medium | `xml.rs:escape_xml` escapes XML entities; CSP header prevents execution |
| 6 | WASM sandboxing prevents host access | [x] | High | `wasm_upload.rs:30-32` (magic bytes), fuel/memory/timeout limits |

**Remediation notes**:
- [A03-3] Verify `quick-xml` configuration disables external entity (XXE) processing. Confirm `quick-xml` does not resolve DTD entities by default (it does not).

---

## A04: Insecure Design

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | Simple auth + OIDC for different threat models | [x] | Medium | `simple_auth.rs`, `auth/oidc.rs` |
| 2 | Rate limiting prevents brute force | [x] | High | `rate_limit.rs:50-77` |
| 3 | Account lockout after N failed attempts | [ ] | Medium | Not implemented |
| 4 | Cedar policies for fine-grained access control | [x] | High | `auth/cedar.rs:20-125` |
| 5 | Share links have expiry and download limits | [x] | Medium | `shares.rs:43-59` |
| 6 | WOPI tokens have expiry (8 hours) | [x] | Low | `wopi.rs:350` |
| 7 | Threat modeling performed | [x] | Medium | `docs/security/threat-model.md` |

**Remediation notes**:
- [A04-3] Implement failed auth counter per IP/user. Lock after 10 failures for 15 minutes.

---

## A05: Security Misconfiguration

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | No default credentials required (auth optional) | [x] | High | `lib.rs:177-179` — auth must be explicitly configured |
| 2 | Security headers recommended in docs | [~] | Medium | `SECURITY.md` recommends reverse proxy headers |
| 3 | HSTS / security headers middleware | [x] | Medium | `security_headers.rs` — added as outermost middleware |
| 4 | Error responses don't leak stack traces | [x] | Medium | `error.rs:7-14` returns JSON error without internals |
| 5 | Default WOPI token secret in code | [~] | High | `lib.rs:94` — production checklist requires changing |
| 6 | Unnecessary features disabled by default | [x] | Low | OIDC, Cedar, WASM, S3 all opt-in |
| 7 | CORS restricted to cross-origin requests only | [x] | Medium | `lib.rs:378-407` |

**Remediation notes**:
- [A05-2] Security headers are now enforced in-code via `security_headers.rs`.
- [A05-5] Consider refusing to start if `--wopi-token-secret` is the default value.

---

## A06: Vulnerable and Outdated Components

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | `cargo audit` in CI | [x] | Medium | `.github/workflows/` CI pipeline |
| 2 | 4 known transitive advisories documented | [x] | Medium | `SECURITY.md:13-53` |
| 3 | Automated dependency updates | [~] | Low | Dependabot configured but not verified |
| 4 | Tauri/GTK advisories isolated to optional feature | [x] | Low | `SECURITY.md:45-53` |
| 5 | SBOM generation | [ ] | Low | Not implemented |

**Remediation notes**:
- [A06-3] Verify Dependabot is active and PRs are being merged.
- [A06-5] Add `cargo sbom` or `syft` to CI for SBOM generation.

---

## A07: Identification and Authentication Failures

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | Basic auth with proper 401 response | [x] | Medium | `simple_auth.rs:8-84` |
| 2 | OIDC PKCE with state parameter | [x] | High | `auth/oidc.rs:66-83` |
| 3 | Password complexity enforcement | [~] | N/A | Simple auth uses single credential pair; no user registration |
| 4 | Session timeout / token expiry | [x] | Medium | OIDC tokens have `exp` claim; WOPI tokens expire in 8h |
| 5 | Bearer token validation with JWKS | [x] | High | `auth/oidc.rs:134-166` |
| 6 | Auth bypass on public paths is intentional | [x] | Medium | `simple_auth.rs:86-94` |
| 7 | No credential transport over HTTP warning | [ ] | High | Not implemented (deferred to reverse proxy) |

**Remediation notes**:
- [A07-3] N/A — Ferro simple auth is a single admin credential, not a user database.
- [A07-7] Log a warning at startup if auth is enabled but `external_url` uses `http://`.

---

## A08: Software and Data Integrity Failures

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | CI pipeline with clippy, tests, audit | [x] | Medium | CI workflow |
| 2 | WASM modules validated (magic bytes) | [x] | High | `wasm_upload.rs:30-32` |
| 3 | WASM filename validation (no path traversal) | [x] | High | `wasm_upload.rs:34-46` |
| 4 | SBOM generation | [ ] | Low | Not implemented |
| 5 | Binary signature verification | [ ] | Low | Not implemented |
| 6 | Cedar policies validated at load time | [x] | Medium | `auth/cedar.rs:40-59` |
| 7 | Dependencies pinned in Cargo.lock | [x] | Medium | `Cargo.lock` in repo |

**Remediation notes**:
- [A08-4] Integrate `cargo sbom` in release pipeline.
- [A08-5] Consider `sigstore` or GPG signatures for release binaries.

---

## A09: Security Logging and Monitoring Failures

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | Structured request logging (method, path, status, duration, IP) | [x] | Medium | `request_logging.rs:7-43` |
| 2 | Request ID tracking (`X-Request-ID`) | [x] | Low | `request_id.rs:6-24` |
| 3 | Audit log (file operations) | [x] | Medium | `audit.rs:30-39`, `webdav.rs:116-123` |
| 4 | Alerting on failed auth attempts | [ ] | Medium | Not implemented |
| 5 | Auth failures logged with IP | [x] | Medium | `simple_auth.rs` returns 401; audit captures status codes |
| 6 | Rate limit events logged | [x] | Low | `rate_limit.rs:74` |

**Remediation notes**:
- [A09-4] Add metric counter for auth failures. Expose via `/metrics`.

---

## A10: Server-Side Request Forgery (SSRF)

| # | Control | Status | Risk | Reference |
|---|---------|--------|------|-----------|
| 1 | No server-side URL fetching of user-controlled URLs | [x] | Critical | Server fetches only OIDC discovery/token endpoints (server-configured issuer) |
| 2 | OIDC issuer URL is server-configured (not user-controlled) | [x] | High | `auth/oidc.rs:17-22` |
| 3 | Presigned URL generation uses server-configured backends | [x] | Medium | `presigned.rs` delegates to `PresignedUrlGenerator` (server-configured) |
| 4 | WOPI office URL is server-configured | [x] | Low | `lib.rs:68`, `wopi.rs:107` |
| 5 | No user-controlled redirect targets | [x] | Medium | OIDC callback validates redirect against `external_url` |

**Remediation notes**:
- No remediation needed. All outbound URLs are server-configured.

---

## Summary

| Category | Done | Partial | Not Done | N/A |
|----------|------|---------|----------|-----|
| A01: Broken Access Control | 7 | 1 | 0 | 0 |
| A02: Cryptographic Failures | 4 | 2 | 1 | 0 |
| A03: Injection | 5 | 1 | 0 | 0 |
| A04: Insecure Design | 6 | 0 | 1 | 0 |
| A05: Security Misconfiguration | 5 | 2 | 0 | 0 |
| A06: Vulnerable Components | 4 | 1 | 1 | 0 |
| A07: Auth Failures | 5 | 1 | 1 | 0 |
| A08: Integrity Failures | 5 | 0 | 2 | 0 |
| A09: Logging Failures | 5 | 0 | 1 | 0 |
| A10: SSRF | 5 | 0 | 0 | 0 |
| **Total** | **51** | **8** | **7** | **0** |
