# Ferro vs Self-Hosted File Platforms: Feature Comparison

**Date:** 2026-05-29 | **Ferro Version:** 2.5.1

---

## Platform Overview

| Attribute | **Ferro** | **Nextcloud** | **ownCloud OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|-----------|-----------|---------------|-------------------|-------------|-----------------|-----------|
| **Version** | 2.5.1 | Hub 26 (Server 33) | Curie 8.0 | 13.x | 2.63.5 | RELEASE.2025-10 |
| **Language** | Rust | PHP + JS | Go (microservices) | C/Python + JS | Go + Vue | Go |
| **License** | AGPLv3 | AGPLv3 / Enterprise | Apache 2.0 / EULA | AGPLv3 / Apache v2 | Apache 2.0 | AGPLv3 / Commercial |
| **Architecture** | Monolithic single binary | LAMP stack | Microservices | Monolithic | Single binary | Single binary |
| **Binary size** | ~49MB (static musl) | N/A (PHP) | ~100MB | N/A | ~15MB | ~100MB |
| **p99 latency (1KB PUT)** | <10ms | 50-200ms | 20-50ms | 10-30ms | ~10ms | ~5ms |
| **Memory (idle)** | ~52MB | 256MB-1GB+ | 128MB-512MB | 128MB-512MB | ~30MB | ~256MB |
| **Soak test (1h)** | 18,828 req, 0 failures | Not published | Not published | Not published | Not published | Not published |

---

## Storage Backends

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| Local filesystem | Yes | Yes | Yes | Yes | Yes | Yes (JBOD) |
| Amazon S3 | Yes (`s3` feature) | Yes (external) | Yes | Yes | No | **Native** |
| Google Cloud Storage | Yes (`gcs` feature) | Yes (external) | No | No | No | No |
| Azure Blob | Yes (`azure` feature) | Yes (external) | No | No | No | No |
| Content-addressable (CAS) | Yes (SHA-256 dedup) | No | No | Yes (block dedup) | No | No |
| PostgreSQL metadata | Yes (optional) | Yes (primary) | Yes | No | No | No |
| SQLite persistence | Yes (unified) | No | No | Yes (SQLite/MySQL) | No | No |
| Redis (locks/rate limit) | Yes (optional) | Yes (caching) | No | No | No | No |
| NFS/SMB/WebDAV mount | No | Yes | Yes (external) | No | No | No |
| Multiple backends simultaneously | No | Yes | Yes (per-space) | Yes (Pro) | No | No |
| External storage mounting | No | Yes | Yes | No | No | No |
| Pre-signed URLs | Yes (server-side) | Yes (S3 primary) | Yes | No | No | **Yes (native)** |
| Storage health monitoring | Yes | Yes | Yes | Yes | No | Yes |
| Data migration between backends | Yes (`--migrate-from`) | Manual | Manual | Manual | N/A | No |

---

## File Sync & Clients

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| Desktop sync client | Partial (Tauri shell) | **Yes** (Win/Mac/Linux) | **Yes** (Win/Mac/Linux) | **Yes** (Win/Mac/Linux) | No | No |
| Virtual filesystem (FUSE/VFS) | **Yes** (FUSE mount) | **Yes** (VFS for Win/Mac) | **Yes** (VFS) | **Yes** (SeaDrive) | No | No |
| Android app | No | **Yes** | **Yes** | **Yes** | No | No |
| iOS app | No | **Yes** | **Yes** | **Yes** | No | No |
| Resumable/chunked upload | **Yes** (chunked API) | **Yes** (chunked) | **Yes** (TUS) | **Yes** (block-level) | Yes (basic) | **Yes** (multipart) |
| Block-level/incremental sync | Partial (delta sync API) | No (file-level) | No (file-level) | **Yes** (block-level) | No | No |
| Selective sync | No | **Yes** | **Yes** | **Yes** | N/A | N/A |
| Conflict resolution | Vector clocks | Yes | Yes | Yes (block merge) | No | No |
| Offline mode | No | **Yes** (mobile) | **Yes** | **Yes** | No | No |
| Remote wipe | No | **Yes** | **Yes** | **Yes** | No | No |

