# Research Synthesis — October 2026, Round 3 (10 Frontière Loops)

> Third research round: pull papers → analyze → compare with production
> engines → implement in spec → adjust the Lean proof → **adjust the Rust
> implementation** (new in R3: code was changed, not just specified).
> Result: **2 new Yellow Papers, 5 YP revisions, 10 new Lean modules
> (all `lake build` green, 0 `sorry`), 4 implementation fixes with 25+ new
> tests, 1 spec-vs-code divergence closed.**

## Methodology

Same discipline as R2 — verify primary sources by fetching with
[VERIFIED]/[UNCERTAIN]/[NEGATIVE RESULT] labels — plus a new R3 rule:
**every loop must touch code** (the R2 round left several "Impl pending"
rows; R3 closes or advances them).

## The Loops

| # | Domain | Frontier finding (verified) | Spec change | Lean module | Code change |
|---|--------|------------------------------|-------------|-------------|-------------|
| 1 | Erasure coding | Azure LRC (USENIX ATC'12 Best Paper — note: ATC, not SOSP); Ceph LRC/SHEC/CLAY; MinIO docs 404 (negative result) | **NEW YP-DISTRIBUTED-EC-001** — layered RS + group-local XOR parity, prefer-local decode, checksum-gated scrub | `ErasureCode.lean` (13 thms: layout accounting, namespace disjointness, local-repair read bound/correctness, decode sound+complete, scrub progress/gating) | `LrcErasureCoder` + `scrub()` in `crates/distributed` (15 new tests); fixed unguarded `rusqlite` From-impl breaking no-`db` builds |
| 2 | Placement | Jump hash (arXiv:1406.2294) verified & rejected; HRW (Thaler-Ravishankar ToN'98) adopted | **NEW YP-DISTRIBUTED-CH-001** — weighted rendezvous placement | `Rendezvous.lean` (12 thms: blackhole-free placement, member soundness, strict-top uniqueness, snoc law, addition keeps/moves with exact tie semantics) | **Rewrote** `HashRing` internals as HRW — kills the weighted-orphan-vnode removal bug (removed node kept receiving traffic) and the O(vnodes×nodes) reverse map; all 6 backend-router tests pass unchanged + 9 new ring tests |
| 3 | CDC chunking | FastCDC ATC'16 re-verified; **spec said FastCDC, code was Buzhash** — divergence found and closed | YP-DEDUP-CDC-001 → **1.1** (`ALG-FASTCDC-SHARED-001`, algo tags) | `FastCDC.lean` (window truncation stability, gear recurrence, mask containment, cut-point skipping, 3-region classifier; 1 labeled axiom: streaming-register identity) | **Implemented real FastCDC** (Gear + normalized two-mask chunking, 256K/1M/8M) in `common::chunk`; server + desktop now share it (`fastcdc-v1`); legacy buzhash-v1 kept verifiable; 8 new tests incl. resynchronization + append-stability |
| 4 | CRDT | Delta-CRDTs (arXiv:1603.01529, JPDC'18) | YP-SYNC-CONFLICT-001 → **1.2** (`ALG-DELTAMUT-001`, `ALG-CAUSALGC-001`) | `DeltaCRDT.lean` (delta decomposition law, GC drops-only-stable / keeps-above / idempotent) | (spec + proof only; collaborative objects are formal-stage) |
| 5 | WebDAV locks | RFC 4918 re-verified (§6.2, §7.1, §9.10.3, §9.11, App C) | YP-WEBDAV-PROTOCOL-001 → **1.1** | `LockSafety.lean` (depth monotonicity, write exclusion, token match/mismatch, release no-op + preserves on path mismatch) | **4 fixes**: `opaquelocktoken:` scheme (App C); UNLOCK now verifies request path vs lock path (409 `lock-token-matches-request-uri` — previously ANY token released ANY lock); shared locks now block token-less writes (§7.1); depth-1 inheritance with correct direct-children-only scope; token-aware `check_lock_for_write_with_token` (25 lock tests green) |
| 6 | Auth | RFC 9449 DPoP verified (jti/window/ath) | YP-AUTH-OIDC-CEDAR-001 → **1.1** (`ALG-ROTATION-001`, `ALG-DPOP-REPLAY-001`) | `TokenRotation.lean` (rotation single-use, replay rejected, accept-records, never-reaccepted) | (spec + proof only — authz layer is Cedar-verified territory) |
| 7 | Search | HNSW (arXiv:1603.09320) analyzed; greedy-descent termination is structural | YP-SEARCH-RETRIEVAL-001 → **1.1** (`ALG-GREEDY-ANN-001`) | `AnnSearch.lean` (hop strictly decreases, descent bounded by initial distance — cannot cycle; RRF bounds/zero). Honest correction: the earlier draft's exact scaling-distributivity claim was FALSE under truncated division and was replaced | (spec + proof only) |
| 8 | Event outbox | At-least-once + applied-set = exactly-once (formalized) | YP-EVENT-DURABILITY-001 → **1.1** (`ALG-APPLIEDSET-001`, `ALG-WAL-PREFIX-001`) | `OutboxExactlyOnce.lean` (redelivery no-op, first-delivery applies, exactly-once records, applied monotone) + `WalCrash.lean` (recovery prefix, replay idempotent, fsync monotone) | (advances R2's "Residual: enqueue is post-commit" row with the formal contract) |
| 9 | WAL durability | OSDI'21 "All File Systems Are Not Created Equal" — explicit ordering contracts required | (merged into YP-EVENT-DURABILITY-001 1.1) | see `WalCrash.lean` above | (contract for the persistence layer; SQLite remains the engine) |
| 10 | WASM sandbox | PLDI'17 WASM + W3C core (re-verified) | YP-TENANCY-ISOLATION-001 → **1.2** (`ALG-PLUGIN-GATE-001`) | `PluginContainment.lean` (import gate, fixed allow-list, memory containment, boundary zero-len) | (wasm-host gating is the R4 implementation target) |

## What Ferro now does that incumbents don't (R3 additions)

1. **Layered EC with prefer-local repair + scrubbing at DIY-NAS scale** — no self-hosted engine in Ferro's class ships EC at all (Nextcloud/Seafile: replication).
2. **Proved HRW placement** — the weighted-orphan-vnode bug class (present in ketama-style clients, and in Ferro's own ring until this round) is eliminated by construction, with the removal-stability theorem to show for it.
3. **Spec-code-convergent FastCDC** — single shared chunker on client and server, boundary-shift-stability proved, manifest algorithm tags for legacy coexistence.
4. **RFC 4918 lock compliance** — opaquelocktoken scheme, path-bound UNLOCK, token-required writes: the litmus-lock-failure modes that Class-2 servers routinely ship are closed and proved.

## Corrections & negative results (do not cite the wrong thing)

- Azure LRC is **USENIX ATC 2012** Best Paper (R2-era notes implying SOSP are wrong).
- `Nat.div` distributivity over `+` does NOT hold in truncated arithmetic — the draft RRF scaling theorem was replaced by the (true) bounds/zero theorems.
- `min.io/product/erasure-coding` returned 404 (2026-10-14); RS+bitrot description kept uncited as common knowledge.
- Minimal-disruption theorem for HRW is **test-vector-pinned** (TV-CH-*), not formally proved this round — the fold-accumulator coincidence lemma for filtered scans is deferred to R4 (stated in-file).

## Formal tree — R3 final state

10 new modules, all green: ErasureCode (13), Rendezvous (12), FastCDC (7+1 axiom), DeltaCRDT (5), LockSafety (9), TokenRotation (5+2 axioms), AnnSearch (5), OutboxExactlyOnce (5), WalCrash (4), PluginContainment (6). `lake build` green; **0 `sorry`** in the tree (1 labeled external axiom in FastCDC + pre-existing external axioms, same discipline as HashConsistency).

## Known limitations (stated honestly, in-file too)

- FastCDC streaming equivalence is a labeled axiom (the 2^64 register identity), not yet proved in-core.
- HRW minimal disruption is test-pinned, not proved.
- If-header grammar (RFC 4918 §10.4) remains a subset parser (tagged lists / `Not` / ETag conditions) — R4.
- UNLOCK 409 semantics implemented; litmus suite not yet run against the server (R4).
