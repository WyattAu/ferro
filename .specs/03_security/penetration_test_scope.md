# Penetration Test Scope — Ferro Server

## 1. Executive Summary

**Ferro** is a self-hosted, open-source file synchronization platform and Nextcloud alternative built as a 56+ crate Rust workspace. It provides file sync, sharing, CalDAV/CardDAV, WebDAV, real-time collaboration, E2EE encryption, federation, WASM plugin execution, and comprehensive admin tooling.

This document defines the scope, targets, test categories, and severity classification for a penetration test of the Ferro server (`ferro-server`). The goal is to identify exploitable vulnerabilities across authentication, authorization, input validation, cryptography, API security, and infrastructure.

**Scope**: Server-side binary and all exposed endpoints. Web UI is out of scope for this engagement.

**Timeline**: 4-week engagement recommended (1 week reconnaissance + 2 weeks active testing + 1 week reporting).

## 2. Target Systems

### 2.1 Server Binary

- **Binary**: `ferro-server`
- **Language**: Rust (Axum web framework)
- **Deployment**: Docker or bare metal
- **Default port**: configurable via `--bind`

### 2.2 API Endpoints (REST)

All endpoints under `/api/` and `/api/v1/`. Key categories:

#### Authentication
| Method | Path | Auth Required | Notes |
|--------|------|---------------|-------|
| GET | `/api/auth/info` | No | Public — returns auth config |
| GET | `/api/auth/login` | No | OIDC redirect initiation |
| GET | `/api/auth/callback` | No | OIDC callback with code exchange |
| POST | `/api/auth/refresh` | Yes | Token refresh |
| POST | `/api/auth/change-password` | Yes | Password change |
| POST | `/api/auth/totp/setup` | Yes | TOTP enrollment |
| POST | `/api/auth/totp/enable` | Yes | TOTP activation |
| POST | `/api/auth/totp/disable` | Yes | TOTP deactivation |
| GET | `/api/auth/totp/status` | Yes | TOTP status |
| POST | `/api/auth/webauthn/register/begin` | Yes | WebAuthn registration (feature-gated) |
| POST | `/api/auth/webauthn/register/finish` | Yes | WebAuthn registration finalization |
| POST | `/api/auth/webauthn/login/begin` | Yes | WebAuthn authentication start |
| POST | `/api/auth/webauthn/login/finish` | Yes | WebAuthn authentication finalization |

#### File Operations
| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/files` | List files |
| POST | `/api/files/mkdir` | Create directory |
| POST | `/api/files/move` | Move file |
| POST | `/api/files/copy` | Copy file |
| POST | `/api/files/encrypt` | Encrypt file |
| POST | `/api/files/decrypt` | Decrypt file |
| GET | `/api/upload-url` | Presigned upload URL |
| GET | `/api/download-url` | Presigned download URL |
| POST | `/api/upload/init` | Initiate chunked upload |
| PUT | `/api/upload/:upload_id/chunk/:chunk_index` | Upload chunk |
| POST | `/api/upload/:upload_id/complete` | Finalize upload |
| DELETE | `/api/upload/:upload_id` | Cancel upload |
| GET | `/api/uploads` | List active uploads |
| POST | `/api/wasm/upload` | Upload WASM module |
| GET | `/api/wasm/modules` | List WASM modules |
| DELETE | `/api/wasm/modules/:filename` | Delete WASM module |

#### Sharing & Federation
| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/shares` | List shares |
| POST | `/api/shares` | Create share |
| DELETE | `/api/shares/:token` | Delete share |
| POST | `/api/shares/ext` | Extended share creation |
| POST | `/api/fed/share` | Federated share |
| POST | `/api/fed/inbox` | Federation inbox |
| GET | `/api/fed/outbox` | Federation outbox |

