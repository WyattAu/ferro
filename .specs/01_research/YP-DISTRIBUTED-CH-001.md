# YP-DISTRIBUTED-CH-001: Rendezvous Placement, Minimal Disruption & Weighted Routing

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-DISTRIBUTED-CH-001                      |
| Domain       | Distributed Storage / Placement            |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration R3-2) |
| Date         | 2026-10-14                                 |
| Crate Ref    | `crates/consistent-hash/src/ring.rs`, `formal/Ferro/Rendezvous.lean`, `crates/backend-router` |
| Supersedes   | virtual-node hash ring internals (public `HashRing` API preserved) |

---

## YP-1: Ground Truth (Prior Implementation)

| Fact | Location |
|---|---|
| SHA-256 ring, 150 vnodes/node, weight scales vnode count | old `add_node` |
| **Bug:** `remove_node` removed only `virtual_nodes` positions; weighted nodes left orphan vnodes that kept routing to the removed node | old `remove_node` |
| **Bug:** O(vnodes × nodes) reverse map per lookup; `"unknown"` fallback node id | old `find_node_id_for_position` |
| Synthetic `virtual_node_index` (`i % virtual_nodes`) in diagnostics | old `entries()` |

**Consequences:** removing a weighted node silently kept serving it traffic (correctness), and every `get_node` paid a full vnode re-hash scan (performance).

---

## YP-2: Executive Summary

### Problem Statement

Ferro routes keys to heterogeneous, weighted backends (local FS, S3, GCS, Azure) where the node set changes on failure and reconfiguration. Placement must be deterministic, weighted-exact, and minimally disruptive; at Ferro's scale (≤ ~16 backends) the machinery must not add bookkeeping failure modes.

### Design Decision (the headline)

> **Replace the virtual-node ring with weighted rendezvous hashing (highest-random-weight): each key goes to the node maximizing `ln(hash(key,node))/weight`. Same public `HashRing` API; zero ring bookkeeping; node removal is correct by construction; the entire vnode-orphan bug class ceases to exist.**

