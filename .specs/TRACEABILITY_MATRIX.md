# Cross-Paper Traceability Matrix

## Yellow Paper to Yellow Paper Dependencies

| Source YP | Target YP | Dependency Type | Elements |
|-----------|-----------|-----------------|----------|
| YP-STORAGE-CAS-001 | YP-AUTH-OIDC-CEDAR-001 | provides-verification | ALG-PRESIGN-001 requires Cedar eval |
| YP-WEBDAV-PROTOCOL-001 | YP-STORAGE-CAS-001 | uses-storage | PROPFIND/GET use CAS get |
| YP-WEBDAV-PROTOCOL-001 | YP-AUTH-OIDC-CEDAR-001 | uses-auth | LOCK requires Cedar authorization |
| YP-DEDUP-CDC-001 | YP-STORAGE-CAS-001 | extends | FastCDC chunking + multihash BlockIDs supersede fixed-hash assumptions |
| YP-SYNC-CONFLICT-001 | YP-DEDUP-CDC-001 | uses-manifests | op log content_rev = manifest Merkle root |
| YP-MESH-TRANSPORT-001 | YP-DEDUP-CDC-001 | uses-hashes | Bloom announcements over BlockIDs; per-block verification |
| YP-SEARCH-RETRIEVAL-001 | YP-AUTHZ-POLICY-001 | uses-authz | Cedar re-verification of top-k; ACL discriminators |
| YP-COLLAB-WOPI-001 | YP-AUTHZ-POLICY-001 | uses-authz | session-bound Cedar decisions; WOPI scope |
| YP-CONSISTENCY-CACHE-001 | YP-AUTHZ-POLICY-001 | maintains-coherence | epoch-tagged authz cache (THM-AUTHZ-COHERENCE) |
| YP-CONSISTENCY-CACHE-001 | YP-SEARCH-RETRIEVAL-001 | caches | authz-filtered search results |
| YP-EVENT-DURABILITY-001 | YP-LIFECYCLE-COMPLIANCE-001 | couples-seq | outbox.seq shares tx with audit chain position |
| YP-TENANCY-ISOLATION-001 | YP-AUTHZ-POLICY-001 | soundness-pair | TenantGuard + validated Cedar (THM-TENANT-POLICY-SOUNDNESS) |
| YP-TENANCY-ISOLATION-001 | YP-EVENT-DURABILITY-001 | uses-outbox | durable quota deltas in upload transactions |
| YP-TENANCY-ISOLATION-001 | YP-MEDIA-DERIV-001 | schedules | media jobs run in bounded batch pool |
| YP-CRYPTO-PQ-001 | YP-LIFECYCLE-COMPLIANCE-001 | provides-signatures | ML-DSA-65 hybrid Ed25519 audit heads; GCM chunk keys for shred |
| YP-MEDIA-DERIV-001 | YP-DEDUP-CDC-001 | uses-cas | derivatives keyed H(content, preset, encoder_ver) in CAS |
| YP-LIFECYCLE-COMPLIANCE-001 | YP-CRYPTO-PQ-001 | provides-crypto | AES-256-GCM chunk AEAD enables THM-CRYPTO-SHRED |

## Yellow Paper to Blue Paper Mapping