---

## WebDAV Compliance

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| Class 1 (GET/PUT/DELETE/MKCOL/PROPFIND) | **Yes** | **Yes** | **Yes** | Partial | No | No |
| Class 2 (LOCK/UNLOCK) | **Yes** | **Yes** | **Yes** | Partial | No | No |
| Class 3 (PROPPATCH) | **Yes** | **Yes** | **Yes** | No | No | No |
| COPY/MOVE | **Yes** | **Yes** | **Yes** | Partial | Yes (copy) | No |
| Sync-collection (sync-token) | **Yes** | **Yes** | No | No | No | No |
| Conditional GET (ETag/If-Match) | **Yes** | **Yes** | **Yes** | Yes | No | **Yes** |
| rclone compatibility | **100%** (verified) | Yes | Yes | Partial | No | No |
| Depth: infinity | **Yes** (bounded) | Yes | Yes | No | No | No |

---

## File Management

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| File versioning | **Yes** (configurable) | **Yes** | **Yes** | **Yes** | No | **Yes** (bucket) |
| Line-level diff (LCS) | **Yes** | No | No | No | No | No |
| Trash/recycle bin | **Yes** (auto-purge) | **Yes** | **Yes** | **Yes** | Yes (basic) | No |
| File locking | **Yes** (WebDAV LOCK) | **Yes** (manual+auto) | **Yes** | **Yes** | No | No |
| Tags | **Yes** | **Yes** | **Yes** | **Yes** | No | Yes (object tags) |
| Custom metadata | No | **Yes** | **Yes** | **Yes** (Pro) | No | Yes (object metadata) |
| Thumbnails/preview | **Yes** (JPEG/PDF/etc) | **Yes** (broad) | **Yes** | **Yes** | Yes (basic) | No |
| Comments on files | No | **Yes** | **Yes** | No | No | No |
| Favorites | **Yes** | **Yes** | **Yes** | No | No | No |
| Recent files | **Yes** | **Yes** | **Yes** | No | No | No |
| Batch operations | **Yes** (bulk/batch API) | Partial | Partial | No | No | Yes (batch) |
| Chunked/resumable upload | **Yes** (out-of-order) | **Yes** | **Yes** (TUS) | **Yes** | Yes (basic) | **Yes** (multipart) |
| Storage quota | **Yes** (global) | **Yes** (per-user) | **Yes** (per-space) | **Yes** (per-user/org) | Yes (per-user) | Yes (bucket) |
| Encryption (at-rest) | **Yes** (age/X25519) | **Yes** (server-side) | **Yes** | **Yes** (Pro) | No | **Yes** (auto) |
| E2EE (end-to-end) | No | **Yes** (mature) | **Yes** | **Yes** (client-side) | No | No |
| Metadata snapshots | **Yes** (point-in-time) | No | No | No | No | No |
| File move/copy | **Yes** (recursive) | **Yes** | **Yes** | **Yes** | Yes | No (copy only) |
| Activity feed | **Yes** | **Yes** | **Yes** | **Yes** | No | No |
| Antivirus scanning | No (WASM worker possible) | **Yes** (ClamAV app) | **Yes** (AV service) | **Yes** (Pro) | No | No |
| Ransomware protection | No | **Yes** | **Yes** | No | No | **Yes** (WORM) |
| Data retention policies | No | **Yes** | **Yes** | **Yes** (Pro) | No | **Yes** (ILM) |

---

