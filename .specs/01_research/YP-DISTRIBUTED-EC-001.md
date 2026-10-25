# YP-DISTRIBUTED-EC-001: Layered Erasure Coding, Prefer-Local Repair & Scrubbing

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-DISTRIBUTED-EC-001                      |
| Domain       | Distributed Storage / Durability           |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration R3-1) |
| Date         | 2026-10-14                                 |
| Crate Ref    | `crates/distributed/src/erasure.rs`, `formal/Ferro/ErasureCode.lean` |
| Supersedes   | (erasure behavior within YP-STORAGE-CAS-001 scope; RS-only coder unchanged) |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| RS(4,2) GF(2^8) coder via `reed-solomon-erasure` 6, plus legacy 1-parity XOR coder | `crates/distributed/src/erasure.rs` |
| No local reconstruction groups — every single-shard loss pays a full-stripe RS repair | `ReedSolomonErasureCoder::decode` |
| No scrubber: corruption is discovered on read only; dead shard backends never proactively repaired | `erasure_storage.rs` |
| Per-shard SHA-256 checksums exist but nothing consumes them for repair | `Shard.checksum` |

**Consequences:** a single lost shard backend forces reads of all `k` fragments per object (Azure measured this exact repair-I/O tax as the motivation for LRC); silent bitrot accumulates until a read fails.

---

## YP-2: Executive Summary

### Problem Statement

Ferro's erasure layer must survive shard-device loss on a single TrueNAS-class host with `shard_backends`, keep repair I/O proportional to the failure (not the stripe), and detect/repair silent corruption (bitrot) without operator involvement — the properties Azure LRC, Ceph LRC/SHEC/CLAY, and MinIO all ship, and which incumbents in Ferro's own class (Nextcloud/Seafile: replication only) do not.

### Design Decision (the headline)

> **Adopt an LRC-style layered code: `k` data shards + one group-local XOR parity shard per `k/l` group + `r` global RS parity shards; decode prefers the local path (reads `k/l` fragments instead of `k`); a checksum-gated scrubber repairs missing/corrupt shards in place via the cheapest applicable path.**

For 50 users / one host: **k=4, l=2, r=2** — 2.0× overhead, any 2 losses survived, isolated single-shard loss repaired from 2 fragments instead of 4.

### Guarantees

- **Local repair bound:** isolated single-shard loss reads ≤ `k/l` fragments; strictly fewer than the RS stripe when `k/l < k`. [Proved — Lean `local_repair_reads_less`]
- **Preference soundness:** the local path is taken only when the group genuinely can repair. [Proved — Lean `decode_local_sound`]
- **Preference completeness:** a reported failure implies the MDS budget was genuinely unmet — fail-closed, no missed path. [Proved — Lean `decode_failed_complete`]
- **Failure is informational:** on failure no data is delivered (pairs with the external MDS-necessity axiom). [Proved — Lean `failure_is_informational`]
- **Local repair correctness:** parity XOR remaining siblings recovers the lost shard exactly. [Proved — Lean `local_repair_correct`]
- **Scrubber progress:** each successful scrub decrements the stripe's missing count by exactly one; healthy shards are never rewritten. [Proved — Lean `scrub_progress`, `scrub_untouched`]

---

## YP-3: Nomenclature

| Term | Definition |
|------|------------|
| Stripe | One object's full shard set: `k + l + r` shards. |
| Group `g` | Data shards `[g·gs, (g+1)·gs)`; its local parity is shard `k+g`. |
| Local parity | Byte-wise XOR of the group's data shards (all-ones GF(2^8) codeword row). |
| Global parity | Reed-Solomon GF(2^8) rows over the `k` data shards only; local shards are NOT RS inputs. |
| MDS budget | `k` fragments of the `k + r` RS set — the recoverability threshold. |
| Scrub | verify(checksum) → repair(missing/corrupt, prefer local) cycle. |

---

## YP-4: Theoretical Foundation

### THM-EC-LAYOUT-ACCOUNTING
A stripe stores exactly `k + l + r` shards; data never exceeds stored total. [Proved — Lean `layout_accounting`]

### THM-EC-NAMESPACE-DISJOINT
Local parity index `k+g` (g < l) never collides with global parity index `k+l+j`; distinct groups never collide. [Proved — Lean `local_global_disjoint`, `local_parity_injective`]

### THM-EC-LOCAL-REPAIR-BOUND
`readFragments(local) = gs ≤ k = readFragments(global)`, strict when `gs > 1` (proper partition, `l ≥ 2`). Azure LRC's repair-read reduction, in the degenerate-but-valid member of the family (RS + local XOR groups, as in HDFS XOR-EC / Ceph LRC with one layer). [Proved — Lean `local_repair_reads_less`]

### THM-EC-LOCAL-REPAIR-CORRECT
`(d ⊕ x) ⊕ x = d` — XOR involution underpins group repair. [Proved — Lean `local_repair_correct`]

### THM-EC-DECODE-SOUND / THM-EC-DECODE-COMPLETE
Decode never takes a path whose precondition is unmet; failure implies no path existed. [Proved — Lean `decode_local_sound`, `decode_failed_complete`]

### THM-EC-SCRUB-PROGRESS / THM-EC-SCRUB-GATING
`missing(after repair) + 1 = missing(before)`; shards outside the repair index are untouched. [Proved — Lean `scrub_progress`, `scrub_untouched`, `filter_missing_update_range`]