#### Admin Endpoints
| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/admin/stats` | Server statistics |
| GET | `/api/admin/users` | List users |
| POST | `/api/admin/users` | Create user |
| PUT | `/api/admin/users/:id` | Update user |
| DELETE | `/api/admin/users/:id` | Delete user |
| POST | `/api/admin/users/:id/reset-password` | Reset password |
| PUT | `/api/admin/users/:id/role` | Set user role |
| POST | `/api/admin/users/:id/transfer` | Transfer user data |
| POST | `/api/admin/devices/:user_id/wipe` | Wipe user devices |
| GET | `/api/admin/users/:id/devices` | List user devices |
| POST | `/api/admin/users/:id/devices/:device_id/revoke` | Revoke device |
| POST | `/api/admin/backup` | Create backup |
| GET | `/api/admin/backup/latest` | Get latest backup |
| GET | `/api/admin/backup/download` | Download backup |
| POST | `/api/admin/backup/restore` | Restore from backup |
| POST | `/api/admin/restore` | Restore backup |
| DELETE | `/api/admin/backup/:id` | Delete backup |
| GET | `/api/admin/backups` | List backups |
| GET | `/api/admin/maintenance` | Get maintenance mode |
| POST | `/api/admin/maintenance` | Toggle maintenance mode |
| POST | `/api/admin/webhooks` | Create webhook |
| GET | `/api/admin/webhooks` | List webhooks |
| DELETE | `/api/admin/webhooks/:id` | Delete webhook |
| GET | `/api/admin/webhooks/:id/deliveries` | Webhook deliveries |
| GET | `/api/admin/webhooks/deliveries/dead` | Dead letter queue |
| GET | `/api/admin/branding` | Get branding config |
| PUT | `/api/admin/branding` | Update branding |
| DELETE | `/api/admin/branding` | Reset branding |
| POST | `/api/admin/guests` | Create guest |
| GET | `/api/admin/guests` | List guests |
| DELETE | `/api/admin/guests/:id` | Revoke guest |
| POST | `/api/admin/retention/policies` | Create retention policy |
| GET | `/api/admin/retention/policies` | List retention policies |
| DELETE | `/api/admin/retention/policies/:id` | Delete retention policy |
| POST | `/api/admin/retention/execute` | Execute retention |
| POST | `/api/admin/worm/policies` | Create WORM policy |
| GET | `/api/admin/worm/policies` | List WORM policies |
| DELETE | `/api/admin/worm/policies/:id` | Delete WORM policy |
| GET | `/api/admin/gdpr` | List GDPR requests |
| POST | `/api/admin/users/:id/export` | Export user data |
| DELETE | `/api/admin/users/:id/data` | Erase user data |
| POST | `/api/admin/triggers` | Create event trigger |
| GET | `/api/admin/triggers` | List event triggers |
| DELETE | `/api/admin/triggers/:id` | Delete event trigger |
| POST | `/api/admin/triggers/:id/toggle` | Toggle event trigger |
| GET | `/api/admin/tenants/rate-limits` | List tenant rate limits |
| GET | `/api/admin/tenants/:id/rate-limit` | Get tenant rate limit |
| PUT | `/api/admin/tenants/:id/rate-limit` | Update tenant rate limit |
| DELETE | `/api/admin/tenants/:id/rate-limit` | Delete tenant rate limit |
| GET | `/api/admin/tenants/:id/rate-limit/status` | Tenant rate limit status |
| GET | `/api/admin/mounts` | List remote mounts |
| POST | `/api/admin/mounts` | Create remote mount |
| DELETE | `/api/admin/mounts/:id` | Delete remote mount |
| GET | `/api/admin/mounts/:id/test` | Test remote mount |
| POST | `/api/admin/plugins/:id/install` | Install plugin |
| POST | `/api/admin/plugins/:id/uninstall` | Uninstall plugin |
| POST | `/api/admin/plugins/:id/enable` | Enable plugin |
| POST | `/api/admin/plugins/:id/disable` | Disable plugin |
| GET | `/api/admin/search/config` | Get search config |
| PUT | `/api/admin/search/config` | Update search config |
| POST | `/api/admin/search/reindex` | Trigger reindex |

#### Application Endpoints
| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/search` | Search files |
| GET | `/api/tags` | List tags |
| POST | `/api/tags/:path` | Add tags to file |
| DELETE | `/api/tags/:path/:tag` | Remove tag |
| GET | `/api/tags/search` | Search by tag |
| GET | `/api/comments` | List comments |
| POST | `/api/comments` | Create comment |
| PUT | `/api/comments/:id` | Update comment |
| DELETE | `/api/comments/:id` | Delete comment |
| POST | `/api/comments/:id/resolve` | Resolve comment |
| GET | `/api/snapshots` | List snapshots |
| POST | `/api/snapshots` | Create snapshot |
| DELETE | `/api/snapshots/:id` | Delete snapshot |
| POST | `/api/snapshots/:id/restore` | Restore snapshot |
| GET | `/api/favorites` | List favorites |
| PUT | `/api/favorites` | Add favorite |
| DELETE | `/api/favorites` | Remove favorite |
| GET | `/api/recent` | Recent files |
| GET | `/api/trash` | List trash |
| DELETE | `/api/trash/:path` | Move to trash |
| POST | `/api/trash/restore` | Restore from trash |
| DELETE | `/api/trash/purge` | Purge trash |
| DELETE | `/api/trash/empty` | Empty trash |
| POST | `/api/bulk/delete` | Bulk delete |
| POST | `/api/batch/copy` | Batch copy |
| POST | `/api/batch/move` | Batch move |
| POST | `/api/batch/delete` | Batch delete |
| POST | `/api/batch/share` | Batch share |
| GET | `/api/calendar/events` | List calendar events |
| POST | `/api/calendar/events` | Create event |
| PUT | `/api/calendar/events/:uid` | Update event |
| DELETE | `/api/calendar/events/:uid` | Delete event |
| GET | `/api/contacts` | List contacts |
| POST | `/api/contacts` | Create contact |
| PUT | `/api/contacts/:uid` | Update contact |
| DELETE | `/api/contacts/:uid` | Delete contact |
| GET | `/api/contacts/export` | Export contacts |
| POST | `/api/contacts/import` | Import contacts |
| GET | `/api/chat/rooms` | List chat rooms |
| POST | `/api/chat/rooms` | Create chat room |
| GET | `/api/chat/rooms/:room_id/messages` | Get chat messages |
| POST | `/api/chat/rooms/:room_id/messages` | Send chat message |
| GET | `/api/photos` | List photos |
| GET | `/api/photos/albums` | List albums |
| POST | `/api/photos/albums` | Create album |
| GET | `/api/photos/thumbnail/:path` | Photo thumbnail |
| GET | `/api/photos/exif/:path` | Photo EXIF data |
| GET | `/api/notes` | List notes |
| POST | `/api/notes` | Create note |
| GET | `/api/notes/:id` | Get note |
| PUT | `/api/notes/:id` | Update note |
| DELETE | `/api/notes/:id` | Delete note |
| GET | `/api/notes/search` | Search notes |
| GET | `/api/tasks` | List tasks |
| POST | `/api/tasks` | Create task |
| GET | `/api/tasks/:id` | Get task |
| PUT | `/api/tasks/:id` | Update task |
| DELETE | `/api/tasks/:id` | Delete task |
| PATCH | `/api/tasks/:id/status` | Move task |
| GET | `/api/e2ee/key/generate` | Generate E2EE key |
| POST | `/api/e2ee/encrypt` | E2EE encrypt |
| POST | `/api/push/register` | Register push token |
| POST | `/api/push/unregister` | Unregister push token |
| GET | `/api/push/tokens` | List push tokens |
| GET | `/api/stream` | Video streaming |
| GET | `/api/whiteboard` | List whiteboards |
| POST | `/api/whiteboard` | Create whiteboard |
| GET | `/api/whiteboard/:id` | Get whiteboard |
| PUT | `/api/whiteboard/:id` | Save whiteboard |
| GET | `/api/whiteboard/:id/image` | Export whiteboard image |
| POST | `/api/offline/sync` | Trigger offline sync |
| GET | `/api/offline/status` | Offline status |
| GET | `/api/offline/pending` | List pending operations |
| POST | `/api/offline/resolve/:id` | Resolve conflict |
| GET | `/api/offline/cached` | List cached items |
| POST | `/api/antivirus/scan/:path` | Scan file for malware |
| GET | `/api/antivirus/status` | Antivirus status |
| POST | `/api/antivirus/scan-all` | Scan all files |
| GET | `/api/antivirus/history` | Scan history |
| GET | `/api/dlp/policies` | List DLP policies |
| POST | `/api/dlp/policies` | Create DLP policy |
| PUT | `/api/dlp/policies/:id` | Update DLP policy |
| DELETE | `/api/dlp/policies/:id` | Delete DLP policy |
| POST | `/api/dlp/scan/:path` | DLP scan file |
| GET | `/api/dlp/alerts` | List DLP alerts |
| GET | `/api/mail/accounts` | List mail accounts |
| POST | `/api/mail/accounts` | Create mail account |
| DELETE | `/api/mail/accounts/:id` | Delete mail account |
| GET | `/api/mail/accounts/:id/folders` | List mail folders |
| GET | `/api/mail/accounts/:id/folders/:folder/messages` | List mail messages |
| GET | `/api/mail/accounts/:id/folders/:folder/messages/:uid` | Mail message detail |
| POST | `/api/mail/accounts/:id/send` | Send email |
| POST | `/api/mail/accounts/:id/folders/:folder/messages/:uid/attachments/:part/download` | Download attachment |
| GET | `/api/analytics/overview` | Analytics overview |
| GET | `/api/analytics/links` | Link analytics |
| GET | `/api/analytics/links/:id/stats` | Link stats |
| POST | `/api/watermark/preview` | Watermark preview |
| POST | `/api/watermark/apply/:path` | Apply watermark |
| GET | `/api/watermark/policies` | List watermark policies |
| POST | `/api/watermark/policies` | Create watermark policy |
| GET | `/api/api-keys` | List API keys |
| POST | `/api/api-keys` | Create API key |
| DELETE | `/api/api-keys/:id` | Delete API key |
| GET | `/api/notification-prefs` | Get notification preferences |
| PUT | `/api/notification-prefs` | Update notification preferences |
| POST | `/api/policies` | Create policy |
| GET | `/api/policies` | List policies |
| DELETE | `/api/policies` | Delete policy |

