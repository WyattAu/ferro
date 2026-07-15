# ADV_FSP: Functional Specification

## Assurance Family Requirement

The developer shall provide a functional specification describing the purpose and means of use of the security functions and security mechanisms.

**EAL Level:** EAL 3+ (ADV_FSP.3)

## Evidence Artifacts

### 1. Security Function Specification

| SF-ID | Function | Implementation | Evidence |
|-------|----------|----------------|----------|
| SF-AC | Access Control | OIDC, TOTP, WebAuthn, LDAP, Cedar RBAC | `crates/auth/src/` |
| SF-AU | Audit | Chain-verified audit log with hash integrity | `crates/audit-log/` |
| SF-CP | Cryptographic Protection | SHA-256, AES-GCM-256, ECDSA P-256, TLS 1.3, Argon2id | `crates/crypto/` |
| SF-KM | Key Management | 3-level hierarchy (Master → KEK → DEK) | `crates/server-security/src/encryption.rs` |
| SF-DP | Data Protection | WORM storage, retention policies, E2EE | `crates/server-compliance/` |
| SF-IC | Integrity Control | CAS dedup, hash chain verification, signed manifests | `crates/core/` |
| SF-RC | Recovery | Snapshots, backup/restore, metadata recovery | `crates/server-storage-ops/` |

### 2. API Endpoints (REST)

| Endpoint | Method | Auth | SF Mapping | Source |
|----------|--------|------|------------|--------|
| `/.well-known/ferro` | GET | None | SF-IC (health) | `README.md:297` |
| `/api/auth/info` | GET | None | SF-AC | `README.md:299` |
| `/api/auth/login` | GET | None | SF-AC | `README.md:300` |
| `/api/auth/callback` | GET | None | SF-AC | `README.md:301` |
| `/api/search` | GET | Yes | SF-IC | `README.md:302` |
| `/api/policies` | GET/POST/DELETE | Yes | SF-AC | `README.md:307` |
| `/api/audit` | GET | Yes | SF-AU | `README.md:313` |
| `/api/snapshots` | GET/POST | Yes | SF-RC | `README.md:315` |
| `/api/shares` | GET/POST | Yes | SF-DP | `README.md:310` |
| `/api/upload-url` | GET | Yes | SF-CP | `README.md:308` |
| `/api/workers` | GET/POST | Yes | SF-DP (sandbox) | `README.md:304` |
| WebDAV (`/*path`) | Various | Yes | SF-DP | `README.md:322` |
| WOPI (`/wopi/files/*path`) | GET/POST | Yes | SF-DP | `README.md:318` |

### 3. Handler Documentation

| Handler | Purpose | Security Relevance |
|---------|---------|-------------------|
| `ferro-server` (main binary) | Axum HTTP server, WebDAV, REST API, admin endpoints | Entry point, TLS termination |
| `server-routes` | Route definitions and handler dispatch | Authorization enforcement |
| `server-security-middleware` | Auth middleware, CORS, rate limiting | Access control enforcement |
| `server-webdav-core` | WebDAV protocol handlers | File access authorization |
| `server-compliance` | WORM, retention, antivirus, DLP | Data protection enforcement |
| `server-storage-ops` | Upload, download, thumbnails, snapshots | Storage integrity |
| `server-admin-api` | Admin API handlers | Privileged operations |
| `server-sharing` | Share link management | Sharing authorization |

### 4. Data Structures

| Type | Crate | Purpose |
|------|-------|---------|
| `DbHandle` | `common` | Single-source-of-truth database handle |
| `ApiError` | `server-security-middleware` | Canonical error type |
| `AuditEntry` | `common::audit` | Audit log entry structure |
| `CedarPolicy` | `auth::cedar` | Access control policy |
| `SyncOp` | `server-state` | Synchronization operation |
| `VectorClock` | `server-state` | Causal ordering |

### 5. Feature Flags

| Flag | Description | Security Impact |
|------|-------------|-----------------|
| `s3` | Amazon S3 backend | External storage |
| `gcs` | Google Cloud Storage backend | External storage |
| `azure` | Azure Blob Storage backend | External storage |
| `pg` | PostgreSQL metadata store | Persistent metadata |
| `redis` | Redis caching | Cache security |
| `ldap` | LDAP authentication | Auth integration |

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Function-to-SF mapping | Medium | Partial — manual traceability needed |
| Complete API spec | Medium | OpenAPI/Swagger not generated |

## Verification Instructions

```bash
# Generate API documentation
cargo doc --workspace --no-deps
# Output: target/doc/

# Verify all security functions have handlers
grep -r "auth_middleware\|cedar\|oidc\|totp" crates/server-routes/src/

# Verify API endpoint coverage
grep -r "#\[axum::" crates/server-routes/src/ | wc -l

# Run cargo doc and check for missing docs warnings
cargo doc --workspace 2>&1 | grep "missing documentation"
```

## References

- `README.md:17-38` — Feature list
- `README.md:293-323` — API endpoints table
- `docs/sdk/api_reference.md` — SDK API reference
- `docs/sdk/developer_guide.md` — Developer guide