| Yellow Paper | Blue Paper | Elements Used | Verification |
|--------------|------------|---------------|---------------|
| YP-WEBDAV-PROTOCOL-001 | BP-WEBDAV-HANDLER-001 | ALG-PROPFIND-001, ALG-COPY-001 | Unit + Integration |
| YP-WEBDAV-PROTOCOL-001 | BP-LOCK-MANAGER-001 | ALG-LOCK-001 | Unit + Property |
| YP-STORAGE-CAS-001 | BP-STORAGE-ENGINE-001 | ALG-CAS-PUT-001, ALG-CAS-GET-001 | Unit + Integration |
| YP-STORAGE-CAS-001 | BP-PRESIGNED-URL-001 | ALG-PRESIGN-001 | Unit + Security |
| YP-AUTH-OIDC-CEDAR-001 | BP-AUTH-MIDDLEWARE-001 | ALG-OIDC-VALIDATE-001 | Unit + Security |
| YP-AUTH-OIDC-CEDAR-001 | BP-CEDAR-ENGINE-001 | ALG-CEDAR-EVAL-001 | Unit + Property |
| YP-SYNC-CONFLICT-001 | BP-SYNC-ENGINE-001 | ALG-OPLOG-001, ALG-LAMPORT-001 | Unit + Property (TV-SYNC-001..006) |
| YP-DEDUP-CDC-001 | BP-STORAGE-ENGINE-001 | ALG-FastCDC-001, ALG-MANIFEST-001 | Unit + Property (TV-CDC-001..002, TV-DEDUP-001..004) |
| YP-DEDUP-CDC-001 | BP-CAS-DEDUP-001 | ALG-DEDUPSCOPE-001 | Unit + Security (TV-DEDUP-005) |
| YP-MESH-TRANSPORT-001 | BP-MESH-TRANSPORT-001 | ALG-ANNOUNCE/DISCOVER/TRANSPORT/NAT-001 | Integration + Security (TV-MESH-001..006) |
| YP-SEARCH-RETRIEVAL-001 | BP-SEARCH-SERVICE-001 | ALG-ACLQUERY-001, ALG-RRF-001 | Unit + Security (TV-SEARCH-001..007) |
| YP-COLLAB-WOPI-001 | BP-WOPI-HOST-001 | ALG-WOPIPROOF-001, ALG-DELTASYNC-001 | Unit + Security (TV-WOPI-001..006, TV-COLLAB-001..004) |
| YP-AUTHZ-POLICY-001 | BP-AUTH-MIDDLEWARE-001 | ALG-SCHEMAVAL-001 | Unit + Security (TV-AUTHZ-001..006) |
| YP-AUTHZ-POLICY-001 | BP-CEDAR-ENGINE-001 | ALG-ENTITIES-001, ALG-METAPERM-001 | Unit + Property (TV-AUTHZ-003..005) |
| YP-EVENT-DURABILITY-001 | BP-EVENT-BUS-001 | ALG-OUTBOX/RELAY/IDEMPOTENCY-001 | Integration + Chaos (TV-EVT-001..007) |
| YP-CONSISTENCY-CACHE-001 | BP-CACHE-LAYER-001 | ALG-EPOCH/SIEVE/SINGLEFLIGHT-001 | Unit + Property (TV-CACHE-001..005) |
| YP-CONSISTENCY-CACHE-001 | BP-SEARCH-SERVICE-001 | ALG-EPOCH-001 (authz coherence) | Integration (TV-CACHE-001) |
| YP-TENANCY-ISOLATION-001 | BP-TENANT-ISOLATION-001 | ALG-RESOLVE/QUOTA/BATCHPOOL-001 | Unit + Security (TV-TENANT-001..005) |
| YP-CRYPTO-PQ-001 | BP-CRYPTO-PROVIDER-001 | ALG-TLS-001, ALG-HEADER-001 | Unit + Security (TV-CRYPTO-001..007) |
| YP-CRYPTO-PQ-001 | BP-FIPS-MODE-001 | THM-FIPS-APPROVED-ONLY gate | Unit (TV-CRYPTO-001..002) |
| YP-MEDIA-DERIV-001 | BP-MEDIA-PIPELINE-001 | ALG-INGEST/THUMB/VOD-001 | Integration (TV-MEDIA-001..005) |
| YP-LIFECYCLE-COMPLIANCE-001 | BP-COMPLIANCE-ENGINE-001 | ALG-WORMHOLD-001, ALG-ERASE-001 | Integration + Security (TV-COMP-001..002, TV-COMP-006..007) |
| YP-LIFECYCLE-COMPLIANCE-001 | BP-AUDIT-LOG-001 | ALG-EPOCHLOG-001 | Unit + Security (TV-COMP-003..005) |