#### GraphQL
| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/graphql` | GraphQL Playground |
| POST | `/api/graphql` | GraphQL endpoint |

#### Sync Protocol
| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/sync/events` | Sync events (SSE) |
| GET | `/api/sync/delta` | Sync delta |
| GET | `/api/sync/status` | Sync status |
| GET | `/api/sync/blocks/manifest` | Block sync manifest |
| POST | `/api/sync/blocks/upload` | Upload blocks |
| GET | `/api/sync/blocks/check` | Check blocks |
| POST | `/api/sync/blocks/assemble` | Assemble file from blocks |
| GET | `/api/sync/blocks/:hash` | Get block |
| GET | `/api/sync/profiles` | List sync profiles |
| POST | `/api/sync/profiles` | Create sync profile |
| PUT | `/api/sync/profiles/:id` | Update sync profile |
| DELETE | `/api/sync/profiles/:id` | Delete sync profile |
| POST | `/api/sync/filter-preview` | Sync filter preview |

#### Metrics & Health
| Method | Path | Auth Required | Notes |
|--------|------|---------------|-------|
| GET | `/metrics` | No | Prometheus metrics |
| GET | `/metrics/prometheus` | No | Prometheus metrics |
| GET | `/healthz` | No | Liveness probe |
| GET | `/health` | No | Health check |
| GET | `/readyz` | No | Readiness probe |
| GET | `/startupz` | No | Startup probe |
| GET | `/.well-known/ferro` | No | Server info |
| GET | `/.well-known/webfinger` | No | WebFinger (federation) |
| GET | `/fed/actor/:username` | No | ActivityPub actor |
| GET | `/fed/actor/:username/followers` | No | Followers |
| GET | `/fed/actor/:username/following` | No | Following |
| POST | `/fed/inbox` | No | ActivityPub inbox |
| GET | `/fed/outbox` | No | ActivityPub outbox |
| GET | `/fed/nodeinfo` | No | NodeInfo |

