# YP-DEDUP-CDC-001: Content-Defined Chunking, Hash Agility & Deduplication Scope

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-DEDUP-CDC-001                           |
| Domain       | Storage Systems / Deduplication            |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/desktop/src/sync/block.rs`, `crates/server-sync-handlers/src/lib.rs` |
| Extends      | YP-STORAGE-CAS-001                         |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| Buzhash CDC, 48-byte cyclic-polynomial window, table seeded `0x9E3779B97F4A7C15` | `block.rs:78-108` |
| Params `target=65536, min=4096, max=1_048_576` (client AND server, hardcoded independently) | `block.rs:44`, `server-sync-handlers/src/lib.rs:142` |
| `compute_mask(t)` rounds target up to power of two | `block.rs:170-173` |
| Buzhash window never primed (`window:[0;48]`, `hash:0` at chunk start) | `block.rs:102-108, 134-155` |
| Manifest/diff hash-only; `HashSet` collapses duplicates | `block.rs:176-186` |
| SHA-256 everywhere; `max/avg = 15.1×`; ~186,700 blocks for 13 GB; manifest ≈ 12.1 MB | derived |

**P0 CORRECTNESS BUG (must fix regardless of algorithm choice):** hash-only manifests are *unsound* under variable-length CDC. A peer that has never seen the remote revision cannot reconstruct the file — chunk *lengths* are not recoverable from hashes. Syncthing avoids this only because its blocks are fixed-size (length = function of file size). Ferro's CDC is variable-length. `THM-DEDUP-SOUNDNESS` below is violated today.

**Secondary defects:** (1) power-of-two mask rounding silently shifts E[chunk] for non-power-of-two targets; (2) unprimed window injects a structured zero-byte bias at chunk-relative offsets 4096..4144; (3) max-size branch can emit a 1-byte orphan chunk; (4) parameter drift risk between the two independent 65536/4096/1048576 literals.

---

## YP-2: Executive Summary

### Problem Statement

Ferro chunks with Buzhash CDC at an effective average of 68 KiB, producing ~186,700 blocks and ~12 MB manifests for 13 GB of user data, with a reconstruction-unsound hash-only manifest format. Deduplication is globally scoped, making the CAS and the mesh `query_block_holders` a textbook cross-tenant existence oracle (Harnik et al. 2010; Dropbox has publicly conceded this leak in production).

### Decisions

1. **Chunker:** migrate Buzhash → Gear/FastCDC Level-2 normalisation. Parameters: `< 256 KiB` files stored whole (no CDC); `≥ 256 KiB` chunked with `min=256 KiB, avg=1 MiB, max=8 MiB`. Consequences: blocks 186,700 → ~13,000 (7.1×); manifests 12.1 MB → ~0.85 MB; `max/avg` 15.1× → 8×; no orphan chunks.
2. **Hash:** BLAKE3-256 for new data behind a self-describing multihash-style 2-byte prefix; SHA-256 retained as legacy + FIPS fallback. Dual-write for one release, background backfill.
3. **Manifests:** carry `(index, hash, length)` triples + a versioned `ChunkingProfile`; revision id = BLAKE3 root over the canonical serialisation (Merkle root; O(1) content equality).
4. **Dedup scope:** tenant-scoped by default; global dedup only as admin-flagged opt-in with a written existence-oracle warning. Mesh block-holder answers are constant-shape (never a per-hash boolean).

### Guarantees

- **Reconstruction soundness:** manifest uniquely determines file bytes.
- **Boundary determinism:** chunk boundaries are a pure function of `(bytes, profile)` — independent of buffering, threading, read size.
- **Edit locality:** an edit of `δ` bytes perturbs at most `⌈δ/W⌉ + B` boundaries (`B = max/avg × 2`).
- **Non-interference:** cross-tenant dedup queries leak nothing (statistical-distance statement).

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| CDC | Content-defined chunking: cut points derived from content, not offset. |
| Gear hash | `h = (h << 1) + TABLE[b]` rolling hash; effective window ~8 bytes; 256-entry u64 table. |
| FastCDC normalisation | Mask widening near chunk start to recover dedup ratio lost to cut-point skipping: `bits(i) = log2(avg) + max(0, log2(avg) − log2(i))`. |
| Cut-point skipping | No boundary test below `min` — the throughput optimisation; costs ~15% dedup ratio at Ferro's old min/avg ratio (6.25%); correct ratio is ≥ 25% at the new parameters. |
| ChunkingProfile | Versioned `(min, avg, max, norm_level, algorithm_id)` persisted per revision. |
| BlockID | `0x01 ‖ 0xB3 ‖ <32 B BLAKE3-256>` (new) or `0x12 ‖ 0x20 ‖ <32 B SHA-256>` (legacy). |
| Tenant-scoped dedup | CAS lookup key `blocks/<tenant_id>/<hash>`; physical blob sharing optional, logical namespace per tenant. |
| Existence oracle | Cross-user ability to infer whether another principal holds content for a hash (Harnik 2010). |
| Plaintext-existence hiding | Formal game (Boyd et al. ePrint 2018/748) bounding adversary advantage in dedup systems. |

---

## YP-4: Theoretical Foundation

### DEF-CDC-001: Content-Defined Chunking

A CDC function `C_p: Bytes → List<Block>` with profile `p = (min, avg, max, L, alg)` maps a byte string to a list of blocks where boundaries are determined by a rolling-hash predicate over a window `W`. Desirable properties: (i) *boundary determinism* — `C_p` is a pure function of content and profile; (ii) *edit locality* — small edits perturb few boundaries; (iii) *size bounds* — `min ≤ len_i ≤ max` for all but at most the final chunk.

### THM-DEDUP-SOUNDNESS ⚠ VIOLATED TODAY

Let `C: Bytes → List<Block>` be the chunker and a manifest be `(profile, [(i, h_i, len_i)]_{i<k})`. Then:
(a) *Reconstruction:* `concat_{i<k} store[h_i]`, reassembled with the declared lengths, equals the original file.
(b) *Injectivity:* `B₁ = B₂ ⟺ content(B₁) = content(B₂)` — the manifest uniquely determines the bytes.

**Ferro's current hash-only manifest violates (a):** `len_i` is unrecoverable by a peer that has not seen the revision. Fix is mandatory and independent of chunker choice.

### THM-CHUNK-BOUNDARY-INVARIANT

For FastCDC with profile `(min, avg, max)` and normalisation level `L`:
(a) `∀ i < n−1 : min ≤ len_i ≤ max`;
(b) at most one chunk has `len < min`, and if it exists it is the last;
(c) the boundary predicate at offset `i` is a pure function of `(bytes[i−W..i], min, avg, max, L)` — independent of prior boundaries and total length;
(d) an edit of `δ` bytes at offset `p` perturbs at most `⌈δ/W⌉ + B` boundaries, `B = max/avg × 2`.

⚠ Violated today: the zero-initialised Buzhash window makes the boundary predicate depend on `min_size` in an unspecified way, and the max-size branch can emit a 1-byte orphan chunk (violating (a)).

### THM-CAS-INTEGRITY

`∀` block `b` stored at key `k = H(b)`: `GET(k)` returns `b'` with `H(b') = k`. Proof obligation: collision resistance of `H` at 128 bits (BLAKE3-256: 128-bit security target, subtree-free ⇒ no length extension; SHA-256 fallback). Requires per-block verification on receipt (Syncthing discipline).

### THM-DEDUP-NONINTERFERENCE

For tenants `T ≠ T'` and any hash `h`, the distribution of (response body, latency, connection behaviour) of the CAS and the mesh block-holder query to `T`'s request for `h` is statistically independent of whether `T'` stores `h`. Satisfied by tenant-scoped namespacing + constant-size always-answer. **FALSE for Ferro's current global namespace** and for the mesh `register(full_inventory)` call.