## Blue Paper to Blue Paper Dependencies

| Blue Paper | Depends On | Interface |
|------------|------------|-----------|
| BP-WEBDAV-HANDLER-001 | BP-STORAGE-ENGINE-001 | IF-STORAGE-001 |
| BP-WEBDAV-HANDLER-001 | BP-LOCK-MANAGER-001 | IF-LOCK-001 |
| BP-WEBDAV-HANDLER-001 | BP-AUTH-MIDDLEWARE-001 | IF-AUTH-001 |
| BP-STORAGE-ENGINE-001 | BP-CAS-DEDUP-001 | IF-CAS-001 |
| BP-PRESIGNED-URL-001 | BP-CEDAR-ENGINE-001 | IF-CEDAR-001 |
| BP-SYNC-ENGINE-001 | BP-CAS-DEDUP-001 | IF-CAS-001 (manifests) |
| BP-MESH-TRANSPORT-001 | BP-CAS-DEDUP-001 | IF-CAS-001 (block verify) |
| BP-MESH-TRANSPORT-001 | BP-TENANT-ISOLATION-001 | IF-TENANT-001 (mesh authz, DC-MESH-007) |
| BP-SEARCH-SERVICE-001 | BP-CEDAR-ENGINE-001 | IF-CEDAR-001 |
| BP-SEARCH-SERVICE-001 | BP-CACHE-LAYER-001 | IF-CACHE-001 |
| BP-WOPI-HOST-001 | BP-CEDAR-ENGINE-001 | IF-CEDAR-001 |
| BP-EVENT-BUS-001 | BP-AUDIT-LOG-001 | IF-AUDIT-001 (seq coupling) |
| BP-TENANT-ISOLATION-001 | BP-CEDAR-ENGINE-001 | IF-CEDAR-001 |
| BP-MEDIA-PIPELINE-001 | BP-CAS-DEDUP-001 | IF-CAS-001 (deriv namespace) |
| BP-MEDIA-PIPELINE-001 | BP-TENANT-ISOLATION-001 | IF-TENANT-001 (batch pool) |
| BP-COMPLIANCE-ENGINE-001 | BP-CRYPTO-PROVIDER-001 | IF-CRYPTO-001 |
| BP-AUDIT-LOG-001 | BP-CRYPTO-PROVIDER-001 | IF-CRYPTO-001 (hybrid sigs) |
| BP-FIPS-MODE-001 | BP-CRYPTO-PROVIDER-001 | IF-CRYPTO-001 |

## Requirement Coverage by Papers

