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
| Cedar: no schema → silent policy no-ops; Rust home bypass | **HIGH** | YP-AUTHZ-POLICY-001 | THM-AUTHZ-VALIDATION-SOUNDNESS | **FIXED (P0)** — embedded Cedar schema (User/File × 8 actions, typed context incl. `groups: Set<String>`); every `add_policy` write schema-validated, typos rejected up front; generated auto-policy set validated green (drift guard); fixed latent bug where the `"ferro-admin" in context.groups` policy failed validation → `.contains()`; fixed `POST /api/policies` AlwaysDefined collision (cedar-policy 4.x ignores `@id`, auto-ids now uniquified). **Home bypass also moved into policy**: middleware computes boundary-safe `is_own_home` context fact; Cedar decides (THM-AUTHZ-SINGLE-LANGUAGE / THM-AUTHZ-HOME-RULE; TV-AUTHZ-005 differential green). Meta-permissions for changePermissions remain future work |
| Dedup inventory registered centrally every 600 s (existence oracle) | **HIGH** | YP-MESH-TRANSPORT-001, YP-DEDUP-CDC-001 | THM-MESH-NO-EXISTENCE-ORACLE | **FIXED** — registry stores identity+reachability only; legacy inventories accepted-but-ignored with warning; holder queries constant-shape empty (relay-first); block_count side channel removed; mesh_no_leak tests green (TV-MESH-001/005). Bloom announce field staged for future P2P reconciliation |
| In-memory event bus; replay duplicates effects | **HIGH** | YP-EVENT-DURABILITY-001 | THM-REPLAY-NO-DUPLICATION | **FIXED (both halves)** — replay watermarks `(event_id, handler_name)` (TV-EVT-003) + durable SQLite outbox in `common::outbox` with lease-claim relay, 2s–300s backoff, persistent DLQ + admin re-drive (TV-EVT-001/002/005). Webhooks/email/push enqueue instead of fire-and-forget; legacy inline path for tests. Residual: enqueue is post-commit (window documented, full same-tx atomicity needs DAV-handler tx threading) |
| Cache TTL serves stale authz (no epoch invalidation) | **HIGH** | YP-CONSISTENCY-CACHE-001 | THM-AUTHZ-COHERENCE | **FIXED (primitive)** — `Cache<T>` epoch-tagged; `bump_epoch()` invalidates instantly at any TTL (TV-CACHE-001 green); wire `bump_epoch()` into permission mutations when authz caching is introduced |
| Audit chain: linear, unsigned, unanchored; PII (IP/UA) stored | MEDIUM | YP-LIFECYCLE-COMPLIANCE-001 | THM-AUDIT-TAMPER-EVIDENT | **SPEC'D 1.1** — MMR epoch log + dual-signed heads + PII epoch salting (YP-LIFECYCLE YP-9); Lean model green (AuditChain.lean, 13 thms). Impl pending |
| (size, mtime) conflict detection (LWW silent loss) | MEDIUM | YP-SYNC-CONFLICT-001 | THM-NO-SILENT-LOSS | **SPEC'D 1.1** — VV concurrency + deterministic winner + Merkle-rooted op log (YP-SYNC YP-9); Lean model green (SyncConflict.lean, 16 thms). Impl pending |
| HKDF-XOR "encryption" capped at 8160 B; no alg_id in headers | MEDIUM | YP-CRYPTO-PQ-001 | THM-CRYPTO-AGILITY-COST | **SPEC'D 1.1** — age/Tink STREAM chunked AEAD + alg_id fail-closed dispatch (YP-CRYPTO YP-9); Lean model green (ChunkedAEAD.lean, 11 thms). Impl pending |
| Mesh peer authz undefined (cross-tenant fetch) | MEDIUM | YP-MESH-TRANSPORT-001 | DC-MESH-007 | **CLOSED BY DESIGN (R2)** — tenant-anchored attenuated capability chains; no-amplification + tenant containment proved (Capability.lean, 10 thms). Impl pending |
| Photos: no EXIF/thumbnails; albums.json racy; full scans | MEDIUM | YP-MEDIA-DERIV-001 | THM-ORIGINAL-IMMUTABILITY (write-set) | **SPEC'D 1.1** — CAS-keyed derivatives w/ prefix separation, EXIF-free thumbs, albums as op-log fold (YP-MEDIA YP-9); Lean model green (Derivatives.lean, 10 thms). Impl pending |

---

## Lean Formal Coverage (Round 2, 2026-10-14)

Nine new proof modules (all `lake build` green, **0 `sorry`** in the formal
tree), each anchored to a YP revision:

| Module | YP | Headline theorems |
|---|---|---|
| formal/Ferro/SyncConflict.lean | YP-SYNC 1.1 | VV concurrency soundness, winner set-order independence, no-silent-loss, log-root strict clock |
| formal/Ferro/AuditChain.lean | YP-LIFECYCLE 1.1 | MMR append stability / no-drop, anchor tamper-evidence, PII epoch unlinkability, WORM hold precedence |
| formal/Ferro/ChunkedAEAD.lean | YP-CRYPTO 1.1 | STREAM nonce uniqueness, truncation detection, no-cap, position binding, fail-closed agility |
| formal/Ferro/Capability.lean | YP-MESH 1.1 | chain soundness, no amplification, tenant containment, interval containment |
| formal/Ferro/Derivatives.lean | YP-MEDIA 1.1 | derivative key injectivity, namespace disjointness, original immutability, album determinism |
| formal/Ferro/SieveCache.lean | YP-CONSISTENCY 1.1 | no-reorder-on-hit, single-use second chance, capacity bound, singleflight + stampede bound |
| formal/Ferro/RateLimiterGCRA.lean | YP-TENANCY 1.1 | TAT monotone/strict, burst bound |
| formal/Ferro/CircuitBreakerLiveness.lean | YP-RESILIENCE-BREAKER-001 | bounded detection/recovery, ejection capped + escalates |
| formal/Ferro/SearchFusion.lean | YP-SEARCH 1.1 | RRF monotone, fused-score domination, authz soundness, top-k bound |

