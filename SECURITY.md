# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 3.x     | Yes       |
| < 3.0   | No        |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Email**: security@wyatt.au (PGP key available upon request)
2. **GitHub Security**: Use [GitHub Security Advisories](https://github.com/WyattAu/ferro/security/advisories/new)
3. **Encryption**: Please encrypt your report using the PGP key below

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)
- Your name/handle for credit (optional)

### Response Timeline

| Severity | Initial Response | Patch Release |
|----------|-----------------|---------------|
| Critical (RCE, auth bypass) | 24 hours | 72 hours |
| High (data exposure, privilege escalation) | 48 hours | 7 days |
| Medium (CSRF, XSS, information disclosure) | 72 hours | 14 days |
| Low (best practices, minor issues) | 1 week | Next release |

### Coordinated Disclosure

We follow responsible disclosure practices:
1. Acknowledge receipt within 24 hours
2. Communicate timeline and progress
3. Credit researchers (unless anonymity requested)
4. Request CVE assignment for critical issues
5. Publish advisory after patch is available

## Known Vulnerabilities

### Transitive Dependencies (Documented, Accepted Risk)

No active advisories in the dependency tree as of 2026-07-14. The `cargo-deny` configuration in `deny.toml` is clean with no ignore entries.

### Security Decisions

| Decision | Rationale | ADR |
|----------|-----------|-----|
| Bcrypt cost factor 12 | ~250ms, resists GPU brute-force | ADR-001 |
| HMAC-SHA256 for HTTP sigs | Industry standard, FIPS-approved | ADR-002 |
| age for E2E encryption | Modern, audited, passphrase-based | ADR-003 |
| Federation requires signatures | Prevents spoofed activities | ADR-004 |
| SQLite WAL mode | Concurrent reads, crash recovery | ADR-005 |

## Security Architecture

### Authentication
- **Simple auth**: HTTP Basic authentication, bcrypt-hashed passwords (cost 12)
- **OIDC**: Enterprise SSO via OpenID Connect
- **Authorization**: Cedar policy engine
- **Rate limiting**: Token bucket, per-IP (not per-route)

### Federation Security
- HTTP Signatures (draft-cavage-http-signatures-12)
- HMAC-SHA256 verification
- Actor keyId validation (must match activity actor)
- Empty federation secret = disabled (503)

### Encryption
- TLS 1.3 for all transport (rustls)
- age-based E2E file encryption (X25519, ChaCha20-Poly1305)
- Bcrypt password hashing (cost factor 12)
- Constant-time string comparison for secrets

### Input Validation
- Path traversal prevention (normalized paths, `..` rejection)
- Content-Type validation on uploads
- Request body size limits (configurable)
- XML entity expansion prevention in WebDAV

### Headers
- Strict-Transport-Security (HSTS)
- X-Content-Type-Options: nosniff
- X-Frame-Options: DENY
- Content-Security-Policy (configurable)
- X-Request-ID for audit trail

## Penetration Testing Guide

