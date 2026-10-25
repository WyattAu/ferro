# YP-CONSISTENCY-CACHE-001: Cache Coherence, Read-Your-Writes & Eviction Theory

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-CONSISTENCY-CACHE-001                   |
| Domain       | Distributed Systems / Caching              |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/cache/src/{cache,lru,stats}.rs`, `crates/fuse/src/cache.rs` |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| `TimedCache` (DashMap) serves **TTL-stale authorization and metadata**; no invalidation hook, no epoch/generation anywhere | `crates/cache/src/cache.rs` |
| LRU is O(n) `Vec` under two mutexes (linear reindex on every access/remove) | `crates/cache/src/lru.rs:19-49` |
| `set()` **silently drops** at capacity while `set_with_size()` returns errors — inconsistent capacity semantics | `cache.rs:110-134, 188-200` |
| No singleflight, no stale-while-revalidate, no epoch invalidation | repo-wide |
| FUSE cache has the same shape | `crates/fuse/src/cache.rs` |
| Single SQLite = strong raw consistency; the cache layer weakens it to TTL-epsilon | derived |

**Core risk:** a permission revocation can ride a cache entry up to TTL seconds. The planned authz-filtered search (YP-SEARCH-RETRIEVAL-001) makes **authz-cache coherence** the critical invariant: a cached hit must never serve rows a policy now denies.

---

## YP-2: Executive Summary

### Problem Statement

Ferro needs caches that are *correct before fast*: a revocation must be visible to all readers immediately after commit, independent of TTL; eviction must be O(1), scan-resistant, and evict-not-error; concurrent misses must coalesce.

### Decisions

1. **Coherence = monotonic per-tenant metadata epoch** (`u64` in SQLite, bumped in the same transaction as any permission/metadata write). Cache entries store `(tenant_epoch, value)`; reads miss if stale. Result: revocation visible **immediately after commit** (same process ⇒ same epoch read). TTL (60 s + SWR 30 s per RFC 5861) remains purely a latency optimization, never the coherence mechanism. Negative caching for authz denies: TTL 5–15 s (blunts token-bucket probing).
2. **Eviction = SIEVE** (NSDI '24): one FIFO list + one lazy-promotion hand bit; O(1); no tuning knobs; ~40 LoC. `record_access` becomes `bit = 1`.
3. **Capacity semantics unified on evict-not-error:** `set_with_size` evicts SIEVE victims until the entry fits; error only if the single entry exceeds the budget. Silent drop deleted.
4. **Stampede control:** per-key singleflight + TTL jitter ±10% + `stale-if-error` fallback (RFC 9111 §4 request-collapsing is the protocol analog).
5. **HTTP surface:** ETags on metadata responses; conditional GETs honored (RFC 9111 §4.3) — clients get RYW via `If-None-Match` after writes.
6. **FUSE cache:** same epoch tagging; attr TTL 1 s with write-through invalidation.
7. **Multi-host future (pg feature):** session tokens (`last_write_version` per session; replicas filter by version ≥ token) — real-time causal consistency, the strongest availability-compatible guarantee (Mahajan/Alvisi/Dahlin ceiling). Do NOT advertise linearizability not possessed (Kleppmann 2015).

### Guarantees

- RYW with staleness bound **0** for metadata/authz (single-process epoch-tagged).
- No object readable under policy P is served after P revokes it, at any time.
- Bounded staleness under SWR (age ≤ T+S; ≤ 1 background revalidation per entry per window).
- Recompute concurrency per key ≤ 1 (singleflight).

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| Metadata epoch | Per-tenant monotone counter bumped in every permission/metadata transaction. |
| SIEVE | Eviction algorithm: FIFO + one hand bit, lazy promotion; O(1), scan-resistant, knob-free. |
| Singleflight | Per-key in-flight dedup: N concurrent misses → 1 recompute, followers share the result. |
| SWR / SIE (RFC 5861) | `stale-while-revalidate` / `stale-if-error` cache-control extensions. |
| RYW | Read-your-writes: a read after a commit observes the commit's effects. |
| RT-causal | Real-time causal consistency; strongest guarantee compatible with one-copy availability in async systems. |
| Scan resistance | Immunity of hit ratio to one-pass sequential scans polluting the cache. |

---

## YP-4: Theoretical Foundation

### THM-RYW-VISIBILITY

With single-process serialized SQLite and epoch-tagged caches, any read `R` issued after commit of write `W` observes `W`'s effects: RYW holds with staleness bound **0** for metadata/authz. (Multi-session remote: bound becomes max in-flight replication delay — vacuous in the single-host deployment.)

### THM-AUTHZ-COHERENCE (the core invariant)

If every permission mutation bumps `tenant_epoch` in-transaction, and cache hits require `entry.epoch == current(tenant)`, then no authorization decision rendered from cache postdates a denied state — i.e., no object readable under policy `P` is served after `P` revokes it, at any time.

### THM-SWR-BOUNDED-STALENESS

Serving under `stale-while-revalidate` with freshness `T` and window `S` serves entries of age ≤ `T + S` and guarantees at most one background revalidation per entry per window (RFC 5861 §3 mechanics formalized).

### THM-STAMPEDE-BOUND

With per-key singleflight and TTL jitter `U[0.9T, 1.1T]`, recompute concurrency per key ≤ 1 and expected burst amplification at expiry ≤ `1/(0.2T)` req/s — vs unbounded under naive TTL.

### THM-CAUSAL-AVAILABILITY (Mahajan, Alvisi, Dahlin 2011)

In a future async multi-host deployment, real-time causal consistency (with session guarantees) is the strongest guarantee maintainable while every read is served during a partition; hence the spec names **RT-causal**, not "eventual".

---

## YP-5: Algorithm Specification

### ALG-EPOCH-001: Epoch-Tagged Coherence

```
fn write_metadata(tx, op, tenant):
    apply(op); bump epoch_t := epoch_t + 1          # same tx
