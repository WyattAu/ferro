# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2.x     | Yes       |
| < 2.0   | No        |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Email**: security@wyatt.au (PGP key: [fingerprint])
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

| Package | Version | CVE | Risk | Mitigation |
|---------|---------|-----|------|------------|
| rsa | 0.9 | CVE-2023-... | Timing side-channel | Pending upstream fix |
| rustls-pemfile | 1.0 | N/A | Parse error handling | Input validation in callers |
| lru | 0.12 | N/A | Memory pressure | LRU eviction limits size |
| rand | 0.8 | N/A | Deprecation notice | Migrating to rand 0.9 |

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
- **Simple auth**: Bearer token, bcrypt-hashed passwords (cost 12)
- **OIDC**: Enterprise SSO via OpenID Connect
- **Authorization**: Cedar policy engine
- **Rate limiting**: Token bucket, configurable per-route

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
# Start test server
ferro-server --auth simple --token test-token-123

# Admin account
FERRO_TOKEN=test-token-123
BASE_URL=http://localhost:8080

# Create test user
curl -X POST $BASE_URL/api/users \
  -H "Authorization: Bearer $FERRO_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","password":"TestPass123!","role":"user"}'
```

### Test Cases

#### 1. Authentication Bypass
```bash
# No auth header — should return 401
curl -v $BASE_URL/api/files/

# Invalid token — should return 401
curl -H "Authorization: Bearer invalid" $BASE_URL/api/files/

# SQL injection in token
curl -H "Authorization: Bearer ' OR 1=1 --" $BASE_URL/api/files/
```

#### 2. Path Traversal
```bash
# Try to escape root directory
curl -X PUT $BASE_URL/../../../etc/passwd \
  -H "Authorization: Bearer $FERRO_TOKEN" \
  -d "test"

# URL-encoded traversal
curl -X PUT '%2e%2e/%2e%2e/etc/passwd' \
  -H "Authorization: Bearer $FERRO_TOKEN" \
  -d "test"

# Double encoding
curl -X PUT '%252e%252e/%252e%252e/etc/passwd' \
  -H "Authorization: Bearer $FERRO_TOKEN" \
  -d "test"
```

#### 3. XML Injection (WebDAV)
```bash
# Entity expansion (billion laughs)
curl -X PROPFIND $BASE_URL/ \
  -H "Authorization: Bearer $FERRO_TOKEN" \
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
curl -X REPORT $BASE_URL/dav/calendars/default/ \
  -H "Authorization: Bearer $FERRO_TOKEN" \
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
  -H "Authorization: Bearer $FERRO_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"{ __schema { types { name } } }"}'

# Deep nesting (DoS attempt)
curl -X POST $BASE_URL/graphql \
  -H "Authorization: Bearer $FERRO_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"{ files { files { files { files { files { name } } } } } }"}'
```

#### 7. WebSocket
```bash
# Connect without auth
wscat -c ws://localhost:8080/ws

# Connect with invalid token
wscat -c "ws://localhost:8080/ws?token=invalid"
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
