# ADV_TDS: Technical Design

## Assurance Family Requirement

The developer shall provide a technical design describing the building blocks of the TOE and their relationships, including all security-relevant internal interfaces.

**EAL Level:** EAL 3+ (ADV_TDS.4)

## Evidence Artifacts

### 1. ServerState Trait (Central State Interface)

**File:** `crates/server-state/src/lib.rs:42-165`

The `ServerState` trait defines **72 accessor methods** providing the central interface to all server subsystems. This is the primary integration point between route handlers and backend services.

| Method Category | Count | Methods |
|-----------------|-------|---------|
| Storage | 3 | `storage()`, `cas_store()`, `metadata_store()` |
| Authentication | 5 | `oidc()`, `user_store()`, `api_key_store()`, `admin_user()`, `admin_password()` |
| Authorization | 1 | `cedar()` |
| Compliance | 4 | `worm_store()`, `retention_store()`, `dlp_store()`, `fips_validator()` |
| Collaboration | 4 | `tags()`, `comments()`, `calendar_store()`, `address_book_store()` |
| Sharing | 3 | `share_store()`, `favorites()`, `selective_sync_store()` |
| Infrastructure | 6 | `event_bus()`, `read_cache()`, `health_checker()`, `ws_manager()`, `plugin_registry()`, `webhooks()` |
| Resilience | 5 | `storage_circuit_breaker()`, `auth_circuit_breaker()`, `ldap_circuit_breaker()`, `bulkhead_pools()`, `retry_policy()` |
| Monitoring | 6 | `request_count()`, `storage_op_counts()`, `request_duration_buckets()`, `request_status_counts()`, `slo_collector()`, `slo_definitions()` |
| WASM | 3 | `wasm_runtime()`, `wasm_dispatch_count()`, `wasm_error_count()`, `wasm_fuel_total()` |
| Federation | 2 | `federation_secret()`, `activity_store()` |
| Configuration | 10 | `external_url()`, `max_body_size()`, `thumbnail_size()`, `data_dir()`, `max_file_versions()`, `quota_bytes()`, `used_bytes()`, `file_count()`, `auth_enabled()`, `wopi_office_url()` |
| Search | 2 | `search()`, `search_ranking_config()` |
| Push Notifications | 2 | `push_notification_store()`, `push_notification_config()` |
| Rate Limiting | 2 | `tenant_rate_limit_store()`, `tenant_rate_limiter()` |

Additional traits in `crates/server-state/src/traits.rs`:

| Trait | Methods | Purpose |
|-------|---------|---------|
| `SyncStoreTrait` | 6 | Sync operation tracking |
| `IdempotencyStoreTrait` | 2 | Idempotent request handling |
| `NotificationPrefsStoreTrait` | 3 | Notification preferences |
| `RansomwareDetectorTrait` | 1 | Ransomware detection |
| `UploadStoreTrait` | 8 | Chunked upload management |
| `AuditLogTrait` | 5 | Audit log operations |

### 2. Crate Dependency Graph (6-Layer Architecture)

**File:** `.specs/02_architecture/crate_dependency_graph.md`

```
Layer 0 (Foundation):     common
Layer 1 (Core):           core, auth, crypto, circuit-breaker, crdt, rate-limiter, event-bus, cache, health
Layer 2 (Domain):         dav, caldav, webdav-handler, sync-protocol, offline, distributed, graphql
Layer 3 (Infrastructure): server-security, server-security-middleware, server-compliance, server-content,
                          server-storage-ops, server-sharing, server-collaboration, server-admin-api,
                          server-user-mgmt, server-api-core, server-automation, server-webdav-core,
                          server-state, server-routes, server-infra (20+ crates)
Layer 4 (Application):    server
Layer 5 (Clients):        web, cli, client, desktop, mobile, admin
```

### 3. All Crate Cargo.toml Files

**76 Cargo.toml files** across the workspace (see `Cargo.toml` workspace manifest):

Key crates with security relevance:

| Crate | Path | Dependencies |
|-------|------|-------------|
| `ferro-common` | `crates/common/Cargo.toml` | Zero internal deps |
| `ferro-core` | `crates/core/Cargo.toml` | `common` |
| `ferro-auth` | `crates/auth/Cargo.toml` | `common`, `crypto` |
| `ferro-crypto` | `crates/crypto/Cargo.toml` | Zero internal deps |
| `ferro-server` | `crates/server/Cargo.toml` | 40+ crates |
| `ferro-server-security` | `crates/server-security/Cargo.toml` | `common`, `auth` |
| `ferro-server-security-middleware` | `crates/server-security-middleware/Cargo.toml` | `auth`, `server-security` |
| `ferro-server-compliance` | `crates/server-compliance/Cargo.toml` | `common`, `circuit-breaker`, `server-security` |

### 4. Interface Contracts

**Path:** `.specs/02_architecture/interface_contracts/` — **NOT YET CREATED**

Key interface contracts are defined through Rust traits:

| Interface | Crate | Trait | Methods |
|-----------|-------|-------|---------|
| Storage | `common` | `StorageEngine` | Upload, download, list, delete, etc. |
| Lock management | `common` | `LockManagerTrait` | Lock, unlock, check |
| Metadata | `core` | `MetadataStore` | CRUD for metadata |
| CAS | `core` | `CasStore` | Content-addressable storage |
| User store | `auth` | `UserStoreTrait` | User CRUD |
| API keys | `auth` | `ApiKeyStoreTrait` | API key management |
| Cedar | `auth` | `CedarAuthorizer` | Policy evaluation |
| Shares | `server-sharing` | `ShareStoreTrait` | Share management |
| Calendar | `dav` | `CalendarStore` | CalDAV storage |
| Address book | `dav` | `AddressBookStore` | CardDAV storage |

### 5. Cryptographic Design

| Algorithm | Usage | Crate | Standard |
|-----------|-------|-------|----------|
| SHA-256 | Content hashing, audit chain | `crypto` | NIST SP 800-107 |
| AES-GCM-256 | Encryption at rest | `server-security` | NIST SP 800-38D |
| ECDSA P-256 | Digital signatures | `crypto` | FIPS 186-4 |
| TLS 1.3 | Encryption in transit | Axum/hyper | RFC 8446 |
| Argon2id | Password hashing | `auth` | IETF RFC 9106 |
| HMAC-SHA256 | HTTP signatures, WOPI tokens | `crypto` | NIST SP 800-107 |

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Detailed design documents | Medium | Blue Papers needed per-crate |
| Interface contracts directory | Medium | `.specs/02_architecture/interface_contracts/` not yet created |
| Sequence diagrams | Medium | Critical flows need visualization |

## Verification Instructions

```bash
# Verify trait completeness
cargo doc -p ferro-server-state --no-deps 2>&1 | grep "trait ServerState"

# Verify dependency graph matches
cargo tree --workspace --depth 1

# Verify no circular dependencies
cargo metadata --format-version 1 | jq '.resolve.nodes[] | select(.deps | length > 0) | .id'

# Verify all crates compile
cargo check --workspace --all-features
```

## References

- `.specs/02_architecture/crate_dependency_graph.md` — Full Mermaid graph
- `crates/server-state/src/lib.rs` — ServerState trait definition
- `crates/server-state/src/traits.rs` — Supporting trait definitions
- `docs/compliance/nist_sp80053_mapping.md` — Control-by-control mapping