### Axioms (external, stated honestly)
- **MDS necessity:** the RS decoder delivers data only when ≥ `k` of `k+r` fragments exist (Vandermonde full-rank property — external, as collision resistance is in HashConsistency). [Lean `mds_necessity`]
- SHA-256 collision resistance (per-shard bitrot gate).

---

## YP-5: Algorithm Specification

### ALG-LRC-001 (encode)

```
layout:    shards 0..k = data | k..k+l = local parity (group g → index k+g) | k+l..k+l+r = global RS parity
encode:    pad data to k·chunk; data_bufs[0..k]
           local[g]   = XOR of data_bufs[g·gs .. (g+1)·gs)         (per group)
           global[0..r] = RS_encode(data_bufs)                      (GF(2^8))
```

### ALG-LRC-DECODE-001 (prefer-local cascade)

```
decode(shards):
  if all k data present                  → Direct (0 extra reads)
  else if exactly 1 data shard missing
       ∧ its group's local parity present
       ∧ all group siblings present      → Local  (gs−1 data + 1 local reads)
  else if present(data) + present(global RS) ≥ k
                                         → Global (RS reconstruct)
  else                                   → FAILED (no data delivered)
```

### ALG-SCRUB-001

```
scrub(shards):
  bad ← [i : checksum(shards[i]) ≠ sha256(data) ∨ shards[i] = None]
  for i in bad:  shards[i] ← reconstruct(shards with i = None)   // local path first
  return repaired indices                                        // beyond-budget holes stay None
```

### Parameters (50 users / one TrueNAS host)

| Parameter | Value | Rationale |
|---|---|---|
| k (data shards) | 4 | 2 backends can die without data loss; matches existing default |
| l (groups) | 2 | gs=2: local repair reads 2 fragments vs 4 |
| r (global RS parity) | 2 | any 2 concurrent shard losses |
| shard_size | 1 MiB | unchanged |
| overhead | 2.0× | (k+l+r)/k = 8/4 |

### Honest scope limits (stated, not hidden)

- This is the **one-layer LRC** (RS + local XOR groups), not full Azure LRC with multiple local groups per parity nor Ceph SHEC's overlapping groups; 2 losses in one group fall back to the global path.
- Local parity cannot assist the RS decoder for multi-loss stripes (Azure's jointly-designed LRC can); the win is isolated single-shard/bitrot repair I/O.
- The scrubber repairs within the code's loss budget; beyond it, holes remain `None` and reads fail closed.

---

## YP-6: Comparison — what incumbents do

| System | Scheme | Repair read for 1 loss | Notes |
|---|---|---|---|
| Azure WAS (ATC'12) | LRC(k, l, r) jointly designed | k/l + 1 | Best Paper; our reference model |
| Ceph | jerasure/ISA RS, **LRC**, **SHEC**, **CLAY** plugins | plugin-dependent | CLAY = minimum-bandwidth regeneration (watched, not adopted — complexity out of scale for 1 host) |
| HDFS EC | RS(6,3) + XOR local groups (HDFS-EC) | gs+1 | Same family as ALG-LRC-001 |
| MinIO | RS k+m + per-shard bitrot checksums (HighwayHash) | k (full stripe) | URL 404'd at research time — negative result recorded; checksum+RS parity with no local repair |
| Nextcloud / Seafile | replication / no EC | n/a | Ferro's own class — no local repair at all |
| **Ferro before this YP** | RS(4,2), no scrub | 4 + no proactive repair | behind all of the above |

**Verdict:** the *layered-code* idea is not novel (Azure/HDFS/Ceph); what is Ferro-specific is pinning the **prefer-local decode cascade + checksum-gated scrub** as proved, test-vector-backed contract at DIY-NAS scale, where no incumbent in the self-hosted class ships EC at all.

## YP-7: Test Vector Specification

Encode layout (k=4,l=2,r=2) → 8 shards, indices/parity flags correct; direct roundtrip reports `Direct`; single data loss → `Local` path; 2 losses same group → `Global`; data+its-local-parity loss → `Global`; 3 data losses → FAILED; reconstruct via local group (8,2,2); global parity reconstruct; checksum verification; verify flags corruption; scrub repairs corrupt shard to byte-identical content; scrub repairs missing shard; scrub beyond budget leaves holes and reports empty; uneven groups rejected.

## YP-8: Domain Constraints

- DC-EC-001: decode MUST prefer the local path whenever its precondition holds (repair I/O proportional to failure).
- DC-EC-002: decode MUST fail closed when no path's precondition holds — never deliver partial data.
- DC-EC-003: the scrubber MUST NOT rewrite shards that pass checksum verification.
- DC-EC-004: `data_shards % groups == 0` MUST be enforced at construction (fail-closed, panic on misconfiguration).

## YP-9: Bibliography

1. Huang, C., Simitci, H., Xu, Y., Ogus, A., Calder, B., Gopalan, P., Li, J., Yekhanin, S. "Erasure Coding in Windows Azure Storage," USENIX ATC 2012 (Best Paper). Publication page fetched 2026-10-14. [VERIFIED]
2. Ceph docs, "Erasure code profiles" (reef): plugin list incl. LRC/SHEC/CLAY. Fetched 2026-10-14. [VERIFIED]
3. MinIO erasure-coding product page — `min.io/product/erasure-coding` returned 404 (2026-10-14). RS + bitrot checksums claim recorded as common knowledge, uncited. [NEGATIVE RESULT]
4. Xia, M. et al. "A Tale of Two Erasure Codes in HDFS" (FAST'15) — XOR local groups + RS global. [UNCERTAIN — not fetched this session]