| Requirement | Yellow Paper(s) | Blue Paper(s) | Test Coverage |
|-------------|-----------------|---------------|---------------|
| REQ-STOR-001 | YP-STORAGE-CAS-001 | BP-STORAGE-ENGINE-001 | TV-CAS-001..015 |
| REQ-STOR-002 | YP-STORAGE-CAS-001, YP-DEDUP-CDC-001 | BP-CAS-DEDUP-001 | TV-CAS-001..015, TV-DEDUP-001..005 |
| REQ-STOR-004 | YP-STORAGE-CAS-001 | BP-PRESIGNED-URL-001 | TV-CAS-016..020 |
| REQ-STOR-005 | YP-STORAGE-CAS-001, YP-DEDUP-CDC-001 | BP-STORAGE-ENGINE-001 | TV-CAS-006..008, TV-CDC-001..002 |
| REQ-STOR-006 | YP-STORAGE-CAS-001, YP-DEDUP-CDC-001 | BP-STORAGE-ENGINE-001 | TV-CAS-009..010, TV-DEDUP-001..004 |
| REQ-WEBDAV-001 | YP-WEBDAV-PROTOCOL-001 | BP-WEBDAV-HANDLER-001 | TV-PROP-001..003 |
| REQ-WEBDAV-002 | YP-WEBDAV-PROTOCOL-001 | BP-LOCK-MANAGER-001 | TV-LOCK-001..005 |
| REQ-WEBDAV-004 | YP-WEBDAV-PROTOCOL-001 | BP-WEBDAV-HANDLER-001 | TV-RCLONE-001..003 |
| REQ-WEBDAV-006 | YP-WEBDAV-PROTOCOL-001 | BP-WEBDAV-HANDLER-001 | TV-PROP-001..006 |
| REQ-AUTH-001 | YP-AUTH-OIDC-CEDAR-001 | BP-AUTH-MIDDLEWARE-001 | TV-OIDC-001..008 |
| REQ-AUTH-002 | YP-AUTH-OIDC-CEDAR-001, YP-AUTHZ-POLICY-001 | BP-CEDAR-ENGINE-001 | TV-CEDAR-001..012, TV-AUTHZ-001..006 |
| REQ-AUTH-003 | YP-AUTH-OIDC-CEDAR-001, YP-AUTHZ-POLICY-001 | BP-CEDAR-ENGINE-001 | TV-CEDAR-001..012, TV-AUTHZ-001..006 |
| REQ-SYNC-001 (conflict safety) | YP-SYNC-CONFLICT-001 | BP-SYNC-ENGINE-001 | TV-SYNC-001..006 |
| REQ-SYNC-002 (mesh) | YP-MESH-TRANSPORT-001 | BP-MESH-TRANSPORT-001 | TV-MESH-001..006 |
| REQ-SEARCH-001 (authz-sound search) | YP-SEARCH-RETRIEVAL-001 | BP-SEARCH-SERVICE-001 | TV-SEARCH-001..007 |
| REQ-COLLAB-001 (WOPI security) | YP-COLLAB-WOPI-001 | BP-WOPI-HOST-001 | TV-WOPI-001..006 |
| REQ-REL-001 (durable effects) | YP-EVENT-DURABILITY-001 | BP-EVENT-BUS-001 | TV-EVT-001..007 |
| REQ-REL-002 (coherent cache) | YP-CONSISTENCY-CACHE-001 | BP-CACHE-LAYER-001 | TV-CACHE-001..005 |
| REQ-TEN-001 (isolation/quotas) | YP-TENANCY-ISOLATION-001 | BP-TENANT-ISOLATION-001 | TV-TENANT-001..005 |
| REQ-SEC-001 (PQ/FIPS) | YP-CRYPTO-PQ-001 | BP-CRYPTO-PROVIDER-001, BP-FIPS-MODE-001 | TV-CRYPTO-001..007 |
| REQ-MEDIA-001 (derivatives) | YP-MEDIA-DERIV-001 | BP-MEDIA-PIPELINE-001 | TV-MEDIA-001..005 |
| REQ-COMP-001 (WORM/hold/erase/audit) | YP-LIFECYCLE-COMPLIANCE-001 | BP-COMPLIANCE-ENGINE-001, BP-AUDIT-LOG-001 | TV-COMP-001..007 |

## Known Implementation Gaps (spec precedes code; P0 items)

Status legend: **FIXED** = code implements the theorem + regression tests green (2026-10-11); OPEN = still to implement.

