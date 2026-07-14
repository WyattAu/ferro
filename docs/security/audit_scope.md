# Ferro Security Audit Scope

**Document:** External Security Audit Scope  
**Version:** 1.0.0  
**Date:** 2026-07-12  
**Status:** Draft  

---

## Scope

### In-Scope Components

| Component | Description | Priority |
|-----------|-------------|----------|
| ferro-server | Main HTTP/WebDAV server (Axum framework) | Critical |
| ferro-auth | Authentication (OIDC/PKCE, Basic, TOTP, SAML, WebAuthn) | Critical |
| ferro-crypto | Encryption primitives (AES-GCM, X25519, age) | Critical |
| ferro-dav | WebDAV protocol implementation (Class 1/2/3) | High |
| ferro-server-security | Security middleware (CORS, rate limiting, headers, request smuggling protection) | Critical |
| ferro-server-webdav | WebDAV handler (PROPFIND, MKCOL, COPY, MOVE, LOCK) | High |
| ferro-server-admin | Admin API (user CRUD, backups, plugins, WORM, retention, GDPR) | High |
| ferro-wasm-host | WASM worker sandbox (wasmtime, fuel metering, memory cap) | High |
| ferro-caldav | CalDAV/CardDAV protocol (vCard/iCal parsing) | Medium |
| ferro-scim | SCIM provisioning | Medium |
| ferro-client | Client library (Rust SDK) | Medium |
| ferro-ldap | LDAP authentication integration | Medium |
| ferro-federation | ActivityPub federation (HTTP Signatures, WebFinger) | High |

### Out-of-Scope Components

| Component | Reason |
|-----------|--------|
| ferro-web (Leptos frontend) | Client-side only, no server secrets; XSS in SPA is separate engagement |
| ferro-desktop (Tauri) | Desktop app, no network attack surface |
| ferro-fuse | FUSE mount, requires local filesystem access |
| ferro-mount-nfs | NFS mount, requires network-level access |
| formal/ (Lean4 proofs) | Mathematical verification, not runtime code |
| monitoring/ | Infrastructure observability, not application security |

### Attack Surfaces

#### 1. Network-Facing APIs

| Surface | Protocol | Auth Required | Notes |
|---------|----------|---------------|-------|
| REST API (`/api/*`) | HTTP/HTTPS | Yes (except `/api/auth/info`, `/api/auth/login`) | 100+ endpoints |
| WebDAV (`/`) | HTTP/HTTPS | Yes (Basic auth) | Class 1/2/3, LOCK/UNLOCK |
| CalDAV (`/dav/cal/*`) | HTTP/HTTPS | Yes | vCalendar parsing |
| CardDAV (`/dav/card/*`) | HTTP/HTTPS | Yes | vCard parsing |
| GraphQL (`/api/graphql`) | HTTP/HTTPS | Yes | Introspection, deep nesting |
| WebSocket (`/api/ws`, `/ws/*`) | WS/WSS | Yes | Real-time sync, collaboration |
| WOPI (`/wopi/*`) | HTTP/HTTPS | Token-based | Office Online integration |
| Federation (`/fed/*`) | HTTP/HTTPS | HTTP Signatures | ActivityPub inbox/outbox |
| Public shares (`/s/:token`) | HTTP/HTTPS | Token-based | Password-protected optional |
| Remote mount proxy (`/remote/*`) | HTTP/HTTPS | Yes | SSRF risk |
| Metrics (`/metrics`) | HTTP/HTTPS | No | Prometheus format |
| Health (`/healthz`, `/readyz`) | HTTP/HTTPS | No | Liveness/readiness |

#### 2. Authentication Mechanisms

| Mechanism | Implementation | Risk Areas |
|-----------|---------------|------------|
| OIDC/PKCE | `OidcValidator`, JWKS caching | Token validation, algorithm confusion, JWKS cache poisoning |
| Simple Auth | Argon2 password hashing | Brute force, credential stuffing |
| HTTP Basic | WebDAV endpoints | Timing attacks, no MFA |
| TOTP | `/api/auth/totp/*` | Replay, secret prediction |
| WebAuthn/FIDO2 | Feature-gated (`webauthn`) | Attestation bypass |
| SAML 2.0 | Enterprise SSO | XML signature wrapping |
| API Keys | Separate key store | Entropy, timing attacks |
| Cedar | Policy engine | Policy bypass, malformed input |
| Guest tokens | Guest middleware | Privilege escalation |

#### 3. File Operations

| Operation | Endpoint | Risk Areas |
|-----------|----------|------------|
| Upload (multipart) | `/api/files/*` | Path traversal, filename injection, Content-Type spoofing |
| Chunked upload | `/api/upload/*` | Chunk reassembly, TOCTOU |
| Download (ranged GET) | `/api/files/*` | Path traversal, information disclosure |
| COPY/MOVE/DELETE | `/api/files/*`, WebDAV | Overwrite, symlink following |
| Share links | `/s/:token` | Token brute force, password bypass |
| Pre-signed URLs | `/api/upload-url`, `/api/download-url` | Token prediction, replay |
| E2EE encrypt/decrypt | `/api/e2ee/*` | Key management, side-channel |
| WASM upload | `/api/wasm/upload` | Malicious bytecode, sandbox escape |