### THM-CHUNKING-DETERMINISM

`∀ f, f′`: `C_p(f) = C_p(f′)` whenever `f, f′` agree on all bytes any boundary decision can depend on. Equivalently `C_p` is a pure function of `(data, profile)` with no dependence on read chunk size, buffering, thread count, or hash-map iteration order. ⚠ Risk today: `par_iter().collect()` ordering and the two independent parameter literals (`block.rs:44` vs `server-sync-handlers/lib.rs:142`) — parameter drift here is silent and corrupting.

---

## YP-5: Algorithm Specification

### ALG-FastCDC-001: Gear/FastCDC Level-2 Chunking

```
if len(f) < 256 KiB: return [(0, H(f), len(f))]            # small-file fast path

h ← 0; start ← 0
for i in 0..len(f):
    h ← (h << 1) + TABLE[f[i]]                             # Gear: 1 shift + 1 add + 1 lookup
    if i − start < min (2^18): continue                    # cut-point skipping
    bits ← 20 + max(0, 20 − log2(i − start))               # normalisation level 2
    if (h & ((1 << bits) − 1)) == 0: cut at i
    if i − start ≥ max (2^23): hard cut at i               # never emits orphan sub-min chunk
```

Resulting cut expectations: `[2¹⁸,2¹⁹)` P=2⁻²² (E[wait] 4 MiB); `[2¹⁹,2²⁰)` P=2⁻²¹; `[2²⁰,∞)` P=2⁻²⁰ (E[wait] 1 MiB); hard stop 8 MiB.