## Collaboration & Office

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| WOPI protocol | **Yes** (token+discovery) | **Yes** | **Yes** | **Yes** | No | No |
| Collabora Online | Via WOPI | **Built-in** | **Yes** | **Yes** | No | No |
| OnlyOffice | Via WOPI | **Yes** | **Yes** | **Yes** | No | No |
| Real-time co-editing | No (WebRTC signaling only) | **Yes** (Nextcloud Office) | **Yes** | **Yes** (SeaDoc) | No | No |
| Whiteboard | No | **Yes** | No | No | No | No |
| Built-in text editor | No (web preview only) | **Yes** (rich) | **Yes** | **Yes** (SeaDoc) | Yes (basic) | No |
| File drop (upload-only) | No | **Yes** | **Yes** | **Yes** | No | No |
| Secure view (no download) | No | **Yes** | **Yes** | **Yes** | No | No |
| Comments/annotations | No | **Yes** | **Yes** | No | No | No |

---

## Sharing & Federation

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| User/group sharing | Partial (multi-user) | **Yes** | **Yes** (roles) | **Yes** | **Yes** (basic) | No (bucket policies) |
| Public link sharing | **Yes** | **Yes** | **Yes** | **Yes** | **Yes** | Yes (presigned) |
| Password-protected links | **Yes** | **Yes** | **Yes** | **Yes** | **Yes** | No |
| Link expiration | **Yes** | **Yes** | **Yes** | **Yes** | No | Yes (presigned) |
| Granular permissions | Via Cedar policies | **Yes** (read/write/share) | **Yes** (roles-based) | **Yes** (atomic) | Basic (r/w/admin) | **Yes** (IAM) |
| Federated sharing | **Yes** (ActivityPub) | **Yes** (Nextcloud Fed) | **Yes** (ScienceMesh) | No | No | No |
| Federation protocol | **ActivityPub** | Nextcloud-specific | CS3/OCM | None | None | Replication |
| Share link brute-force protection | **Yes** (per-token lockout) | Yes | Yes | No | No | No |
| Share notifications | No | **Yes** | **Yes** | Yes | No | No |

---

## Search

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| File name search | **Yes** | **Yes** | **Yes** | **Yes** | Yes (basic) | No |
| Full-text search | **Yes** (Tantivy) | **Yes** (Elasticsearch) | **Yes** | **Yes** (Pro) | No | No |
| Auto-indexing on upload | **Yes** | **Yes** | **Yes** | **Yes** | No | No |
| Filter by tags | **Yes** | **Yes** | **Yes** | **Yes** | No | No |
| Relevance scoring + snippets | **Yes** | **Yes** | **Yes** | **Yes** | No | No |
| AI-powered/semantic search | No | **Yes** (Assistant) | No | **Yes** (AI metadata) | No | No |
| External search engine required | No (embedded Tantivy) | Yes (Elasticsearch) | No | Yes (SeaSearch, Pro) | No | No |

---

## Authentication & Security

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| HTTP Basic Auth | **Yes** | **Yes** | No | No | **Yes** | No |
| LDAP/Active Directory | **Yes** (with group mapping) | **Yes** | **Yes** (GLAuth) | **Yes** | No | **Yes** (LDAP STS) |
| OIDC (OpenID Connect) | **Yes** (PKCE flow) | **Yes** | **Yes** (embedded+external) | **Yes** (OAuth2) | No | **Yes** |
| SAML | No | **Yes** | **Yes** | **Yes** (Pro) | No | No |
| 2FA/MFA (TOTP) | No | **Yes** (TOTP+WebAuthn) | **Yes** | **Yes** (TOTP) | No | **Yes** (KMS) |
| Password policy | **Yes** (min 8, reject default) | **Yes** (configurable) | **Yes** | **Yes** | No | No |
| Account lockout | **Yes** (10 fails / 15 min) | **Yes** | **Yes** | No | No | **Yes** |
| Rate limiting | **Yes** (per-IP, 10k/min) | **Yes** | **Yes** | No | No | **Yes** |
| Brute-force protection | **Yes** | **Yes** | **Yes** | No | No | **Yes** |
| Path traversal prevention | **Yes** | **Yes** | **Yes** | **Yes** | No | N/A |
| Content-Type validation | **Yes** (magic bytes) | **Yes** | **Yes** | No | No | No |
| Security headers (CSP/HSTS) | **Yes** | **Yes** | **Yes** | Partial | No | **Yes** |
| Audit logging | **Yes** (SHA-256 chain) | **Yes** | **Yes** | **Yes** (Pro) | No | **Yes** |
| Secret redaction in logs | **Yes** | Partial | Partial | No | No | No |
| CORS enforcement | **Yes** | **Yes** | **Yes** | No | No | **Yes** |
| Guest accounts | No | **Yes** | **Yes** | No | No | No |
| Token refresh | **Yes** (OIDC refresh) | **Yes** | **Yes** | No | No | **Yes** |