### 2.3 WebSocket Endpoints

| Path | Notes |
|------|-------|
| `/api/ws` | General WebSocket |
| `/ws/collab/:document_id` | Collaborative editing WebSocket |
| `/ws/chat/:room_id` | Chat WebSocket |

### 2.4 CalDAV / CardDAV / WebDAV

| Path | Notes |
|------|-------|
| `/dav/cal` | CalDAV options |
| `/dav/cal/` | CalDAV calendar list / create |
| `/dav/cal/:calendar` | CalDAV calendar operations |
| `/dav/cal/:calendar/:uid` | CalDAV event operations |
| `/dav/card` | CardDAV options |
| `/dav/card/` | CardDAV address book list / create |
| `/dav/card/:book` | CardDAV address book operations |
| `/dav/card/:book/:uid` | CardDAV contact operations |
| `/` (catch-all) | WebDAV PROPFIND, MKCOL, COPY, MOVE, LOCK, UNLOCK, PROPPATCH, GET, PUT, DELETE |

### 2.5 WOPI (Office Online Integration)

| Path | Notes |
|------|-------|
| `/wopi/*` | WOPI file operations |
| `/hosting/discovery` | WOPI discovery |

### 2.6 Static & Share Endpoints

| Path | Auth Required | Notes |
|------|---------------|-------|
| `/ui` | No | SPA entry point |
| `/s/:token` | No | Public share link (GET/POST) |
| `/s/:token/upload` | No | Upload to share |
| `/s/:token/uploads` | No | List share uploads |
| `/s/:token/view` | No | View share |
| `/remote/{*path}` | Varies | Remote mount proxy |