### ALG-MANIFEST-001: Sound Manifest Format

```
Manifest = {
  profile : ChunkingProfile{alg_id, min, avg, max, norm_level},   # versioned, persisted
  blocks  : [(index: u32, hash: BlockID, length: u64)],           # ordered, lengths included
  root    : BLAKE3(canonical_serialise(Manifest w/o root))         # Merkle root = revision id
}
diff(A, B) := {h : h ∈ A.blocks, h ∉ B.blocks} ∪ {h : h ∈ B.blocks, h ∉ A.blocks}
              with multiplicities preserved for reconstruction
```

### ALG-HASHID-001: Hash Agility (multihash discipline)

```
BlockID = 0x01 ‖ 0xB3 ‖ <32-byte BLAKE3-256>     # new writes
BlockID = 0x12 ‖ 0x20 ‖ <32-byte SHA-256>        # legacy, permanently resolvable
Physical CAS path: blocks/<first-2-hex>/<full-hex>   # uniform across algorithms
Migration: dual-write (both hashes, one authoritative) for one release → background backfill
FIPS mode: SHA-256 path only; BLAKE3 rejected at admission when FipsMode::Strict
```

### ALG-DEDUPSCOPE-001: Tenant-Scoped Dedup with Non-Observability

```
lookup key: blocks/<tenant_id>/<hash>            # logical namespace per tenant
mesh block-holder query: responder answers with constant-size constant-time bitvector
  over a peer-chosen random subset; NEVER a per-hash boolean
registry: stores (instance_id → {addrs, expires_at, last_seen}) ONLY — never hashes
global dedup: admin opt-in only, SECURITY.md documents the D28 existence oracle
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_sync_stack.toml` — sections `dedup_*` and `cdc_*`.