fn read_cached(key, tenant):
    (e, v) = cache.get(key)
    return v if e == current_epoch(tenant) else MISS    # stale ⇒ recompute
authz-decision entries: TTL 60 s + SWR 30 s (latency only); denies TTL 5–15 s
```

### ALG-SIEVE-001: O(1) Eviction

```
FIFO queue Q; each item has visited bit; hand pointer H
lookup(x): return x; x.visited ← 1                        # O(1) lazy promotion
insert(x): x.visited ← 0; push_back(Q, x)
evict():  while H.visited: H.visited ← 0; H ← H.next
          victim ← H.obj; H ← H.next; return victim
set_with_size(k, v): while size(k,v) > free: evict()      # evict-not-error
                     if size(k,v) > budget: Err(SizeExceeded)   # only single-entry overflow
```

### ALG-SINGLEFLIGHT-001: Coalesced Recompute

```
inflight: DashMap<Key, Arc<Notify>>
miss(k): if let Some(n) = inflight.get(k): n.notified().await; retry lookup (≤ 1 TTL)
         else insert notify; compute; broadcast; remove
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_durability.toml` — sections `cache_*`.

Critical vectors: revoke-then-read ⇒ miss (epoch), at any TTL; SIEVE vs LRU hit ratio on skewed + scan fixture traces; evict-not-error boundary; singleflight N-calls-1-compute; SWR staleness bound; ETag conditional-GET round trip; silent-drop regression (must never silently drop).

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_durability.toml`.

Summary: authz/metadata cache entries MUST be epoch-tagged; hits with stale epochs are MISSes (`DC-CACHE-001`); eviction MUST be O(1) amortized and scan-resistant (`DC-CACHE-002`); capacity overflow MUST evict, never silently drop (`DC-CACHE-003`); concurrent recomputes per key MUST coalesce (`DC-CACHE-004`); TTL is a latency optimization and MUST NOT be the coherence mechanism for authz (`DC-CACHE-005`).

---

## YP-8: Bibliography

1. Kleppmann, M. "Please stop calling databases CP or AP." Blog, **11 May 2015** (year corrected from 2020). CAP applies only to replicated linearizable registers; single-node systems are neither C nor A in that sense. [VERIFIED]
2. Lu, H., Veeraraghavan, K., Ajoux, P., Hunt, J., Song, Y. J., Tobagus, W., Kumar, S., Lloyd, W. "Existential consistency." *SOSP 2015*. DOI `10.1145/2815400.2815426` (Facebook). **Negative result:** the alternative title "Existential Consistency: Representing and Reasoning About Nonlinear Consistency Restrictions" (Wang/Eldeeb/Yang 2016) does not exist. [VERIFIED]
3. Dean, J., Barroso, L. A. "The tail at scale." *CACM* 56(2):74–80, 2013. DOI `10.1145/2408776.2408794`. Hedging/coalescing/micro-partitioning. [VERIFIED]
4. Nottingham, M. "HTTP Cache-Control Extensions for Stale Content." **RFC 5861** (May 2010): `stale-while-revalidate`, `stale-if-error` with exact examples. [VERIFIED]
5. Fielding, R., Nottingham, M., Reschke, J. (Eds.). "HTTP Caching." **RFC 9111, STD 98** (June 2022), obsoletes RFC 7234. Request collapsing (§4), invalidation on unsafe methods (§4.4), authenticated-response storage (§3.5). [VERIFIED]
6. Mahajan, P., Alvisi, L., Dahlin, M. "Consistency, Availability, and Convergence." UT Austin CS TR-11-22, May 2011. RT-causal = the availability ceiling. [VERIFIED]
7. Megiddo, N., Modha, D. S. "ARC: A Self-Tuning, Low Overhead Replacement Cache." *USENIX FAST '03*. LRU 9.19% vs ARC 20.0% hit ratio on SPC1-like workload; O(1). [VERIFIED]
8. ⭐ Zhang, Y., Yang, J., Yue, Y., Vigfusson, Y., Rashmi, K. V. "SIEVE is Simpler than LRU: an Efficient Turn-Key Eviction Algorithm for Web Caches." *USENIX NSDI '24* (Community Award). One FIFO + hand bit; < 20 LoC; ties beat LRU on skewed traces. **Correct attribution** (the "Kosaraju/Yang/Fan arXiv" version circulating is wrong on all counts). [VERIFIED]
9. Zheng, Y., Yang, J. "SR-Gadgets: Make Scan-Resistant Caching Practical." arXiv:`2609.30468` (Sept 2026). Miss-ratio cliffs/Belady metrics; scan-resistance as composable gadgets on SIEVE/S3-FIFO/ARC/2Q/TinyLFU; 5,538 production traces; up to −23.1% miss ratio. [VERIFIED — 2026 frontier]
10. Berend, D., et al. "DynamicAdaptiveClimb: Adaptive Cache Replacement with Dynamic Resizing." arXiv:`2511.21235` (Nov 2025). ~10–15% over SIEVE in fluctuating working sets. [VERIFIED]
11. Lloyd, W., et al. "Don't Settle for Eventual: Scalable Causal Consistency for Wide-Area Storage with COPS." *SOSP 2011*. Session+causality precedent. [VERIFIED via bibliography context]
12. Nextcloud/Seafile/Dropbox consistency commitments — vendor docs, not re-fetched; verify before quoting guarantees. [UNCERTAIN]
13. **Negative result:** no consistency-models paper by Elsts/Marandi located this session; do not cite. [VERIFIED ABSENCE]