### Scope
- WebDAV endpoints (PROPFIND, PUT, GET, DELETE, MKCOL, COPY, MOVE)
- REST API endpoints (/api/*)
- CalDAV/CardDAV endpoints
- GraphQL endpoint
- WebSocket endpoint
- Federation inbox

### Out of Scope
- Denial of service (rate limiting is in place)
- Social engineering
- Physical access to infrastructure
- Third-party dependencies (see Known Vulnerabilities)

### Test Accounts
```bash
# Start test server with simple auth
ferro-server --admin-user admin --admin-password TestPass123!

# Admin credentials (HTTP Basic auth)
BASE_URL=http://localhost:8080
AUTH=admin:TestPass123!

# Create test user via admin API
curl -X POST $BASE_URL/api/admin/users \
  -u "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","password":"TestPass123!","role":"user"}'
```

### Test Cases

#### 1. Authentication Bypass
```bash
# No auth header -- should return 401
curl -v $BASE_URL/api/files/

# Invalid credentials -- should return 401
curl -u "admin:wrongpassword" $BASE_URL/api/files/

# SQL injection in credentials
curl -u "' OR 1=1 --:password" $BASE_URL/api/files/
```

#### 2. Path Traversal
```bash
# Try to escape root directory
curl -X PUT $BASE_URL/../../../etc/passwd \
  -u "$AUTH" \
  -d "test"

# URL-encoded traversal
curl -X PUT '%2e%2e/%2e%2e/etc/passwd' \
  -u "$AUTH" \
  -d "test"

# Double encoding
curl -X PUT '%252e%252e/%252e%252e/etc/passwd' \
  -u "$AUTH" \
  -d "test"
```

#### 3. XML Injection (WebDAV)
```bash
# Entity expansion (billion laughs)
curl -X PROPFIND $BASE_URL/ \
  -u "$AUTH" \
  -H "Depth: 1" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe "test">
  <!ENTITY xxe2 "&xxe;&xxe;&xxe;&xxe;">
]>
<D:propfind xmlns:D="DAV:"><D:prop><D:all/></D:prop></D:propfind>'
```

#### 4. Federation Spoofing
```bash
# Try to deliver activity without valid signature
curl -X POST $BASE_URL/federation/inbox \
  -H "Content-Type: application/json" \
  -d '{"type":"Follow","actor":"https://evil.com/user"}'

# Try with wrong keyId
curl -X POST $BASE_URL/federation/inbox \
  -H "Content-Type: application/json" \
  -H "Signature: keyId=\"https://evil.com/keys/1\",headers=\"(request-target)\",signature=\"fake\"" \
  -d '{"type":"Follow","actor":"https://evil.com/user"}'
```

#### 5. CalDAV/CardDAV
```bash
# Calendar query with malicious filter
curl -X REPORT $BASE_URL/dav/cal/default/ \
  -u "$AUTH" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0"?>
<C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav">
  <C:filter><C:comp-filter name="VCALENDAR">
    <C:prop-filter name="VEVENT">
      <C:time-range start="00000000T000000Z" end="99991231T235959Z"/>
    </C:prop-filter>
  </C:comp-filter></C:filter>
</C:calendar-query>'
```

#### 6. GraphQL Injection
```bash
# Introspection (should be limited)
curl -X POST $BASE_URL/graphql \
  -u "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{"query":"{ __schema { types { name } } }"}'

# Deep nesting (DoS attempt)
curl -X POST $BASE_URL/graphql \
  -u "$AUTH" \
  -H "Content-Type: application/json" \
  -d '{"query":"{ files { files { files { files { files { name } } } } } }"}'
```

#### 7. WebSocket
```bash
# Connect without auth
wscat -c ws://localhost:8080/api/ws

# Connect with invalid token
wscat -c "ws://localhost:8080/api/ws?token=invalid"
```

#### 8. Rate Limiting
```bash
# Burst requests
for i in $(seq 1 100); do
  curl -s -o /dev/null -w "%{http_code}\n" $BASE_URL/api/files/
done | sort | uniq -c
```

### Report Template

```markdown
## Vulnerability Report

**Title**: [Brief description]
**Severity**: Critical/High/Medium/Low
**Component**: [Which endpoint/module]
**Environment**: [OS, Rust version, Ferro version]

### Description
[Detailed description of the vulnerability]

### Steps to Reproduce
1. [Step 1]
2. [Step 2]
3. [Step 3]

### Expected Behavior
[What should happen]

### Actual Behavior
[What actually happens]

### Impact
[What an attacker could do]

### Suggested Fix
[Optional: how to fix it]
```

## Dependency Security

### Audit Schedule
- Weekly: `cargo audit` (automated in CI)
- Monthly: manual review of new dependencies
- On release: full dependency tree review

### Dependency Policy
1. No dependencies with known critical CVEs
2. Dependencies must have a maintainer
3. No dependencies with < 1.0.0 version (unless necessary)
4. Minimize dependency tree depth
5. Prefer pure-Rust implementations over C bindings

### Current Dependency Count
Run `cargo tree --depth 1` for current count.

## STRIDE Threat Model Update (2026-05-24)

### New Attack Surfaces

| Surface | Vector | Threat | Mitigation |
|---------|--------|--------|------------|
| WASM Worker Upload | Malicious WASM bytecode | Denial of service (CPU/memory exhaustion) | Fuel limit (1B), memory cap (64MB), timeout (30s), sandboxed wasmtime runtime |
| WASM Worker Execution | Side-channel via WASM | Information leakage | Linear memory isolation, no shared memory, no network access in sandbox |
| Federation Inbox | Spoofed activities | Impersonation, spam | HTTP Signatures (HMAC-SHA256), actor keyId validation, federation_secret required |
| Federation Webfinger | Actor enumeration | Information disclosure | Public endpoint, only exposes actor URL (no private data) |
| OIDC Token Refresh | Stolen refresh token | Token replay | Token rotation on refresh, short-lived access tokens, provider-side revocation |
| LDAP Auth | LDAP injection | Auth bypass | Parameterized queries via ldap3 crate, group DN validation |
| Share Links | Token brute force | Unauthorized file access | UUID v4 tokens (122-bit entropy), per-token lockout (10 fails / 5 min), expiration |
| Content-Type Spoofing | Malicious file upload | XSS via HTML upload | Magic bytes verification, Content-Type mismatch logging, nosniff header |
| Audit Log Tampering | SQLite DB modification | Cover tracks | Chain hash verification (`GET /api/admin/audit-chain`), append-only design |

### WASM Worker Security Model

1. **Sandboxing**: Wasmtime runtime with WASI capabilities disabled
2. **Resource limits**: Fuel-based execution metering (1 billion units), 64MB memory cap, 30s timeout
3. **Upload validation**: `.wasm` extension required, WASM magic bytes verification, filename sanitization
4. **Network isolation**: Workers cannot make outbound network requests
5. **Filesystem isolation**: Workers receive file content as input, cannot access storage directly

### Federation Security Model

1. **Inbound**: HTTP Signatures required on all POST to `/fed/inbox`, actor keyId validated against activity actor
2. **Outbound**: Activities signed with server's federation_secret via HMAC-SHA256
3. **Discovery**: Webfinger and NodeInfo are public read-only endpoints
4. **Rate limiting**: Subject to global rate limiter
5. **Disable mechanism**: Empty federation_secret = federation endpoints return 503