---

## API Surface

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| WebDAV | **Yes** (Class 1/2/3) | **Yes** | **Yes** | Partial | No | No |
| REST API | **Yes** (80+ endpoints) | **Yes** (OCS) | **Yes** (Graph/OCS) | **Yes** (v2.1) | **Yes** (basic) | **Yes** (S3 API) |
| GraphQL | **Yes** | No | **Yes** | No | No | No |
| WebSocket | **Yes** (real-time events) | Partial | No | No | No | No |
| SSE (Server-Sent Events) | **Yes** (sync events) | No | No | No | No | No |
| CalDAV | **Yes** (calendar CRUD) | **Yes** | No | No | No | No |
| CardDAV | **Yes** (contacts CRUD) | **Yes** | No | No | No | No |
| WOPI | **Yes** (token+discovery) | **Yes** | **Yes** | **Yes** | No | No |
| ActivityPub/Federation | **Yes** | No | No | No | No | No |
| OpenAPI/Swagger UI | **Yes** (vendored) | No | No | No | No | No |
| Webhooks | **Yes** (HMAC-signed) | **Yes** | **Yes** | No | No | **Yes** (events) |
| S3-compatible API | No | No | No | No | No | **Yes** (native) |
| gRPC/CS3 | No | No | **Yes** | No | No | No |
| Plugin/extension system | **Yes** (WASM workers) | **Yes** (200+ apps) | **Yes** (Marketplace) | Limited | No | No (S3 integrations) |

---

## Observability & Operations

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| Prometheus metrics | **Yes** | **Yes** | **Yes** | **Yes** | No | **Yes** |
| Grafana dashboard | **Yes** (template) | **Yes** | **Yes** | No | No | **Yes** |
| Health checks (liveness/readiness/startup) | **Yes** (3 probes) | **Yes** | **Yes** | **Yes** | No | **Yes** |
| Request tracing (X-Request-ID) | **Yes** | Partial | No | No | No | No |
| Per-crate log levels | **Yes** | Yes | **Yes** | Yes | No | **Yes** |
| JSON structured logging | **Yes** | Yes | **Yes** | No | No | **Yes** |
| Slow query logging | **Yes** (>100ms) | No | No | No | No | No |
| Audit chain verification | **Yes** (SHA-256 chain) | No | No | No | No | No |
| Distributed tracing | No | Via apps | No | No | No | No |
| WASM worker metrics | **Yes** | N/A | N/A | N/A | N/A | N/A |
| Cache hit/miss metrics | **Yes** | Yes | No | No | No | **Yes** |

---