---

## YP-9: Revision 1.1 — Hardening Pass (Iteration R2-6, 2026-10-14)

### R2-6.1: Frontier Verified — SIEVE is the settled default; formalize its mechanism, not just its name

**Source:** SIEVE project site (`sievecache.com`, fetched 2026-10-14) — NSDI'24 paper (Zhang, Yang, Yue, Vigfusson, Rashmi); production adoption: TiDB, Pelikan, DragonFlyDB, Ceph, PostgREST, dnscrypt-proxy, Immudb. [VERIFIED]

The epoch-tagging primitive (ALG-EPOCH-001) is FIXED in code; the eviction side is still LRU in `formal/`. SIEVE's decisive properties, now captured formally:

| Property | Why it matters for Ferro | Lean status |
|----------|--------------------------|-------------|
| Hits don't reorder the queue | O(1) flag write per authz/search hit — no list surgery, no mutex contention on the hot path | **Proved** — `lookup_preserves_order` |
| Second chances are single-use | The hand clears the visit bit on rotation — a hot entry survives, a dead one dies within one extra pass | **Proved** — `rotate_unmarks` (mechanism), `sieve_evict_unvisited_head` (victim selection) |
| Eviction removes exactly one entry per walk from a nonempty cache | Bounded work per insert-when-full | **Proved** — `sieve_evict_reduces` (strong induction on the visited count — the same measure that drives the real hand) |
| Capacity bound | Cache never exceeds cap (cap ≥ 1) | **Proved** — `insert_size_bound` |
| Fresh inserts unvisited | New objects get no second chance until used (SIEVE invariant) | **Proved** — `insert_unvisited` |
| Singleflight dedup | A caller for an in-flight key performs zero origin loads | **Proved** — `singleflight_dedup` |
| **Stampede bound** | Origin loads ≤ distinct keys, never the call count — the formal anti-thundering-herd statement behind ALG-SINGLEFLIGHT-001 | **Proved** — `stampede_bound` |

**Decision (upgrade):** replace LRU with SIEVE in the authz/search layer (the YP already prescribed SIEVE; this revision pins the mechanism and proves it). Noted frontier: DynamicAdaptiveClimb (arXiv:2511.21235, already in YP-8) beats SIEVE ~10–15% under fluctuating working sets — keep watched; SIEVE first, DACL when workload evidence exists.

### R2-6.2: Lean Proof Companion

`formal/Ferro/SieveCache.lean` (new; imported by `Ferro.lean`). Cache model: FIFO list of (key, visit-bit); eviction walk is well-founded recursion on the visited count — mirroring the hand's progress. The singleflight model (`sfStep`/`sfRun`) proves the stampede bound with the seed invariant loads ≤ keys.

### R2-6.3: Bibliography Additions

14. cacheMon. "SIEVE: an Efficient Turn-Key Eviction Algorithm for Web Caches" project site; adoption list (TiDB, Pelikan, DragonFlyDB, Ceph, PostgREST, Immudb, dnscrypt-proxy). [VERIFIED]

---

## R4-3 Addendum: SIEVE Eviction + SingleFlight — IMPLEMENTED

`crates/server/src/cache.rs`: the arbitrary HashMap-order eviction is replaced by the SIEVE mechanism exactly as formalized in R2 (`SieveCache.lean`): FIFO queue + one visited bit; hits mark visited; eviction walks the hand — visited head gets a single-use second chance (bit cleared, rotated), unvisited head is evicted; exactly one victim per over-capacity insert; stale queue entries GC lazily. Plus `SingleFlight`: per-key in-flight locks for the double-checked origin-load pattern (THM-STAMPEDE-BOUND). 10 tests, including the visited-head-survives and single-use-second-chance orderings.