### 2.7 Authentication Mechanisms

1. **OIDC (OpenID Connect)**: Full PKCE flow with JWKS key caching. Token validation via `OidcValidator`. When OIDC is not configured, requests proceed with anonymous claims.
2. **Simple Auth**: Basic username/password for local admin user. Password hashing via Argon2.
3. **API Keys**: Token-based authentication with separate key store. Keys bypass OIDC validation and generate synthetic claims.
4. **Basic Auth**: WebDAV endpoints support HTTP Basic auth (`WWW-Authenticate: Basic realm="Ferro"`).
5. **WebAuthn/FIDO2**: Feature-gated (`webauthn` feature). Registration and authentication flows.
6. **TOTP**: Two-factor authentication setup, enable, disable, and status.
7. **Cedar**: Policy-based authorization engine layered on top of OIDC/Simple Auth.

### 2.8 Existing Security Measures

- **CORS**: Configurable allowed origins. Logs warning when `*` is used with auth enabled.
- **Rate Limiting**: Token bucket rate limiter (global). Tenant-level rate limiting via `X-Tenant-ID` header.
- **Security Headers**: CSP, HSTS (HTTPS only), X-Frame-Options: DENY, X-Content-Type-Options: nosniff, Referrer-Policy, Permissions-Policy.
- **Request Smuggling Protection**: Middleware rejects requests with both Content-Length and Transfer-Encoding headers.
- **Request ID**: UUID-based request tracking (`X-Request-Id`).
- **Body Size Limit**: Configurable `max_body_size` via `DefaultBodyLimit`.
- **Concurrency Limit**: `ConcurrencyLimitLayer` to cap in-flight requests.
- **Maintenance Mode**: Blocks write operations (allows reads and maintenance toggle).
- **Default Password Enforcement**: Forces password change when default admin password is in use.
- **Panic Handler**: Logs request context on 500 errors.
- **Audit Middleware**: Request logging with status codes and durations.

