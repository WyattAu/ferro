# YP-SYNC-CONFLICT-001: File Sync, Version Vectors & Conflict Resolution

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-SYNC-CONFLICT-001                       |
| Domain       | Sync & Replication                         |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/desktop/src/sync/`, `crates/common/src/chunk.rs` |
| Supersedes   | (none; extends implicit behaviour of YP-STORAGE-CAS-001 manifests) |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| Conflict decided by comparing `(size, mtime)` in a `HashMap` | `crates/desktop/src/sync/engine.rs:249-257` |
| Default strategy `KeepBoth`, suffix `<filename> (conflict).ext` | `crates/desktop/src/sync/conflict.rs:11-17` |
| Manifest/diff is hash-only (`HashSet<&str>`), no lengths | `crates/desktop/src/sync/block.rs:176-186` |
| No version vector, no Lamport clock, no revision identity anywhere | repo-wide |

**Consequences:** (1) no true concurrency detection — silent LWW loss when clocks agree; (2) rename-vs-edit indistinguishable from delete-vs-edit; (3) `KeepBoth` suffix not idempotent (no `MaxConflicts` cap); (4) no basis for causal-stability GC.

---

## YP-2: Executive Summary

### Problem Statement

Ferro's sync treats file conflicts as a metadata diff problem decided by wall-clock mtime. The literature and production evidence (Kleppmann et al. demonstrate real concurrent-move bugs in Dropbox and Google Drive, IEEE TPDS 2021) show this class of design silently loses data. Ferro requires: (a) true concurrency detection, (b) deterministic conflict resolution independent of clock skew, (c) move/rename safety, (d) bounded conflict-copy growth.

### Design Decision (the headline)

> **Do NOT put a CRDT in the block layer. Put a minimal, server-sequenced, op-based metadata log in the metadata layer only, and keep Dropbox-style explicit conflict copies as the file-level outcome.**

Blocks are content-addressed and immutable — a pure function of content. They never merge. Conflict resolution lives entirely in the metadata/tree layer over a small ordered op log, with a single sequencer (the server). Binary formats (.docx/.xlsx/.pdf/video) cannot be CRDT-merged into anything an application can open; per-block CRDT metadata would multiply manifest size an order of magnitude for zero benefit.

### Guarantees

- **Convergence (metadata):** all replicas applying the sequenced op prefix reach identical trees.
- **No silent loss:** every write op is represented in the materialized state of at least one device — at its primary path or a conflict path.
- **Determinism:** the winner of a concurrent-write conflict is a pure function of the concurrent set, `(lamport, device_id)`-ordered — never wall-clock.
- **Move identity:** concurrent subtree moves yield identical path sets with exactly one copy per node.

---

## YP-3: Nomenclature

| Term | Definition |
|------|------------|
| Op | A metadata-layer operation: `Create`, `Write{rev}`, `Move{from,to}`, `Delete`, `Mkdir`, `Chmod`. |
| Sequencer | The single server process that totally orders ops per volume (Ferro server). |
| Lamport clock | Monotone per-device counter `L`; on local op `L += 1`; on receiving remote op `L = max(L, L_remote) + 1`. |
| Version vector (VV) | Sparse map `device_id -> u64` of the last op counter applied per device. |
| Causal stability | An op is stable when `head(o) <= ⋀_d ack(v,d)` componentwise; its conflict material is GC-eligible. |
| Conflict copy | Materialization of a losing concurrent write at a sibling path; Dropbox/Seafile/Nextcloud parity. |
| `MaxConflicts` | Per-directory cap (32) on conflict copies before resolution is forced. |
| Advisory lock | `LOCK` with TTL 30 min, owner `(user, device, app)`; prevents conflicts rather than resolving them. |
| Move transform | The transform relation for concurrent tree ops from Kleppmann et al. TPDS 2021 (formally proved in Isabelle/HOL). |

---

## YP-4: Theoretical Foundation

### DEF-LWW-FAIL-001: The (size, mtime) Anti-Pattern

Ferro's current predicate `conflict(f) ⟺ size₁ ≠ size₂ ∨ mtime₁ ≠ mtime₂` is *not* a concurrency detector: it conflates "different" with "concurrent" and "equal" with "ordered". Two devices editing offline for 10 days produce a conflict iff mtimes differ; two devices with identical clocks produce silent LWW loss. Formally, the relation induced is not a partial order over write events, so "happens-before" cannot be recovered. **This is the property THM-NO-SILENT-LOSS repairs.**

### DEF-VV-001: Version Vector Order

For version vectors `u, v ∈ (DeviceId ⇀ ℕ)`: `u ≤ v ⟺ ∀d: u(d) ≤ v(d)`; `u ∥ v ⟺ u ≰ v ∧ v ≰ u`. Two writes `w₁`, `w₂` are **concurrent** iff their VV contexts are incomparable. This is the *only* sound concurrency test.

### THM-CONFLICT-CONVERGENCE

For any two devices `d₁, d₂` and any prefix `σ` of the total op order produced by the sequencer, applying `σ` to `d₁`'s local state and to `d₂`'s local state yields identical metadata trees `M₁(σ) = M₂(σ)`.

*Requires:* ⊕-Closure of the move-transform relation (Move∘Write, Move∘Delete, Move∘Move, Delete∘Write) and associativity + idempotence of the transform on the operation multiset. Does **not** hold for file *content* (binary formats); content convergence is achieved by conflict copies, not merging.

### THM-NO-SILENT-LOSS

For every op `o` applied at the sequencer, `o` is represented in the resulting materialized state of at least one device — either at its primary path `p` or at a conflict path `conf(p, o)`: `|{repr(o)}| ≥ 1`. Contrapositive: for all concurrent `o, o'`, `¬(o and o'` collapse to a single path`)`. **Ferro's current (size, mtime) scheme violates this theorem.**

### THM-MOVE-IDENTITY

∀ concurrent ops `(Move(σ, a, b), Move(σ', b, c))` applied in either order, the set of materialized paths of `σ ∪ σ'` is identical, and exactly one copy of each node of `σ` appears. Instantiates the acyclicity/termination conditions of Kleppmann et al. (TPDS 2021), which are formally proved in Isabelle/HOL and have a Rust reference implementation (`crdt_tree`).

### THM-CONFLICT-DETERMINISM

Given the set `C` of ops concurrent with the maximal op at path `p`, the winning path is a pure function of `C`: the op maximizing `(lamport, device_id)` under the total order `≤` on `(u64, DeviceId)`. Two sequencers that have seen the same `C` choose the same winner. `⇒` Ferro's current `(size, mtime)` scheme is not deterministic (violates this theorem).

### THM-CAUSAL-STABILITY-GC

Let `VV[v]` be the version vector of volume `v` and `ack(v,d)` the vector acked by device `d`. An op `o` with `head(o) ≤ ⋀_d ack(v,d)` (componentwise) is causally stable and its conflict-path material is reclaimable. Conversely, if `¬(head(o) ≤ ⋀_d ack(v,d))` and some device in `support(o)` has not acked, reclaiming `o`'s metadata is unsound.

*Precondition satisfied:* Ferro's device set is small and enumerable (50 users × N devices) — the closed-membership condition every P2P system lacks. Exploit it.

---

## YP-5: Algorithm Specification

### ALG-OPLOG-001: Server-Sequenced Metadata Op Log

```
Op fields:
  op_id        = (device_id: u64, lamport: u64)
  parent_vv    : VersionVector          # context, for concurrency test
  kind         ∈ {Create, Write{revision_id}, Move{from,to}, Delete, Mkdir, Chmod}
  path         : String                 # display only — never an authz key (see YP-AUTHZ-POLICY-001)
  content_rev  : Blame3 root of manifest (see YP-DEDUP-CDC-001)

Sequencer (server), per volume:
  1. receive op o; assign seq = next_seq++
  2. C := {o' ∈ pending_at_p : o'.vv ∥ o.vv}          # concurrent set
  3. if kind == Write ∧ C contains Write:
       winner := argmax_{(lamport, device_id)} (o ∪ C)
       materialize winner at p
       for each loser l: materialize at
         conf(p, l) = stem(p) + " (conflict " + RFC3339(l) + " by " + user(l) + ")" + ext(p)
  4. if kind == Move: apply transform relation T (TPDS 2021):
       T(Move, Move)  → target-disambiguation or acyclic reorder
       T(Move, Write) → propagate to new path
       T(Move, Delete)→ tombstone-orphan policy (modify wins; file resurrects at new path)
       T(Delete, Write)→ modify wins (resurrect)
  5. append to log; ack to originator with seq; broadcast VV delta
  6. stability sweep: any o with head(o) <= ⋀ ack → mark GC-eligible
```

### ALG-LAMPORT-001: Clock Maintenance

```
on local op:   L ← L + 1
on recv op o': L ← max(L, o'.lamport) + 1
conflict tie-break order: (lamport, device_id)   # NEVER wall-clock
```

### ALG-LOCK-ADV-001: Advisory Locks (conflict prevention)

```
acquire(p, owner=(user,device,app), ttl=1800s) → granted | held_by(other)
- surfaced in desktop client before write
- TTL refresh every 600s; expiry releases silently
- advisory only: never blocks a sequencer-ordered write; exists to prevent, not resolve
```

### Parameters

| Parameter | Value | Rationale |
|---|---|---|
| Lamport counter | `u64` per device | collision-free at any realistic op rate |
| Version vector pruning | entries older than 90 d | bounded state, closed membership |
| Conflict name | `{stem} (conflict {RFC3339UTC} by {user}){ext}` | Seafile-shape, deterministic, sortable |
| `MaxConflicts` per dir | 32 | Syncthing's unbounded growth is a documented defect |
| Lock TTL | 1800 s, refresh 600 s | Collabora/WOPI-compatible horizon |
| Strategy enum | `KeepBoth` only; `KeepLocal`/`KeepRemote` deprecated | data-loss footguns in multi-tenant product |

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_sync_stack.toml` — sections `sync_*`.

Critical vectors: silent-loss regression (two devices, equal mtime, divergent content → MUST produce conflict copy), move-vs-move determinism (both orderings → identical tree), lamport tie-break, `MaxConflicts` cap, GC-safety (unstable op must not be reclaimed).

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_sync_stack.toml`.

Summary of inviolable invariants:
- Conflict decisions NEVER consult `mtime` or `size` (`DC-SYNC-001`).
- Winner selection is a pure function of the concurrent set (`DC-SYNC-002`).
- `KeepBoth` is the only conflict strategy exposed by default (`DC-SYNC-003`).
- Conflict-copy suffix is idempotent; re-conflicting an already-suffixed path replaces the timestamp, never nests (`DC-SYNC-004`).
- GC of conflict material only under causal stability (`DC-SYNC-005`).

---

## YP-8: Bibliography

1. Ellis, C. A., Gibbs, S. J. "Concurrency control in groupware systems." *ACM SIGMOD '89*, pp. 399–407. DOI `10.1145/67544.66963`. [VERIFIED]
2. Gomes, V. B. F., Kleppmann, M., Mulligan, D. P., Beresford, A. R. "Verifying Strong Eventual Consistency in Distributed Systems." *PACMPL* 1 (OOPSLA), Art. 109, 2017. arXiv:`1707.01747`. [VERIFIED]
3. Kleppmann, M., Mulligan, D. P., Gomes, V. B. F., Beresford, A. R. "A highly-available move operation for replicated trees." *IEEE TPDS* 33(7):1711–1724, 2021/2022. DOI `10.1109/TPDS.2021.3118603`. Formally proved in Isabelle/HOL; documents real Dropbox/Google Drive concurrent-move bugs. [VERIFIED]
4. Kleppmann, M. "Moving Elements in List CRDTs." *PaPoC 2020*. [VERIFIED]
5. Weidner, R., Kleppmann, M. "The Art of the Fugue: Minimizing Interleaving in Collaborative Text Editing." arXiv:`2305.00583`; *IEEE TPDS* 36(11):2425–2437, 2025. [VERIFIED]
6. Gentle, J., Kleppmann, M. "Collaborative Text Editing with Eg-walker: Better, Faster, Smaller." arXiv:`2409.14252`, *EuroSys 2025*. [VERIFIED]
7. Alonso, G., Weiss, S., Koprowski, A. "Approaches to Conflict-free Replicated Data Types." arXiv:`2310.18220` (survey; formalizes causal stability). [VERIFIED]
8. Wu, J., et al. "From Causality to Stability: Understanding and Reducing Meta-Data in CRDTs." *MPLR 2020*. DOI `10.1145/3426182.3426183`. [VERIFIED]
9. Kleppmann, M., Wiggins, A., van Hardenberg, P., McGranaghan, M. "Local-first Software: You Own Your Data, in Spite of the Cloud." Ink & Switch, 2019. [VERIFIED]
10. Automerge 3.0 announcement, 2025-07-14: >10× memory reduction; Moby-Dick doc 700 MB → 1.3 MB. Context for why full-CRDT remains wrong for binary files despite this. [VERIFIED]
11. Da, X., Kleppmann, M. "Extending JSON CRDTs with Move Operations." *PaPoC 2024*. arXiv:`2311.14007`. [VERIFIED]
12. Stewen, T., Kleppmann, M. "Undo and Redo Support for Replicated Registers." *PaPoC '24*. arXiv:`2404.11308`. [VERIFIED]
13. Kleppmann, M. "Using Bloom filters to efficiently synchronise hash graphs." Blog, 2020. (Basis of mesh inventory exchange; see YP-MESH-TRANSPORT-001.) [VERIFIED]
14. **Negative result:** "Diamond Types and semilattices" — no citable publication exists under this title; the `josephg/diamond-types` repository is real but unpublished as a paper. Do not cite in downstream specs. [VERIFIED ABSENCE]
15. **Negative result:** "Siena multi-master file system, SOSP 2009" — no primary record located this session; treat as [UNCERTAIN] and do not cite.
16. Syncthing documentation — version-vector conflict handling, `.sync-conflict-<date>-<time>-<modifiedBy>` naming, device-ID tie-break on first 63 bits. [VERIFIED]
17. Nextcloud Desktop manual — conflict files not uploaded by default (`OWNCLOUD_UPLOAD_CONFLICT_FILES=1`); advisory locks only. [VERIFIED]
18. Dropbox LAN Sync / "Streaming File Sync" engineering posts — conflicted-copy naming `<editor> (conflicted copy) <date>`. [VERIFIED]

---

## YP-9: Revision 1.1 — Hardening Pass (Iteration R2-1, 2026-10-14)

### R2-1.1: Frontier Finding — Merkle-Rooted Op Logs

**Source:** Sanjuan, Poyhtari, Teixeira, Psaras. "Merkle-CRDTs: Merkle-DAGs meet CRDTs." arXiv:2004.00107 (2020). [VERIFIED — fetched this session; note arXiv:2004.09368 is an unrelated econometrics paper and is NOT this work].

The op log of ALG-OPLOG-001 gains a **running root digest**: `H_n = hash(H_{n-1} ‖ seq(n) ‖ op(n))`. The root doubles as (a) a **logical clock** — strictly monotone under append; (b) a **replica-equality oracle** — two devices advertising equal roots have identical log prefixes up to hash collision, so the pre-delta handshake exchanges 32 B roots instead of diffing manifests. Neither Syncthing (VV exchange only), Dropbox (block-list diff), nor Seafile ships this.

**New theorems (added to scope):**

| ID | Statement | Formal status |
|----|-----------|---------------|
| `THM-LOG-ROOT-CLOCK` | Appending any op strictly advances the log root (`logDigest σ < logDigest (σ ++ [x])`) | **Proved** — Lean `logDigest_strictly_advances` |
| `THM-LOG-REPLICA-EQUALITY` | Distinct log prefixes never share a root without a hash collision (`logDigest σ ≠ logDigest (σ ++ [x])`); collision resistance delegated to the real SHA-256/BLAKE3 hash | **Proved** (conditional form) — Lean `logDigest_append_ne` |
| `THM-WINNER-SET-ORDER-INDEPENDENT` | Adjacent-swap invariance of winner selection over the concurrent set under distinct `(lamport, device)` keys — winner is a function of the SET, not its serialization order | **Proved** — Lean `winner_adjacent_swap` + `step_swap` |
| `THM-VV-CONCURRENCY-SOUNDNESS` | VV incomparability is irreflexive, symmetric, and never conflates "different" with "concurrent" — the soundness gap that the (size, mtime) anti-pattern violates | **Proved** — Lean `vv_concurrent_irrefl`, `vv_concurrent_symmetric`, `concurrent_not_ordered` |
| `THM-LAMPORT-CAUSALITY` | A received op's lamport is strictly below the receiver's next clock; equal lamport values across devices imply genuine concurrency, never hidden causality | **Proved** — Lean `lamport_recv_above`, `lamport_causality` |
| `THM-WINNER-DOMINATES` | The winner dominates every member of the concurrent set; the running winner never drops below the seed | **Proved** — Lean `winner_ge_of_mem`, `winnerOf_ge_seed` |
| `THM-NO-SILENT-LOSS` (two-write case) | Both writes are materialized: winner at the primary path, loser at `2p+1`-encoded conflict path; the two paths are distinct | **Proved** — Lean `no_silent_loss`, `loser_materialized`, `conflict_copy_distinct` |

### R2-1.2: Lean Proof Companion

`formal/Ferro/SyncConflict.lean` (new; imported by `Ferro.lean`; full `lake build` green). The key encoding: winner key = `lamport * 2³² + device` (exact lexicographic order for u64 device IDs, matching the spec's field widths). Key-distinctness for `winner_adjacent_swap` is justified by `lamport_recv_above` (a device never emits two ops with one lamport value).

### R2-1.3: Considered and Rejected — Dotted Version Vectors

DVVs (Preguiça et al.) buy O(1) GC without pruning but require dynamic replica membership bookkeeping. Ferro's device set is closed and enumerable (50 users × N devices); 90-day pruning (YP-5) is strictly simpler with equivalent boundedness. Revisit only if multi-tenant device churn breaks pruning.

### R2-1.4: Bibliography Additions

19. Sanjuan, H., Poyhtari, S., Teixeira, P., Psaras, I. "Merkle-CRDTs: Merkle-DAGs meet CRDTs." arXiv:2004.00107, 2020. [VERIFIED]
20. **Verification note:** arXiv:2004.09368 is "Awareness of crash risk improves Kelly strategies…" (econ.EM) — do not confuse with the Merkle-CRDT paper. [VERIFIED ABSENCE of the wrong-ID citation]

---

## R3-4 Addendum (Revision 1.2): Delta-State CRDTs & Causal-Stability GC

**Source verified:** Almeida, Shoker, Baquero, "Delta State Replicated Data Types," JPDC 111:162-173, 2018 (arXiv:1603.01529) [VERIFIED 2026-10-14].

### ALG-DELTAMUT-001
Collaborative objects (chat, whiteboard, tags) disseminate **delta mutators** — the increment as a standalone delta-state with only the changed component nonzero — joined into local and remote states. Small messages with state-based (reliable-channel-free) semantics: the best of op-based and state-based CRDTs.

### THM-DELTA-DECOMPOSITION
Joining a delta mutator into any state equals joining the full incremented state — deltas are behaviorally indistinguishable from full states. [Proved — Lean `gc_delta_decomposition`, DeltaCRDT.lean]

### ALG-CAUSALGC-001
Op-log entries at or below the per-replica stable watermark are garbage-collected.

### THM-GC-DROPS-ONLY-STABLE / THM-GC-KEEPS-ABOVE / THM-GC-IDEMPOTENT
GC drops only sub-watermark entries; every above-watermark entry survives; collection is idempotent. [Proved — Lean `gc_drops_only_stable`, `gc_keeps_above_watermark`, `gc_idempotent`]

### Comparison
Yjs/Automerge ship delta-encoded doc updates but without formal convergence/GC contracts; Redis CRDTs (Enterprise) ship delta counters without causal-stability GC proofs. Ferro's differentiator remains the theorem-pinned op-log (R2) — this addendum extends it with bounded-memory GC.

---

## R4-2 Addendum: VV Conflict Classification — IMPLEMENTED

`crates/sync-protocol/src/conflict.rs`: `classify` (Identical / LocalDominated / RemoteDominated / **Concurrent**) by vector clock only — never wall clock; `deterministic_winner` ((content_hash, path) lexicographic — every replica independently picks the same side); `conflict_copy_path` (deterministic loser-preservation name). `engine.rs::resolve_conflict` LastWriterWins branch now checks the class: concurrent conflicts preserve the loser (THM-NO-SILENT-LOSS), dominated ones may fast-forward. 7 tests.