#### 4. WASM Sandbox

| Aspect | Implementation | Risk Areas |
|--------|---------------|------------|
| Runtime | wasmtime | Sandbox escape, CVE in wasmtime |
| Resource limits | Fuel (1B units), 64MB memory, 30s timeout | Bypass, exhaustion |
| I/O isolation | No network, no filesystem access | Information leakage via side-channel |
| Module validation | Magic bytes, `.wasm` extension | Malformed WASM, obfuscation |

#### 5. Configuration

| Aspect | File/Source | Risk Areas |
|--------|-------------|------------|
| Server config | `ferro.toml` | Insecure defaults, debug mode |
| CLI arguments | `--bind`, `--admin-user`, etc. | Command injection, credential exposure |
| Environment variables | `FERRO_*` | Secret leakage in process list |
| TLS certificates | rustls config | Weak ciphers, expired certs |
| CORS | Configurable origins | Credential theft via `*` |
| Federation secret | Config | Empty = disabled, weak = spoofing |

### Test Scenarios

| Category | Scenarios | Priority |
|----------|-----------|----------|
| Authentication | Brute force, credential stuffing, session hijacking, token forgery, OIDC state manipulation, PKCE bypass | Critical |
| Authorization | Privilege escalation (user→admin), IDOR, path traversal, Cedar bypass, guest token escalation, horizontal file access | Critical |
| Injection | SQL injection, command injection, LDAP injection, XSS (stored/reflected), SSRF (federation, remote mounts, OIDC discovery), XXE (WebDAV), GraphQL injection | Critical |
| Cryptography | Key management, encryption bypass, padding oracle, TLS downgrade, JWT algorithm confusion, API key entropy, Argon2 parameter review | High |
| DoS | Resource exhaustion (uploads, connections), memory exhaustion (WASM, XML entity expansion), CPU exhaustion (deep GraphQL nesting), connection flood | High |
| Data exposure | Information disclosure (error messages, metrics, EXIF), log leakage, backup file exposure, debug endpoints | High |
| Supply chain | Dependency vulnerability (`cargo audit`), malicious crate, build compromise, WASM module tampering | Medium |
| Configuration | Default credentials, insecure defaults, debug mode in production, CORS misconfiguration, missing security headers | Medium |
| Federation | Activity spoofing, signature bypass, WebFinger enumeration, inbox flooding, key rotation | High |
| Protocol | WebDAV method tampering, CalDAV/CardDAV injection, WOPI token forgery, WebSocket hijacking | High |

### Success Criteria

- All critical/high vulnerabilities remediated within SLA (Critical: 72h, High: 7d)
- No bypass of authentication/authorization mechanisms
- No data exfiltration without detection
- No denial of service without mitigation
- Cryptographic implementations verified against standards (NIST SP 800-57, FIPS 140-3)
- WASM sandbox remains isolated under adversarial input
- Federation activities cannot be spoofed without valid HTTP Signatures
- All input validation handles malformed/malicious payloads gracefully

### Timeline

| Phase | Duration | Description |
|-------|----------|-------------|
| Pre-audit | 2 weeks | Scope finalization, environment setup, credential provisioning, test data preparation |
| Audit execution | 4 weeks | Penetration testing, code review, cryptographic analysis, fuzzing |
| Remediation | 2 weeks | Fix critical/high findings, patch release |
| Re-test | 1 week | Verify fixes, regression testing |
| Report | 1 week | Final audit report, executive summary, CVSS scoring |

### Environment Requirements

| Requirement | Specification |
|-------------|---------------|
| Rust version | Latest stable (1.80+) |
| Deployment | Docker or bare metal, dedicated test instance |
| TLS | Self-signed cert for testing (or Let's Encrypt staging) |
| OIDC provider | Mock provider (e.g., Keycloak test realm) |
| LDAP | Test LDAP server (e.g., OpenLDAP container) |
| Database | SQLite (default), PostgreSQL (optional) |
| Network | Isolated test network, no production data |

### Deliverables

| Deliverable | Format | Due |
|-------------|--------|-----|
| Executive summary | PDF/Markdown | End of audit |
| Detailed findings | Markdown with CVSS | End of audit |
| Proof-of-concept code | Scripts/replay files | With findings |
| Risk assessment matrix | Spreadsheet | End of audit |
| Remediation recommendations | Per-finding | With findings |
| Re-test verification | Checklist | After remediation |

---

*Document references: `penetration_test_scope.md`, `SECURITY.md`, codebase analysis of 56+ crate workspace.*