Analyzed and **rejected**: jump consistent hash (arXiv:1406.2294; verified); buckets must be numbered sequentially, no weights, no arbitrary removal; ideal for flat shard arrays, wrong shape for weighted backend routing. Maglev lookup tables (NSDI'16) rejected: memory cost sized for load balancers with hundreds of backends. Plain Karger rings with vnodes: superseded; every vnode bookkeeping bug (collisions, orphaned removals, weight drift) is a bug HRW cannot have.

### Guarantees

- **Member soundness:** a placement is always an existing node; the empty set places nothing. [Proved; Lean `argmaxBy_mem`, `argmaxBy_exists`/`argmaxBy_none_eq_nil`]
- **Score domination:** no member out-scores the winner. [Proved; Lean `argmaxBy_dominates`]
- **Strict-top uniqueness:** when a member strictly out-scores all others it is THE winner; the hash uniformity assumption (external) makes strict tops the overwhelming common case. [Proved; Lean `strict_top_unique`, `strict_top_is_pick`]
- **Minimal disruption:** removing node `r` moves only keys placed on `r`; every other placement is provably stable. [Proved; Lean `minimal_disruption`]
- **Strict addition monotonicity:** adding a node captures keys only by strictly out-scoring the incumbent; otherwise nothing moves. [Proved; Lean `strict_addition_keeps`, `strict_addition_moves`]

---

## YP-3: Nomenclature

| Term | Definition |
|------|------------|
| HRW / rendezvous | For key `k`, each node `n` computes `score(k,n)`; max wins; ties prefer the earlier node (deterministic given a node ordering). |
| Weighted score | `ln(u01(hash(k,n))) / w`; node with weight `w` wins share `w/Σw` in expectation (Thaler-Ravishankar weighted rule). |
| Minimal disruption | Node membership changes re-place only the keys whose winner changed membership. |
| Strict top | Winner out-scores every other member (a.s. under uniform hashing); the case where uniqueness is provable without tie-order assumptions. |

---

## YP-4: Theoretical Foundation

### THM-CH-PICK-MEMBER / THM-CH-PICK-EXISTS
Placement is total on nonempty node sets and returns a member. [Proved; Lean `argmaxBy_exists`, `argmaxBy_mem`]

### THM-CH-PICK-DOMINATES
`score(k, winner) ≥ score(k, n)` for all members `n`. [Proved; Lean `argmaxBy_dominates`]

### THM-CH-STRICT-TOP-UNIQUE
A strict top is the unique possible winner. [Proved; Lean `strict_top_unique`]

### THM-CH-MINIMAL-DISRUPTION
`argmax(l) = m ∧ m ≠ r → argmax(l \ {r}) = m`; the ring-removal invariant the old vnode implementation violated for weighted nodes. [Proved; Lean `minimal_disruption`, with the filter formulation]

### THM-CH-ADDITION-KEEPS / THM-CH-ADDITION-MOVES
`f(nw) ≤ f(m)` → adding `nw` keeps `m`; `f(nw) > f(m)` → the new node takes exactly the incumbent's keys. [Proved; Lean `strict_addition_keeps`, `strict_addition_moves`]

### Axioms (external, stated honestly)
- Hash uniformity of `SHA-256(key ‖ 0x00 ‖ node)`; external, as collision resistance is in HashConsistency. The Lean layer proves all structure above it.

---

## YP-5: Algorithm Specification

### ALG-HRW-001

```
score(k, n)  = ln(max(u01, ε)) / weight(n),   u01 = (pair_hash(k,n) >> 11) · 2⁻⁵³
pair_hash    = first 8 bytes (BE) of SHA-256(key ‖ 0x00 ‖ node_id)   [domain-separated]
place(k)     = argmax_n score(k, n)      ties → deterministic node ordering
replicas(k, R) = nodes sorted by score desc, take R     [distinct by construction]
weight guard: w ∉ (0, ∞) or NaN → 1.0 (fail-closed to unit weight)
```

### Compatibility contract

- `HashRing::new(v)` / `with_defaults()` signatures retained; `v` ignored (HRW needs no vnodes; `virtual_nodes_per_physical()` reports 1).
- `get_node` → `Option<(&T, String)>`; `get_nodes(k, R)` → primary-first distinct replicas; `physical_nodes()` sorted; `entries()` one per physical node.
- All 6 pre-existing `backend-router` tests pass unchanged; `test_consistent_hash_backend_remove_node` now also covers weighted removal (the previously broken case) via `test_weighted_removal_leaves_no_orphans`.

### Parameters

| Parameter | Value | Rationale |
|---|---|---|
| node count | ≤ ~16 backends | O(N) scan per lookup is trivial at Ferro scale |
| hash | SHA-256 first 8 bytes | same primitive as the rest of the stack; no new dependency |
| tie-break | deterministic node order | reproducibility across processes |

---

## YP-6: Comparison: what incumbents do

| System | Placement | Notes |
|---|---|---|
| Memcached clients (ketama) | vnode ring | the design Ferro previously copied; ketama itself documents the vnode-count/weight drift hazards |
| Cassandra | vnode/token ring | token metadata service needed; out of scale here |
| ScyllaDB | moved away from vnodes toward deterministic tokens | independent confirmation that vnode bookkeeping is the failure-prone part |
| Cloudflare LB, Discord's fluent | HRW / rendezvous | HRW is the production-proven choice for small weighted sets |
| Jump hash (Google) | O(ln N), no memory | sequential buckets only; storage-shard-shaped, not router-shaped |

**Verdict:** HRW is not novel; what is Ferro-specific is (a) deleting the vnode layer entirely rather than patching it, (b) pinning minimal disruption + addition monotonicity as proved theorems with a test vector, and (c) documenting the jump-hash/Maglev rejections with verified citations so the decision is auditable.

## YP-7: Test Vector Specification

Empty ring places nothing; single-node routing; 3-node distribution within ±150‰; removal leaves exactly one node; **weighted removal leaves no orphans** (500 keys, all re-routed); minimal disruption on removal (keys off the removed node never move); 2-replica distinctness; 3-replica distinctness; weight 3.0 wins ~75%; add-node stability; zero/NaN weight falls back to unit; cross-instance determinism; entries/is_empty accounting.

## YP-8: Domain Constraints

- DC-CH-001: placement MUST be a pure function of (key, node-set, weights); no ring state.
- DC-CH-002: node removal MUST re-place only keys previously won by that node (minimal disruption).
- DC-CH-003: replica sets MUST contain distinct physical nodes.
- DC-CH-004: invalid weights (≤ 0, NaN, ∞) MUST be treated as unit weight, never as `-∞` scores that blackhole a node.

## YP-9: Bibliography

1. Thaler, D., Ravishankar, C. "Using Name-Based Mappings to Increase Hit Rates," IEEE/ACM ToN 6(1), 1998; highest-random-weight selection. [VERIFIED; canonical HRW citation]
2. Lamping, J., Veach, E. "A Fast, Minimal Memory, Consistent Hash Algorithm," arXiv:1406.2294. Abstract page fetched 2026-10-14: "buckets must be numbered sequentially… more suitable for data storage applications." [VERIFIED; rejected with reason]
3. Karger, D. et al. "Consistent Hashing and Random Trees," STOC 1997. [UNCERTAIN; not fetched this session; foundational, widely reproduced]
4. Eisenbud, D. et al. "Maglev: A Network Load Balancer," NSDI 2016. [UNCERTAIN; not fetched; rejected on memory-footprint grounds]

---

## R4-6 Addendum: Minimal Disruption: PROVED

The R3 test-pinned gap is closed: `formal/Ferro/Rendezvous.lean` now proves `argmax_dominates_snoc` (winner dominance, snoc-induction) and `minimal_disruption`; removing node `r` moves only keys placed on `r` (`argmaxBy f l = some m → m ≠ r → argmaxBy f (l.filter (≠ r)) = some m`). Proof shape: length-measure induction with snoc decomposition (`exists_snoc`), case analysis on the removed element's position; `hrwStep` tie semantics (earliest node wins) are load-bearing in the kept-last-element case.