| Gap | Severity | Source YP | Theorem currently violated | Status |
|-----|----------|-----------|---------------------------|--------|
| Search path has no ACL filtering (global index) | **CRITICAL** | YP-SEARCH-RETRIEVAL-001 | THM-SEARCH-AUTHZ | **FIXED** — `search_scoped` in-scan MUST-clause; principal-typed handlers; admin-group bypass; TV-SEARCH-001 green |
| WOPI request proof not implemented | **CRITICAL** | YP-COLLAB-WOPI-001 | THM-WOPI-NO-FALLTHROUGH | **FIXED** — proof verify (current+old HKDF keys, ±20 min) when present; token↔path binding; view/edit scope; constant-time MAC; TV-WOPI-001..006 green |
| Hash-only sync manifests (no lengths) | **CRITICAL (P0 correctness)** | YP-DEDUP-CDC-001 | THM-DEDUP-SOUNDNESS | **FIXED** — `diff_manifests` returns descriptors (index/hash/size/offset, multiplicity preserved); reconstruction + boundary-invariant tests green |
| Chunking parameter drift (4 duplicated literal sites) | HIGH | YP-DEDUP-CDC-001 | THM-CHUNKING-DETERMINISM | **FIXED** — `common::chunk::{CHUNK_TARGET,MIN,MAX}_SIZE` shared constants wired into both sides |
| Tenant path isolation fails open on canonicalize error | **HIGH (security bug)** | YP-TENANCY-ISOLATION-001 | THM-PREFIX-CONTAINMENT | **FIXED** — fail-closed with lexical-normalization fallback for pre-create checks; prefix-boundary fix (t1 vs t10 collision); symlink-escape + traversal + layout-unification tests green (TV-TENANT-001a/b/c) |
| Cedar: no schema → silent policy no-ops; Rust home bypass | **HIGH** | YP-AUTHZ-POLICY-001 | THM-AUTHZ-VALIDATION-SOUNDNESS | **FIXED (P0)** — embedded Cedar schema (User/File × 8 actions, typed context incl. `groups: Set<String>`); every `add_policy` write schema-validated, typos rejected up front; generated auto-policy set validated green (drift guard); fixed latent bug where the `"ferro-admin" in context.groups` policy failed validation → `.contains()`; fixed `POST /api/policies` AlwaysDefined collision (cedar-policy 4.x ignores `@id`, auto-ids now uniquified). Rust home bypass → OPEN (meta-permissions phase) |
| Dedup inventory registered centrally every 600 s (existence oracle) | **HIGH** | YP-MESH-TRANSPORT-001, YP-DEDUP-CDC-001 | THM-MESH-NO-EXISTENCE-ORACLE | **FIXED** — registry stores identity+reachability only; legacy inventories accepted-but-ignored with warning; holder queries constant-shape empty (relay-first); block_count side channel removed; mesh_no_leak tests green (TV-MESH-001/005). Bloom announce field staged for future P2P reconciliation |
| In-memory event bus; replay duplicates effects | **HIGH** | YP-EVENT-DURABILITY-001 | THM-REPLAY-NO-DUPLICATION | **FIXED (both halves)** — replay watermarks `(event_id, handler_name)` (TV-EVT-003) + durable SQLite outbox in `common::outbox` with lease-claim relay, 2s–300s backoff, persistent DLQ + admin re-drive (TV-EVT-001/002/005). Webhooks/email/push enqueue instead of fire-and-forget; legacy inline path for tests. Residual: enqueue is post-commit (window documented, full same-tx atomicity needs DAV-handler tx threading) |
| Cache TTL serves stale authz (no epoch invalidation) | **HIGH** | YP-CONSISTENCY-CACHE-001 | THM-AUTHZ-COHERENCE | **FIXED (primitive)** — `Cache<T>` epoch-tagged; `bump_epoch()` invalidates instantly at any TTL (TV-CACHE-001 green); wire `bump_epoch()` into permission mutations when authz caching is introduced |
| Audit chain: linear, unsigned, unanchored; PII (IP/UA) stored | MEDIUM | YP-LIFECYCLE-COMPLIANCE-001 | THM-AUDIT-TAMPER-EVIDENT | OPEN |
| (size, mtime) conflict detection (LWW silent loss) | MEDIUM | YP-SYNC-CONFLICT-001 | THM-NO-SILENT-LOSS | OPEN |
| HKDF-XOR "encryption" capped at 8160 B; no alg_id in headers | MEDIUM | YP-CRYPTO-PQ-001 | THM-CRYPTO-AGILITY-COST | OPEN |
| Mesh peer authz undefined (cross-tenant fetch) | MEDIUM | YP-MESH-TRANSPORT-001 | DC-MESH-007 | OPEN |
| Photos: no EXIF/thumbnails; albums.json racy; full scans | MEDIUM | YP-MEDIA-DERIV-001 | THM-ORIGINAL-IMMUTABILITY (write-set) | OPEN |