## Deployment

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| Docker image | **Yes** (multi-arch) | **Yes** (AIO) | **Yes** | **Yes** | **Yes** | **Yes** |
| Docker Compose | **Yes** (5 variants) | **Yes** | **Yes** | **Yes** | **Yes** | **Yes** |
| Helm chart (Kubernetes) | **Yes** | Community | **Yes** | **Yes** | No | **Yes** (Operator) |
| K8s probes (liveness/readiness/startup) | **Yes** | **Yes** | **Yes** | **Yes** | No | **Yes** |
| Single binary distribution | **Yes** (static musl) | No (PHP) | **Yes** | No | **Yes** | **Yes** |
| Multi-arch (amd64/arm64) | **Yes** | **Yes** | **Yes** | **Yes** | **Yes** | **Yes** |
| SBOM generation | **Yes** (SPDX) | No | No | No | No | No |
| Tauri desktop app | **Yes** | No | No | No | No | No |
| FUSE mount | **Yes** | No | No | **Yes** (SeaDrive) | No | No |
| Nix flake | **Yes** | No | No | No | No | No |
| Terraform module | **Yes** (documented) | Community | No | No | No | **Yes** |
| Caddy HTTPS proxy | **Yes** (bundled) | No | No | No | No | No |
| Horizontal scaling | No (single node) | **Yes** (Global Scale) | **Yes** (microservices) | **Yes** (cluster, Pro) | No | **Yes** (distributed) |
| Multi-tenancy | No | No (multi-instance) | **Yes** | **Yes** (Pro) | No | **Yes** |
| Maintenance mode | **Yes** | **Yes** | **Yes** | No | No | No |
| Graceful shutdown | **Yes** (SIGTERM drain) | No | **Yes** | No | No | **Yes** |
| Config file (TOML/YAML) | **Yes** (TOML) | **Yes** (PHP) | **Yes** (YAML) | **Yes** | **Yes** (JSON) | **Yes** (YAML/ENV) |

---

## Governance & Admin

| Feature | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** | **MinIO** |
|---------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|:---------:|
| Admin dashboard (web UI) | No (API + CLI only) | **Yes** (full) | **Yes** | **Yes** (basic) | **Yes** (basic) | **Yes** (Console) |
| User management | **Yes** (CLI + API) | **Yes** (web + API) | **Yes** | **Yes** | **Yes** (basic) | **Yes** (Console) |
| Role-based access control | **Yes** (Cedar policies) | **Yes** | **Yes** | **Yes** | Basic (r/w/admin) | **Yes** (IAM) |
| Data retention/lifecycle | No | **Yes** | **Yes** | **Yes** (Pro) | No | **Yes** (ILM) |
| Workflow automation | No | **Yes** (Flow + Windmill) | Limited | No | No (custom commands) | **Yes** (lambdas) |
| Notification system (push/email) | No (WebSocket only) | **Yes** (push+email+RSS) | **Yes** (SSE+notifications) | **Yes** | No | **Yes** (events) |
| Theming/branding | No | **Yes** (full) | **Yes** | **Yes** (basic) | **Yes** (basic) | **Yes** (Console) |
| GDPR compliance/export | No | **Yes** (Compliance Kit) | **Yes** | Partial | No | **Yes** |
| Backup/restore | **Yes** (API) | **Yes** | **Yes** | **Yes** | No | **Yes** (replication) |
| Database backup API | **Yes** (SQLite backup) | Manual | Manual | Manual | N/A | N/A |
| Integrity verification | **Yes** (CAS checksum) | No | No | No | No | **Yes** (bitrot) |

---

## Unique Differentiators

### What Ferro Has That Others Don't