Critical vectors: NIST-class reconstruction round-trips for the manifest format; E[chunk] ∈ [0.9, 1.1]·avg over 256 MiB fixtures (chunk-size-variance discipline per the 2024 CDC survey); edit-locality bound; orphan-chunk absence; parameter-consistency (client/server literals must match — one shared constant); BLAKE3/SHA-256 prefix disambiguation; FIPS-mode rejection.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_sync_stack.toml`.

Summary: manifests MUST carry lengths (`DC-DEDUP-001`); client and server chunk parameters MUST come from a single shared constant (`DC-DEDUP-002`); rolling window MUST be explicitly reset/primed and the semantics stated (`DC-DEDUP-003`); E[chunk] property test mandatory in CI (`DC-DEDUP-004`); cross-tenant block-holder answers constant-shape (`DC-DEDUP-005`); BLAKE3 blocked in FIPS mode except where explicitly approved (`DC-DEDUP-006`).

---

## YP-8: Bibliography

1. Xia, W., Zhou, Y., Jiang, H., Feng, D., Hua, Y., Hu, Y., Liu, Q., Zhang, Y. "FastCDC: A Fast and Efficient Content-Defined Chunking Approach for Data Deduplication." *USENIX ATC '16*, pp. 101–114. [VERIFIED]
2. Xia, W., Zou, X., Jiang, H., Zhou, Y., Liu, C., Feng, D., Hua, Y., Hu, Y., Zhang, Y. "The Design of Fast Content-Defined Chunking for Data Deduplication Based Storage Systems." *IEEE TPDS* 31(9), 2020. [VERIFIED]
3. FastCDC normalisation table (avg 8 KB, min 2 KB, Level 2): `[2K,4K)` 15 bits; `[4K,8K)` 14 bits; `[8K,64K)` 13 bits; generalised `bits(i) = log2(avg) + max(0, log2(avg) − log2(i))`. [VERIFIED — paper/slides]
4. Gear rolling hash: `fp = (fp << 1) + G(b)`; Ddelta, *PERFORMANCE '14* (Xia et al.). [VERIFIED]
5. Rabin-based CDC / LBFS, *SOSP 2001* (`pdos.csail.mit.edu/papers/lbfs:sosp01/lbfs.pdf`). [VERIFIED]
6. restic/chunker parameters: window 64, min 512 KiB, max 8 MiB, poly `0x3DA3358B4DC173` (degree 53). [VERIFIED]
7. Rabin, M. "Fingerprinting by Random Polynomials" (1981); Broder, A. "Some Applications of Rabin's Fingerprinting Method" (1993). [VERIFIED]
8. HF5/Xet Gearhash: target 64 KiB, min 8 KiB, max 128 KiB, mask `0xFFFF000000000000`. [VERIFIED]
9. Gregoriadis, M., Balduf, M., Scheuermann, W., Pouwelse, J. "A Thorough Investigation of Content-Defined Chunking Algorithms for Data Deduplication." arXiv:`2409.06066` (v3, Sept 2024). *The neutral 2024 comparison; measures chunk-size variance as first-class.* [VERIFIED]
10. "The Chonkers Algorithm: Content-Defined Chunking with Provable Strict Guarantees on Size and Locality." arXiv:`2509.11121` (Sept 2025). First theorem-bearing CDC result; watch, do not depend. [VERIFIED]
11. Udayashankar, Baba, Al-Kiswany. "SeqCDC: Hashless Content-Defined Chunking for Data Deduplication." *ACM/IFIP MIDDLEWARE '24*, pp. 292–298. DOI `10.1145/3652892.3700766`. 1.5×–3.1× throughput at similar savings. [VERIFIED]
12. "Vectorized Sequence-Based Chunking for Data Deduplication." arXiv:`2505.21194` (2025). [VERIFIED]
13. Udayashankar, Baba, Al-Kiswany. "VectorCDC: Accelerating Data Deduplication with SSE/AVX Instructions." *USENIX FAST '25*, pp. 513–522. SS-Gear 3.3 GB/s (3.13×); vectorization ceiling ~3× documented. [VERIFIED]
14. "Taking a Bigger Byte: Wide Hashless Chunking for Data Deduplication" (WideCDC), *USENIX FAST '27*. Up to 2.95× better hashless dedup ratio at 2.04× throughput. [VERIFIED]
15. **Negative result:** "GearCDC (2023)" — no paper by this name exists; Gear is Ddelta/PERFORMANCE'14 (D4). Do not cite. [VERIFIED ABSENCE]
16. O'Connor, L., Aumasson, J.-P., Neves, S., Wilcox-O'Hearn, Z. "BLAKE3: One Function, Fast Everywhere." Spec, announced Real World Crypto, 2020-01-09. No arXiv/DOI — cite by title + repo. [VERIFIED]
17. "The BLAKE3 Hashing Framework." IETF draft `draft-aumasson-blake3` — expired -00, never an RFC. 128-bit security target. [VERIFIED]
18. BLAKE3 throughput: ~13.2 GB/s vs SHA-256 ~2.4 GB/s single-core (Zen 5, AVX-512) ≈ 5.5×; ~80 GB/s at 16 threads (repo figure). ARM caveat: only 1.3× SHA-256 on ARM1176-class. [VERIFIED — vendor/blog benchmarks]
19. Aumasson, J.-P., Neves, S., Wilcox-O'Hearn, Z., Winnerlein, C. "BLAKE2: simpler, smaller, fast as MD5." ePrint `2013/322`; RFC `7693`. [VERIFIED]
20. Harnik, D., Pinkas, B., Shulman-Peleg, A. "Side Channels in Cloud Services: Deduplication in Cloud Storage." *IEEE Security & Privacy*, 2010. The founding existence-oracle result. [VERIFIED]
21. Armknecht, F., Boyd, C., Davies, W., Gjøsteen, K., Toorani, M. "Side Channels in Deduplication: Trade-offs between Leakage and Efficiency." *AsiaCCS 2017*. DOI `10.1145/3052973.3053019`; ePrint `2016/977`. Formal games + provable bound. [VERIFIED]
22. Boyd, C., Davies, G., Gjøsteen, K., Raddum, H., Toorani, M. "Definitions for Plaintext-Existence Hiding in Cloud Storage." ePrint `2018/748`. [VERIFIED]
23. Bacs, A., Musaev, A., Razavi, B., Giuffrida, C., Bos, H. "DUPEFS: Leaking Data Over the Network With Filesystem Deduplication Side Channels." *USENIX FAST '22*. Byte-granularity network amplification. [VERIFIED]
24. Xu, J., Chang, E.-C., Zhou, J. "Weak leakage-resilient client-side deduplication of encrypted data in cloud storage." *ASIACCS '13*, pp. 195–206. DOI `10.1145/2484313.2484340`. [VERIFIED]
25. Tang, Y., Zhu, H., Fu, Z. "Comments on 'Privacy Aware Data Deduplication for Side Channel in Cloud Storage'." *IEEE TCC* 12(2):814–817, 2024. DOI `10.1109/TCC.2024.3376996` (ZEUS/ZEUS+ fall to random-chunk-generation attacks). [VERIFIED]
26. "Random Coding Responses for Resisting Side-Channel Attacks in Client-Side Deduplicated Cloud Storage" (RACORE). *IEEE TSC* 18(3):1697–1710, 2025. DOI `10.1109/TSC.2025.3568252`. Countermeasure SOTA; **not deployable at Ferro's scale — scope reduction is the correct move.** [VERIFIED]
27. Dropbox, "Inside LAN Sync" (2015): *"computers cannot pretend to be servers for namespaces that they do not control … they might be able to learn something by watching which block hashes you request."* — Dropbox publicly conceding the leak Ferro must design out. [VERIFIED]
28. Dropbox Magic Pocket: fixed 4 MB blocks, SHA-256, immutable buckets, EC-after-ageing; 2026 update documents <5%-live-data fragmentation worst cases. [VERIFIED]
29. Syncthing: fixed blocks 128 KiB–16 MiB from file size; `(offset, size, hash)` triples; every received block re-hashed and discarded on mismatch. [VERIFIED]
30. Seafile: LBFS-derived Rabin CDC, avg block ≈ 1 MB, cross-version dedup. [VERIFIED]
31. IPFS CID spec (`specs.ipfs.tech/cid`): CIDv1 = `<multibase><0x01><multicodec><multihash>`; self-describing, forward-compatible. Template for BlockID prefixes. [VERIFIED]
32. Dimakis et al. "Network Coding for Distributed Storage Systems." *IEEE TIT* 56(9):4539–4551, 2010. + "Reducing Data Fragmentation in Data Deduplication Systems via Partial Repetition and Coding." arXiv:`2411.01407` (2024) — align `crates/distributed/erasure.rs`; **Coded Dedup is an explicit non-goal** (destroys single-block serving for the mesh). [VERIFIED]