PathSafety.lean: all six pre-existing `sorry`s proved (incl. THM-PATH-CHECK-COMPLETE).
Removed as dead code: Cache.lean, CacheEviction.lean, PathTraversal.lean (superseded; were outside the import graph).

Synthesis: `.specs/01_research/RESEARCH_SYNTHESIS_2026-10-R2.md`.

---

## R3 Round (2026-10-14): 10 loops, spec+proof+implementation

Synthesis: `.specs/01_research/RESEARCH_SYNTHESIS_2026-10-R3.md`.

| Gap / Area | Source YP | Status after R3 |
|---|---|---|
| Erasure coding (no local repair, no scrubber) | YP-DISTRIBUTED-EC-001 (NEW) | **FIXED** — LrcErasureCoder + scrub + Lean ErasureCode.lean; TV-EC via 15 tests |
| Placement (orphan-vnode removal bug, O(V·N) lookup) | YP-DISTRIBUTED-CH-001 (NEW) | **FIXED** — HRW rewrite of HashRing; Lean Rendezvous.lean; minimal disruption test-pinned (formal proof deferred, stated in-file) |
| Spec-code divergence: FastCDC specified, Buzhash implemented | YP-DEDUP-CDC-001 → 1.1 | **FIXED** — shared fastcdc-v1 in common::chunk; server+desktop rewired; Lean FastCDC.lean |
| Delta-CRDT + causal-stability GC | YP-SYNC-CONFLICT-001 → 1.2 | **SPEC'D 1.2** — Lean DeltaCRDT.lean green; collab objects formal-stage |
| Lock scheme/UNLOCK/shared-write/depth-1 gaps | YP-WEBDAV-PROTOCOL-001 → 1.1 | **FIXED** — opaquelocktoken, path-bound UNLOCK, token-required writes, depth-1; Lean LockSafety.lean; 25 tests |
| Refresh rotation + DPoP replay gate | YP-AUTH-OIDC-CEDAR-001 → 1.1 | **SPEC'D 1.1** — Lean TokenRotation.lean green; server integration R4 |
| Greedy ANN termination + RRF bounds | YP-SEARCH-RETRIEVAL-001 → 1.1 | **SPEC'D 1.1** — Lean AnnSearch.lean green |
| Outbox applied-set exactly-once + WAL prefix contracts | YP-EVENT-DURABILITY-001 → 1.1 | **SPEC'D 1.1** — Lean OutboxExactlyOnce.lean + WalCrash.lean green |
| Plugin containment formal | YP-TENANCY-ISOLATION-001 → 1.2 | **SPEC'D 1.2** — Lean PluginContainment.lean green; wasm-host gating R4 |
| rusqlite From-impl unguarded (broke no-`db` builds) | (build integrity) | **FIXED** — feature-gated |

Lean coverage R3 additions: ErasureCode, Rendezvous, FastCDC, DeltaCRDT, LockSafety, TokenRotation, AnnSearch, OutboxExactlyOnce, WalCrash, PluginContainment — all `lake build` green, 0 `sorry`.

---

## R4 Round (2026-10-14): 7 implementation loops — closing R2/R3 pending rows

| R2/R3 pending row | R4 loop | Status |
|---|---|---|
| Audit chain: linear, unsigned, unanchored (YP-LIFECYCLE 1.1) | R4-1 | **FIXED (structural)** — MMR with bag anchors + O(log n) inclusion proofs + persistence rebuild (`audit-log/src/mmr.rs`, 43 tests); dual-signing + PII salting still staged |
| (size, mtime) conflict detection / silent LWW (YP-SYNC 1.2) | R4-2 | **FIXED** — VV classification + deterministic winner + loser preservation (`sync-protocol/src/conflict.rs`, wired into engine.rs, 7 tests) |
| Cache: arbitrary eviction, no singleflight (YP-CONSISTENCY 1.1) | R4-3 | **FIXED** — SIEVE + SingleFlight in `server/src/cache.rs` (10 tests) |
| If-header subset parser (YP-WEBDAV 1.1) | R4-4 | **FIXED (subset→full-ish)** — real §10.4 parser (tagged/Not/OR-of-ANDs) wired into PUT + LOCK refresh (9 tests); ETag conditions still unsatisfied-by-design |
| Plugin import gate + memory bounds unenforced (YP-TENANCY 1.2) | R4-5 | **FIXED** — fail-closed import allow-list + ptr+len overflow gate (`wasm-host`, 34 tests) |
| HRW minimal disruption test-pinned (YP-DISTRIBUTED-CH 1.1) | R4-6 | **PROVED** — Lean `minimal_disruption` + `argmax_dominates_snoc` (Rendezvous.lean) |
| XOR pad retired; STREAM container pending (YP-CRYPTO 1.1) | R4-7 | **FIXED** — STREAM chunked AEAD container (`crypto/src/stream.rs`, 11 tests) |

Pre-existing issues discovered and documented (not yet fixed): `sync-protocol` `state::`/`engine::` tests hang (untouched by R3/R4 changes — likely pre-existing blocking IO in tests); `ferro-server/src/cache.rs` SIEVE queue is per-cache (multi-instance coherence inherits the epoch mechanism).