## 3. Test Categories

### 3.1 Authentication & Authorization

| ID | Test Case | Severity | Target |
|----|-----------|----------|--------|
| AUTH-01 | OIDC token validation bypass (empty, expired, malformed, wrong audience/issuer) | Critical | `/api/*` |
| AUTH-02 | JWT algorithm confusion (e.g., `none` algorithm, RS256→HS256) | Critical | `/api/*` |
| AUTH-03 | Session fixation / hijacking via OIDC state parameter | High | `/api/auth/login`, `/api/auth/callback` |
| AUTH-04 | PKCE state parameter reuse or prediction | High | `/api/auth/callback` |
| AUTH-05 | API key brute force (key format, entropy analysis) | High | `/api/api-keys` |
| AUTH-06 | API key timing attack during comparison | Medium | Auth middleware |
| AUTH-07 | Privilege escalation: user → admin via role assignment | Critical | `/api/admin/users/:id/role` |
| AUTH-08 | Horizontal privilege access: user A accessing user B's files | Critical | `/api/files/*`, `/api/v1/files/*` |
| AUTH-09 | Bypass auth via X-Ferro-User header injection | Critical | All authenticated endpoints |
| AUTH-10 | Anonymous claims escalation when auth is disabled | High | All endpoints (auth disabled mode) |
| AUTH-11 | Public path bypass (path traversal to public endpoints) | High | `is_public_auth_path()` boundary |
| AUTH-12 | TOTP bypass (replay, prediction of secret) | High | `/api/auth/totp/*` |
| AUTH-13 | WebAuthn attestation bypass | High | `/api/auth/webauthn/*` |
| AUTH-14 | Password reset token prediction | High | `/api/admin/users/:id/reset-password` |
| AUTH-15 | Cedar policy bypass via malformed policy input | Critical | Policy engine |
| AUTH-16 | Guest token escalation to user/admin | High | Guest middleware |
| AUTH-17 | OIDC JWKS cache poisoning (fetching attacker-controlled JWKS) | Critical | `OidcValidator::refresh_jwks()` |
| AUTH-18 | OIDC discovery endpoint manipulation | High | OIDC discovery fetch |
| AUTH-19 | Default password enforcement bypass | High | `default_password_layer` |
| AUTH-20 | WebDAV Basic Auth brute force | High | `/` (WebDAV) |

### 3.2 Input Validation

| ID | Test Case | Severity | Target |
|----|-----------|----------|--------|
| INPUT-01 | SQL injection in all query parameters | Critical | All list/search endpoints |
| INPUT-02 | Path traversal in file operations (`../../etc/passwd`) | Critical | `/api/files/*`, `/api/v1/files/*`, upload/download |
| INPUT-03 | Path traversal in share token resolution | High | `/s/:token` |
| INPUT-04 | XML injection in WebDAV PROPFIND/REPORT/PROPPATCH | High | `/` (WebDAV), `/dav/*` |
| INPUT-05 | XML External Entity (XXE) via WebDAV | Critical | `/` (WebDAV), `/dav/*` |
| INPUT-06 | Malformed HTTP headers (CRLF injection) | Medium | All endpoints |
| INPUT-07 | HTTP request smuggling (CL-TE, TE-CL) | High | All endpoints (mitigated by smuggling_rejection_middleware) |
| INPUT-08 | SSRF via federation endpoints (`/fed/files/*`, `/remote/*`) | High | Federation proxy, remote mount proxy |
| INPUT-09 | SSRF via WebFinger resolution | Medium | `/.well-known/webfinger` |
| INPUT-10 | SSRF via OIDC discovery (attacker-controlled issuer) | High | `OidcValidator::exchange_code()` |
| INPUT-11 | GraphQL injection / introspection abuse | Medium | `/api/graphql` |
| INPUT-12 | WASM module upload: malicious WASM binary | Critical | `/api/wasm/upload` |
| INPUT-13 | Filename sanitization bypass (null bytes, special chars) | High | File operations |
| INPUT-14 | CalDAV/CardDAV vCard/iCal injection | Medium | `/dav/cal/*`, `/dav/card/*` |
| INPUT-15 | Chat message injection (stored XSS) | Medium | `/api/chat/rooms/:room_id/messages` |
| INPUT-16 | Contact import vCard injection | Medium | `/api/contacts/import` |
| INPUT-17 | Batch operation parameter injection | Medium | `/api/batch/*` |
| INPUT-18 | Upload filename path traversal (chunked upload) | High | `/api/upload/*` |
| INPUT-19 | EXIF data extraction for information leakage | Low | `/api/photos/exif/:path` |
| INPUT-20 | Remote mount proxy SSRF (internal network access) | High | `/remote/*` |
| INPUT-21 | Webhook URL SSRF | High | `/api/admin/webhooks` |