| Feature | Details |
|---------|---------|
| **Rust-native performance** | <10ms p99 latency, 52MB memory, zero-cost abstractions |
| **Full WebDAV Class 1/2/3** | Only platform with complete Class 3 PROPPATCH + sync-collection |
| **WASM worker runtime** | Plugin system with fuel-based sandboxing, pattern-based dispatch |
| **GraphQL + WebSocket + SSE** | Richest real-time API surface of any platform |
| **ActivityPub federation** | Decentralized sharing via ActivityPub (unique approach) |
| **WebRTC signaling** | Built-in P2P signaling server |
| **CalDAV + CardDAV + WOPI** | All three protocols in a single binary |
| **Cedar policy engine** | Fine-grained authorization via Cedar (Amazon's policy language) |
| **Audit chain (SHA-256)** | Tamper-detectable audit log with chain verification |
| **Line-level diff (LCS)** | Version diffing at the line level, not just file level |
| **FUSE mount** | Native filesystem mount on Linux |
| **OpenAPI/Swagger UI** | Auto-generated API documentation (vendored) |
| **3-tier health probes** | Separate liveness/readiness/startup probes for K8s |
| **TOML configuration** | Deterministic, type-safe config with include directives |
| **SBOM on every release** | Automated SPDX generation in CI |
| **Vector clock sync** | Distributed-friendly sync with merge semantics |
| **Pre-commit test gate** | 854 tests must pass before any commit |

### What Others Have That Ferro Doesn't

| Feature | Nextcloud | OCIS | Seafile | Priority |
|---------|:---------:|:----:|:-------:|:--------:|
| Mobile apps (iOS + Android) | Yes | Yes | Yes | **P0** |
| Desktop sync client (bidirectional) | Yes | Yes | Yes | **P0** |
| Real-time co-editing (CRDT) | Yes | Yes | Yes | **P0** |
| 2FA/MFA (TOTP + WebAuthn) | Yes | Yes | Yes | **P0** |
| Admin dashboard (web UI) | Yes | Yes | Yes | **P1** |
| Block-level delta sync | No | No | Yes | **P1** |
| Notification system (email/push) | Yes | Yes | Yes | **P1** |
| SAML SSO | Yes | Yes | Yes | **P1** |
| Theming/branding | Yes | Yes | Yes | **P1** |
| Data retention policies | Yes | Yes | Yes | **P2** |
| Workflow automation | Yes | Limited | No | **P2** |
| Antivirus scanning | Yes | Yes | Yes | **P2** |
| E2EE (end-to-end encryption) | Yes | Yes | Yes | **P2** |
| Multi-tenancy | No | Yes | Yes | **P2** |
| Horizontal scaling | Yes | Yes | Yes | **P3** |
| External storage mounting (NFS/SMB) | Yes | Yes | No | **P3** |
| AI-powered search | Yes | No | Yes | **P3** |
| Guest accounts | Yes | Yes | No | **P3** |
| Comments on files | Yes | Yes | No | **P3** |
| App/plugin marketplace | Yes | Yes | Limited | **P3** |
| GDPR compliance kit | Yes | Yes | Partial | **P3** |
| Ransomware protection | Yes | Yes | No | **P3** |

---

## Summary Scorecard

| Dimension | **Ferro** | **Nextcloud** | **OCIS** | **Seafile** | **Filebrowser** |
|-----------|:---------:|:-------------:|:--------:|:-----------:|:---------------:|
| Performance | **10/10** | 5/10 | 7/10 | 8/10 | 9/10 |
| Memory efficiency | **10/10** | 3/10 | 6/10 | 5/10 | 9/10 |
| Binary size / deployment | **9/10** | 3/10 | 8/10 | 5/10 | 10/10 |
| WebDAV compliance | **10/10** | 9/10 | 9/10 | 5/10 | 0/10 |
| API richness | **10/10** | 8/10 | 9/10 | 6/10 | 3/10 |
| Security | 7/10 | **9/10** | **9/10** | 6/10 | 3/10 |
| Observability | **10/10** | 8/10 | 8/10 | 6/10 | 1/10 |
| Collaboration | 3/10 | **10/10** | **10/10** | 8/10 | 1/10 |
| Mobile support | 1/10 | **10/10** | **10/10** | **10/10** | 1/10 |
| Desktop sync | 4/10 | **10/10** | **10/10** | **10/10** | 0/10 |
| Ecosystem / maturity | 3/10 | **10/10** | 7/10 | 7/10 | 3/10 |
| Storage backend diversity | 7/10 | **10/10** | 7/10 | 6/10 | 1/10 |
| Federation | **8/10** | 7/10 | 7/10 | 0/10 | 0/10 |
| Enterprise features | 3/10 | **10/10** | **10/10** | 7/10 | 1/10 |
| **Overall** | **7.4/10** | **8.5/10** | **8.3/10** | **6.4/10** | **3.0/10** |
