# Research Synthesis — October 2026 Iteration (12 Loops)

> 12 research iterations: pull papers → analyze → compare with other engines →
> extract frontier findings → implement in spec. Result: **12 new Yellow Papers**
> (+38 theorems, +29 algorithms, +70 domain constraints, +51 test vectors),
> lifting spec coverage from 3/73 crates to the 15 highest-leverage domains.

## Methodology

Each loop: (1) read the actual Ferro code for the domain (ground truth tables in
each YP cite file:line), (2) pull primary literature (arXiv/DOI/RFC/FIPS/ISO) with
strict verification discipline — every citation tagged [VERIFIED] or [UNCERTAIN],
and *negative results recorded* (references that sound real but do not exist are
listed so downstream work never cites them), (3) comparative analysis against
Dropbox / Google Drive / Nextcloud / Syncthing / Seafile / Immich / S3, (4)
concrete recommendation with parameters sized for 50 users / one TrueNAS host,
(5) formally statable theorems for the spec.

## The Papers

| YP | Domain | Headline decision |
|----|--------|-------------------|
| YP-SYNC-CONFLICT-001 | Sync/conflict | Server-sequenced op log + version vectors + Dropbox-parity conflict copies. **No CRDT in the block layer** |
| YP-DEDUP-CDC-001 | Chunking/dedup | Buzhash → FastCDC L2 (min 256K/avg 1M/max 8M); (hash,length) manifests; BLAKE3 behind multihash prefixes; **tenant-scoped dedup** |
| YP-MESH-TRANSPORT-001 | Mesh/P2P | Bloom announcements (~24 KB) replace inventory dumps; cert-derived device IDs; relay-first NAT traversal; QUIC |
| YP-SEARCH-RETRIEVAL-001 | Search | **ACL filters in the index scan (P0)**; RRF k=60 fusion; bge-small semantic layer; SPLADE time-boxed eval |
| YP-COLLAB-WOPI-001 | Collaboration | **Implement WOPI request proof (P0)**; Cedar-bound sessions; delta sync only; awareness untrusted |
| YP-AUTHZ-POLICY-001 | Authorization | Cedar schema + validate-on-write + fail-startup; entity IDs not path strings; meta-permissions |
| YP-EVENT-DURABILITY-001 | Reliability | SQLite transactional outbox ("one box, one transaction"); handler watermarks; persistent DLQ; idempotency keys |
| YP-CONSISTENCY-CACHE-001 | Cache | Per-tenant epoch tagging (revocation visible at 0 staleness); SIEVE eviction; singleflight |
| YP-TENANCY-ISOLATION-001 | Tenancy | Fail-closed path isolation + openat2 RESOLVE_BENEATH; durable quotas; interactive-priority batch pool |
| YP-CRYPTO-PQ-001 | Crypto | X25519MLKEM768 now (HNDL); AES-KW/GCM replace XOR-pad wrap; alg_id in headers; honest FIPS-composed posture |
| YP-MEDIA-DERIV-001 | Media | EXIF at ingest; CAS-keyed derivatives; on-demand HLS (SVT-AV1) with permanent cache; no full-file video serves |
| YP-LIFECYCLE-COMPLIANCE-001 | Compliance | ZFS-hold WORM; per-user KEK crypto-shredding + deletion certificates; Merkle epoch audit heads (Ed25519+ML-DSA-65) |

## Critical Bugs Found by the Research (fix before features)

These are the "lot of problems in the spec" made concrete — each is a theorem the
current code violates:

1. **Search leaks unauthorized paths** (CRITICAL, security) — global Tantivy
   index, no result filtering (`server-api-core/src/search.rs`). Any authenticated
   user can enumerate other users' files via `/api/search`. → YP-SEARCH Phase 0.
2. **WOPI request proof missing** (CRITICAL, security) — valid token alone
   authenticates Collabora traffic; Cloudreve shipped this exact bug
   (GHSA-c3jm-gv5r-9wcp). → YP-COLLAB P0.
3. **Hash-only sync manifests are unsound** (CRITICAL, correctness) — under
   variable-length CDC, a peer cannot reconstruct a file from hashes alone.
   Files can corrupt in exactly the mesh-sync scenario Ferro exists for.
   → YP-DEDUP `THM-DEDUP-SOUNDNESS`; fix before any rechunking.
4. **Tenant isolation fails open** (HIGH, security) — `canonicalize()` failure
   falls back to the raw path (`multi-tenant/src/isolation.rs:105-108`), plus the
   org/tenant layout mismatch (`:102` vs `:145`). → YP-TENANCY `THM-PREFIX-CONTAINMENT`.
5. **Cedar silently ignores broken policies** (HIGH) — no schema, no validation;
   a typo'd attribute drops a policy from every decision with no log. Plus the
   Rust-side home bypass splits authorization across two languages.
   → YP-AUTHZ P0.
6. **The mesh actively leaks an existence oracle** (HIGH) — `register()` posts the
   full block inventory to the registry every 600 s; `query_block_holders` answers
   per-hash booleans. Dropbox publicly concedes the passive version of this leak;
   Ferro's is active and by design. → YP-MESH/DEDUP non-interference theorems.
7. **Event bus loses and duplicates effects** (HIGH) — fire-and-forget dispatch
   after commit (dual-write); `replay()` re-runs all handlers with no watermark.
   → YP-EVENT outbox.