### 3.3 Cryptography

| ID | Test Case | Severity | Target |
|----|-----------|----------|--------|
| CRYPTO-01 | TLS configuration review (cipher suites, protocols) | High | Server TLS termination |
| CRYPTO-02 | E2EE implementation review (key derivation, storage, transport) | Critical | `/api/e2ee/*`, `/api/files/encrypt` |
| CRYPTO-03 | Secret storage review (environment variables, config files) | High | Configuration |
| CRYPTO-04 | API key entropy and format analysis | High | `/api/api-keys` |
| CRYPTO-05 | JWT signing key strength | High | OIDC validator |
| CRYPTO-06 | Password hashing parameters (Argon2 memory/time cost) | Medium | Simple auth |
| CRYPTO-07 | WOPI token secret strength | Medium | WOPI integration |
| CRYPTO-08 | Presigned URL token prediction / forgery | High | `/api/upload-url`, `/api/download-url` |
| CRYPTO-09 | Share token entropy analysis | High | `/s/:token` |
| CRYPTO-10 | Encryption at rest review | High | Storage layer |
| CRYPTO-11 | TLS certificate validation in outbound requests (OIDC, federation) | High | HTTP client |

### 3.4 API Security

| ID | Test Case | Severity | Target |
|----|-----------|----------|--------|
| API-01 | Rate limiting bypass (IP spoofing via X-Forwarded-For) | Medium | Rate limit middleware |
| API-02 | Rate limiting bypass (missing X-Forwarded-For) | Medium | Rate limit middleware |
| API-03 | CORS misconfiguration (credential theft) | High | CORS layer |
| API-04 | Content-Type validation bypass | Medium | All endpoints |
| API-05 | Request size limits bypass (chunked encoding abuse) | Medium | Body limit layer |
| API-06 | Error message information leakage (stack traces, internal paths) | Medium | All error responses |
| API-07 | Verbose error messages revealing internal state | Low | All error responses |
| API-08 | HTTP method override (X-HTTP-Method-Override) | Medium | All endpoints |
| API-09 | API versioning bypass (deprecated v1 vs current) | Low | `/api/` vs `/api/v1/` |
| API-10 | Concurrency limit bypass | Medium | ConcurrencyLimitLayer |
| API-11 | Missing audit logging for sensitive operations | Medium | Audit middleware |
| API-12 | Tenant rate limit bypass via missing X-Tenant-ID header | Medium | Tenant rate limit layer |
| API-13 | OpenAPI spec information disclosure | Low | `/api/swagger-ui` |

### 3.5 Infrastructure

