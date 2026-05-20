# Security

Ferro is designed with security as a priority. This page summarizes the security features and policies. For the full security policy, see [SECURITY.md](https://github.com/WyattAu/ferro/blob/main/SECURITY.md) in the repository.

## Reporting Vulnerabilities

| Method | Details |
|--------|---------|
| Email | security@wyatt.au (PGP encrypted) |
| GitHub | [Security Advisories](https://github.com/WyattAu/ferro/security/advisories/new) |

### Response Timeline

| Severity | Initial Response | Patch Release |
|----------|-----------------|---------------|
| Critical (RCE, auth bypass) | 24 hours | 72 hours |
| High (data exposure, privilege escalation) | 48 hours | 7 days |
| Medium (CSRF, XSS, information disclosure) | 72 hours | 14 days |
| Low (best practices, minor issues) | 1 week | Next release |

## Security Features

### Authentication

| Method | Description |
|--------|-------------|
| Simple auth | HTTP Basic Auth with bcrypt-hashed passwords (cost factor 12) |
| OIDC | OpenID Connect with PKCE flow (Keycloak, Auth0, Google, etc.) |
| LDAP | LDAP authentication (behind `ldap` feature flag) |
| Authorization | Cedar policy engine for fine-grained access control |

### Encryption

| Layer | Implementation |
|-------|---------------|
| Transport | TLS 1.3 (rustls) |
| File E2E | age (X25519, ChaCha20-Poly1305) |
| Passwords | bcrypt (cost factor 12) |
| Tokens | HMAC-SHA256 |
| Comparison | Constant-time for secrets |

### Input Validation

- Path traversal prevention (normalized paths, `..` rejection)
- Content-Type validation on uploads
- Request body size limits (configurable, default 1 GB)
- XML entity expansion prevention in WebDAV

### Security Headers

| Header | Value |
|--------|-------|
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains` |
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Content-Security-Policy` | Configurable |
| `X-Request-ID` | Unique per request (audit trail) |

### Federation Security

- HTTP Signatures (draft-cavage-http-signatures-12)
- HMAC-SHA256 verification
- Actor keyId validation (must match activity actor)
- Empty federation secret = disabled (503)

### Rate Limiting

- Per-IP token-bucket rate limiter
- Default: 10,000 requests per 60-second window
- Returns 429 Too Many Requests when exceeded

### Deployment Security

- Non-root containers where supported
- `no-new-privileges` security option
- `cap-drop: ALL` with minimal capabilities
- Resource limits on all containers
- Health checks on all services
- No secrets in configuration files

## Audit Logging

Ferro tracks all file operations in an audit log. Access via:

```bash
curl http://localhost:8080/api/audit?limit=50 \
  -H "Authorization: Bearer TOKEN"
```

### Tamper Evidence

Persisted audit entries are protected by a SHA-256 hash chain: each entry's `chain_hash` field contains `SHA-256(previous_chain_hash || entry_data)`. This makes retroactive modification or deletion detectable. The chain is verified by recomputing each hash from the previous entry's stored `chain_hash`.

## Threat Model

### Attack Surface: Federation

| Threat | Mitigation | Confidence |
|--------|------------|------------|
| Activity spoofing | HTTP Signatures (HMAC-SHA256) + actor keyId validation | High |
| Replay attacks | Timestamp window validation on incoming activities | High |
| Unauthorized federation | Empty `federation_secret` = federation disabled (503) | High |
| Payload tampering | SHA-256 content hash verification on received activities | High |
| DoS via inbox flooding | Per-IP rate limiting (token-bucket, 10k/60s) | Medium |

### Attack Surface: WASM Plugins

| Threat | Mitigation | Confidence |
|--------|------------|------------|
| Arbitrary system access | WASI capability sandboxing (wasmtime) | High |
| Resource exhaustion | Configurable worker count + memory limits | Medium |
| Supply chain (malicious WASM) | Admin-only upload, hash-based dedup | Medium |
| Data exfiltration | No outbound network access in WASM sandbox | High |

### Attack Surface: Web UI

| Threat | Mitigation | Confidence |
|--------|------------|------------|
| XSS | CSP with strict directives | High |
| Supply chain (CDN compromise) | SRI hashes on static assets (post-build injection) | Medium |
| Clickjacking | X-Frame-Options: DENY | High |

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2.x | Yes |
| < 2.0 | No |

## Dependency Security

- Weekly `cargo audit` (automated in CI)
- Monthly manual review of new dependencies
- No dependencies with known critical CVEs
- Prefer pure-Rust implementations over C bindings

## Penetration Testing

Ferro is designed to be pen-testable. See [SECURITY.md](https://github.com/WyattAu/ferro/blob/main/SECURITY.md) for the full penetration testing guide including test cases for authentication bypass, path traversal, XML injection, federation spoofing, and more.