8. **Cache TTL can serve revoked permissions** (HIGH) — no invalidation hook on
   authz caches. → YP-CONSISTENCY epoch tagging.
9. **Sync conflict = mtime LWW** (MEDIUM) — silent data loss when clocks agree.
   → YP-SYNC version vectors.
10. **"Encryption" is an XOR pad capped at 8160 bytes** (MEDIUM) — not an AEAD,
    not FIPS-approvable, can't handle real chunks; headers lack alg_id.
    → YP-CRYPTO.

## Engine-Parity Notes (corrections to common beliefs)

- Google Drive does **not** expose a revision DAG — public API is a linear
  revision list (`headRevisionId` only for blob files, 200-rev cap); the
  merge behavior people cite is Google *Docs* OT, not Drive's file layer.
- Dropbox uses **fixed 4 MB blocks** (not CDC) and its own docs concede the
  LAN-sync hash-request leak; "Dropbox PQ deployment" has no public source.
- Syncthing resolves conflicts via version vectors with a deterministic
  device-ID tie-break — the closest production model to YP-SYNC-CONFLICT-001.
- Nextcloud's desktop client does **not upload conflict files by default** and
  its server locks are advisory only.
- "GearCDC (2023)", "Consistency in Zanzibar (OSDI'21)", the Google Wave quota
  paper (ATC'14), and "Diamond Types and semilattices" **do not exist** as cited;
  verified absences are recorded in the individual YP bibliographies.
- SIEVE (NSDI'24) is by Zhang/Yang/Yue/Vigfusson/Rashmi — not the attribution
  circulating in some blog posts.

## Frontier (2024–2026) findings adopted

- **NAT traversal measured at scale**: 70% ± 7.1% conditional hole-punch success
  (4.4M attempts); TCP ≈ QUIC — informs relay-first mesh design (arXiv:2510.27500).
- **CDC with theorems**: Chonkers (arXiv:2509.11121) — watched, not depended on;
  WideCDC/VectorCDC/SeqCDC line confirms the ~3× vectorization ceiling.
- **Automerge 3.0** kills the "CRDTs are unbounded" objection for *documents* —
  which strengthens, not weakens, the decision to keep CRDTs out of Ferro's
  binary-file layer.
- **Cedar reached OOPSLA 2024 with a Lean proof** of validation soundness —
  Ferro just needs to invoke it.
- **Dual-write impossibility machine-checked** (arXiv:2608.00501) — the formal
  basis for the outbox decision.
- **PQ transport crossed 50%** at Cloudflare (2025); OpenSSL 3.5 defaults
  X25519MLKEM768; NIST IR 8547 sets the 2030/2035 clocks.
- **SIEVE → SR-Gadgets** lineage (NSDI'24 → 2026) gives knob-free, scan-resistant
  O(1) eviction.

## Cross-Cutting Synthesis (one-of-each principle)

1. **One algorithm registry** (`crypto::traits`): `KemId/SigId/AeadId/KdfId` +
   `is_fips_approved()` + `is_pq()` — consumed by FIPS gate, mesh handshake,
   audit signing, storage AEAD.
2. **One ciphertext header**: `{alg_id, key_id, key_version, nonce}` — chunks,
   versions, derivatives, key wrap.
3. **One log substrate**: RFC 9162-style Merkle epoch heads, hybrid-signed —
   compliance tamper-evidence now; federation attestation and backup
   verification later.
4. **One precedence engine**: `hold > WORM > retention > user-delete`, every
   branch audited with legal basis.
5. **One coherence mechanism**: per-tenant epoch, shared by cache, search, and
   FUSE attr caches.

## What was deliberately NOT adopted (and why)

- **libp2p** — 50k-line stack for features (DHT/gossipsub/autonat) a 50-user
  LAN+relay mesh does not need; Syncthing's cert-derived identity model is
  smaller and auditable.
- **Temporal/DBOS/NATS** — durable execution in the SQLite outbox is provably
  sufficient at this scale and removes a distributed dependency.
- **Noise protocol** — QUIC already provides TLS 1.3; a second handshake stack
  doubles audit surface for zero gain.
- **Coded dedup** — destroys single-block on-demand serving required by the mesh.
- **Full CRDT file layer** — binary formats can't merge; metadata op-log +
  conflict copies achieves convergence where it's possible and preservation
  everywhere else.
- **QUIC multipath** — RFC-Editor-queue draft with documented CC-fairness
  problems; abstracted behind `MeshTransport` for later.
- **RACORE/dedup side-channel countermeasures** — correct at cloud scale,
  overkill at 50 users; tenant-scoped dedup is the honest fix.

## Coverage after this iteration

| | Before | After |
|--|--------|-------|
| Yellow papers | 3 | 15 |
| Theorems | 8 | 46 |
| Algorithms | 8 | 37 |
| Domain constraints (files) | 3 | 7 |
| Test vector files | 3 | 7 |
| Crates with a covering paper (major subsystems) | 3 domains | 15 domains (all P0 surfaces) |

Remaining spec gaps (future iterations): WebDAV extensions/NC-specific props,
ActivityPub federation, CalDAV/CardDAV correctness, GraphQL surface, offline
desktop model, FUSE/mount semantics, SCIM provisioning, event-sourced client SDK.