| ID | Test Case | Severity | Target |
|----|-----------|----------|--------|
| INFRA-01 | Docker container escape | Critical | Docker deployment |
| INFRA-02 | File permission issues (world-readable secrets) | High | Config files, data directory |
| INFRA-03 | Dependency vulnerability scan (cargo-audit) | High | Cargo.lock |
| INFRA-04 | Supply chain risks (malicious dependencies) | Medium | Cargo.lock, crates.io |
| INFRA-05 | WASM sandbox escape (if WASM modules execute host code) | Critical | WASM execution engine |
| INFRA-06 | Backup file exposure | High | `/api/admin/backup/download` |
| INFRA-07 | Log injection / log forging | Medium | Logging middleware |
| INFRA-08 | Time-of-check to time-of-use (TOCTOU) in file operations | Medium | File operations |
| INFRA-09 | Resource exhaustion via large uploads or many connections | Medium | Upload, concurrency limit |
| INFRA-10 | Metrics endpoint information disclosure | Low | `/metrics`, `/metrics/prometheus` |

## 4. Out of Scope

- **Social engineering**: Phishing, pretexting, or any human-targeted attacks.
- **Physical security**: Physical access to servers, data centers, or endpoints.
- **Denial of Service (DoS/DDoS)**: Intentional service disruption is not a primary focus, though resource exhaustion from malformed requests is in scope.
- **Web UI frontend vulnerabilities**: XSS in the SPA, browser-specific issues. (Server-side API responses are in scope.)
- **Client-side mobile/desktop applications**.
- **Third-party integrations** (OIDC provider internals, WOPI office server internals).
- **DNS and network-level attacks** (BGP hijacking, DNS poisoning).

## 5. Recommended Tools

| Tool | Purpose |
|------|---------|
| **nuclei** | Template-based vulnerability scanning |
| **nmap / masscan** | Port scanning and service enumeration |
| **sqlmap** | SQL injection testing |
| **Burp Suite / OWASP ZAP** | HTTP proxy, manual testing, fuzzing |
| **cargo-audit** | Rust dependency vulnerability scanning |
| **cargo-fuzz / libFuzzer** | Rust-specific fuzzing (file parsers, XML, JSON) |
| **ffuf / feroxbuster** | Directory and endpoint brute forcing |
| **jwt_tool** | JWT token manipulation |
| **Postman / httpie** | API manual testing |
| **Wireshark** | TLS configuration analysis |
| **semgrep** | Static analysis for Rust security patterns |

## 6. Severity Classification

| Severity | Definition | Examples |
|----------|------------|----------|
| **Critical** | Direct impact on confidentiality, integrity, or availability. Remote code execution, authentication bypass, full data exfiltration. | RCE via WASM, OIDC bypass, path traversal to /etc/passwd, SQL injection with data exfiltration, XXE |
| **High** | Significant security impact. Privilege escalation, partial data access, meaningful authorization bypass. | User→admin escalation, horizontal file access, SSRF to internal network, JWT key confusion |
| **Medium** | Limited security impact. Information disclosure, CSRF, configuration weaknesses. | Verbose errors, CORS misconfiguration, rate limiting bypass, log injection |
| **Low** | Minor issues, defense-in-depth improvements. | Missing security headers, verbose metrics, informational error codes |

## 7. Reporting Format

Each finding should be reported using the following template:

```markdown
### [SEVERITY] FINDING-XXX: Short descriptive title

**Affected Component**: Module/endpoint/file path
**Endpoint**: HTTP method + path
**CVSS Score**: X.X (if applicable)
**CWE**: CWE-XXX

**Description**:
Clear explanation of the vulnerability.

**Reproduction Steps**:
1. Step-by-step instructions
2. Request/response examples
3. Expected vs actual behavior

**Impact**:
What an attacker could achieve.

**Remediation**:
Specific fix recommendations.

**References**:
- Links to relevant CVEs, standards, or documentation
```

---

*Document generated from codebase analysis of the `ferro` workspace. Routes extracted from `crates/server/src/routes.rs`. Security measures analyzed from `crates/server-security-middleware/` and `crates/common/src/auth.rs`.*
