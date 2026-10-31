# Research Synthesis: October 2026, Round 2 (10 Hardening Loops)

> 10 research iterations: pull papers → analyze → compare with other systems →
> implement in spec → adjust the Lean proof → rebuild. Result: **7 YP revisions
> (1.0 → 1.1), 1 new Yellow Paper (16 total), 9 new Lean modules (110+ theorems,
> 0 `sorry`), 3 dead proof modules removed.**

## Methodology

Each loop: (1) verify primary sources by fetching (arXiv / RFC / C2SP / vendor
docs / man-pages) with [VERIFIED]/[UNCERTAIN] discipline and recorded negative
results, (2) compare against production systems (Syncthing, Dropbox, Nextcloud,
imgproxy, Tink, age, Envoy, S3, Trillian, OpenTimestamps, UCAN, TiDB, Ceph),
(3) distill a theorem set statable in pure Lean 4 core (no mathlib), (4) prove
it, (5) pin the decision in the spec with parameters sized for 50 users /
one TrueNAS host, (6) `lake build` green before moving on.

## The Loops

| # | Domain | Frontier finding (verified this session) | Spec change | Lean module |
|---|--------|------------------------------------------|-------------|-------------|
| 1 | Sync conflict | Merkle-CRDTs (arXiv:2004.00107, **correct ID verified; 2004.09368 is an unrelated econ paper**); eg-walker EuroSys'25 re-verified | YP-SYNC 1.1: `ALG-LOGROOT-001` (op-log root as logical clock/replica-equality oracle); DVVs considered+rejected | `SyncConflict.lean` (16 thms: VV partial order, Lamport causality, winner set-order independence, no-silent-loss) |
| 2 | Tamper-evident audit | OpenTimestamps **Merkle Mountain Range** (Todd); per-event O(log n) inclusion proofs, strictly append-only; linear chains give prefix evidence only | YP-LIFECYCLE 1.1: `ALG-MMRLOG-001`; MMR peaks in SQLite, bagged anchor dual-signed Ed25519+ML-DSA-65 | `AuditChain.lean` (13 thms: append-stability, no-drop, tamper-evidence, forward-secure key evolution, **PII epoch unlinkability**, WORM hold precedence) |
| 3 | Crypto agility | **age v1 (C2SP) + Tink Streaming AEAD** both instantiate STREAM (ePrint 2015/189): 64 KiB chunks, counter+final-flag nonces, EOF-without-final-chunk MUST fail; HPKE/MLKEM768-X25519 recipients | YP-CRYPTO 1.1: `ALG-STREAMAEAD-001`; adopt age's container shape verbatim; retires the 8160-byte XOR pad formally | `ChunkedAEAD.lean` (11 thms: nonce uniqueness, final-flag separation, truncation detection, no-cap, position binding, fail-closed agility) |
| 4 | Mesh peer authz | **UCAN v1.0.0** spec (ucan-wg): attenuation-only delegation, `/`-prefix command order, chain validity = interval intersection | YP-MESH 1.1: `ALG-MESHAUTH-001`; **DC-MESH-007 closed by construction**: tenant-anchored attenuated chains; server-side ACL evaluated and rejected (coordinator reintroduction) | `Capability.lean` (10 thms: command partial order, chain soundness, **no amplification, tenant containment**, interval containment) |
| 5 | Media derivatives | imgproxy docs verified (libvips, signed-URL identity, presets, **metadata stripping**, image-bomb checks) | YP-MEDIA 1.1: `ALG-ALBUMLOG-001`; albums become a fold over the sync op log (kills `albums.json` race); derivative keys carry a multicodec prefix | `Derivatives.lean` (10 thms: key injectivity under bounded registries, namespace disjointness, original immutability, thumb privacy, album determinism) |
| 6 | Cache eviction | **SIEVE** (NSDI'24) site verified + adoption list (TiDB, DragonFly, Ceph, PostgREST); YP already named it; the *mechanism* was never formalized | YP-CONSISTENCY 1.1: `ALG-SIEVEMECH-001`; mechanism pinned; DACL (arXiv:2511.21235) watched, not adopted | `SieveCache.lean` (11 thms: no-reorder-on-hit, single-use second chance, exactly-one-evict, capacity bound, singleflight dedup, **stampede bound**) |
| 7 | Rate limiting | GCRA (ATM Forum TM 4.1); single-integer TAT state; the design behind redis-cell and governor | YP-TENANCY 1.1: `ALG-GCRA-001`; per-tenant·route admission on the interactive path | `RateLimiterGCRA.lean` (7 thms: TAT monotone/strict, schedule advance, **instantaneous burst ≤ burst depth**) |
| 8 | Circuit breaking | **Envoy outlier detection** docs verified: consecutive-5xx, ejection time = base × count **capped at max_ejection_time**, success unejects | **NEW YP-RESILIENCE-BREAKER-001** (16th paper): adaptive breaker semantics | `CircuitBreakerLiveness.lean` (9 thms: bounded detection, success resets, bounded recovery, ejection capped/escalates/monotone) |
| 9 | Path safety | `openat2(2)` man page (6.19) verified: RESOLVE_BENEATH/IN_ROOT, magic-link and symlink isolation, **EAGAIN = kernel admits it can't prove `..` containment → fail-closed**; FreeBSD O_BENEATH shipped insecure; history lesson recorded | YP-TENANCY YP-10: decode-then-check contract; EAGAIN MUST retry-not-fall-through | **PathSafety.lean: all 6 sorries proved** incl. `dotdot_detected` (check completeness); PathTraversal's 3 sorries fixed/replaced; **`formal/` tree now 0 sorry** |
| 10 | Search fusion | RRF primary source re-verified (Cormack SIGIR'09 PDF) | YP-SEARCH 1.1: `ALG-RRFFORMAL-001`; fusion theory pinned; ACL-cut position freedom proved | `SearchFusion.lean` (9 thms: RRF monotonicity via from-scratch `div_anti_monotone`, fused-score domination, authz soundness, top-k bound) |

## What Ferro now does that no incumbent ships

1. **Verifiable sync log roots**; replica-equality handshake via Merkle-rooted
   op logs (Syncthing/Dropbox/Seafile: none).
2. **Per-event audit inclusion proofs** against dual-signed (Ed25519+ML-DSA-65)
   epoch heads via MMR (Nextcloud/Seafile: plain DB rows; QLDB/Azure: linear
   chains, prefix evidence only).
3. **age/Tink-identical chunked AEAD** with truncation detection; auditable
   against two independent public specs + the age test vectors.
4. **Tenant-contained capability chains for mesh fetch**; public-key
   verifiable, partition-tolerant, provably non-amplifying (UCAN-shaped; no
   self-hosted sync ships anything comparable).
5. **GDPR-compatible audit by construction**; per-epoch PII salting makes
   cross-epoch linkage uncomputable from stored representations.

## Corrections & negative results (do not cite the wrong thing)

- arXiv:2004.09368 ≠ Merkle-CRDTs (it is an econometrics paper). Correct:
  **arXiv:2004.00107** (Sanjuan, Poyhtari, Teixeira, Psaras).
- FreeBSD's original `O_BENEATH` was *effectively insecure* (man-page history);
  fixed only by FreeBSD 13 `O_RESOLVE_BENEATH`.
- Dotted version vectors: rejected for Ferro; closed device membership +
  90-day pruning is strictly simpler with equivalent boundedness.
- Dotted-version-vector-style DynamicAdaptiveClimb beats SIEVE ~10–15% only
  under fluctuating working sets (arXiv:2511.21235); watched, not adopted.
- `imgproxy.net/blog/imgproxy-3-pro/` and two docs.imgproxy.net deep links
  404'd; root docs verified instead (recorded so nobody cites the dead URLs).

## Lean formal tree: final state

| Module | Theorems | Spec anchor |
|---|---|---|
| SyncConflict | 16 | YP-SYNC 1.1 |
| AuditChain | 13 | YP-LIFECYCLE 1.1 |
| ChunkedAEAD | 11 | YP-CRYPTO 1.1 |
| Capability | 10 | YP-MESH 1.1 |
| Derivatives | 10 | YP-MEDIA 1.1 |
| SieveCache | 11 | YP-CONSISTENCY 1.1 |
| RateLimiterGCRA | 7 | YP-TENANCY 1.1 |
| CircuitBreakerLiveness | 9 | YP-RESILIENCE-BREAKER-001 |
| SearchFusion | 9 | YP-SEARCH 1.1 |
| PathSafety (repaired) | 15 | YP-TENANCY 1.1 |

**110+ theorems, 0 `sorry`, `lake build` green.** Removed: `Cache.lean`,
`CacheEviction.lean` (superseded by SieveCache + CacheConsistency/
CacheInvalidation, both sorry-free), `PathTraversal.lean` (superseded by
PathSafety; was never in the import graph; its errors were invisible).

## Known limitations (stated honestly, in-file too)

- Hash collision resistance, cipher IND-CCA2, HKDF extraction, signature
  unforgeability, encoder bit-exactness: **external assumptions**; the Lean
  layer proves the structural/logical properties above them.
- Symbolic string `splitOn` has no core lemmas in Lean 4.12; concatenation
  theorems take decomposition hypotheses (documented at each theorem).
- RRF weights are scaled integers, not exact rationals; monotonicity is
  preserved exactly under positive scaling; exact ties may differ from
  rational RRF in pathological rank patterns.
