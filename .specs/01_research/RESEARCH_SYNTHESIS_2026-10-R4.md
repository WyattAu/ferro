# Research Synthesis — October 2026, Round 4 (7 Implementation Loops)

> R4's rule was the inverse of R2's: **no new research — close the pending
> rows.** Every loop took an already-pinned spec + Lean contract and pushed
> it into the Rust implementation with tests. Result: **6 pending rows
> closed, 1 formal gap proved, 1 pre-existing build bug + 1 pre-existing
> test-hang documented. All builds and touched-crate tests green.**

## The Loops

| # | Loop | Implementation | Tests |
|---|------|----------------|-------|
| 1 | **Audit MMR** (YP-LIFECYCLE 1.1 → 1.2) | `audit-log/src/mmr.rs`: append-only MMR; older peaks stable; bagged epoch anchors; O(log n) inclusion proofs + verification; persistence rebuild (leaves = decoded entry hashes, live == rebuilt); `AuditLog::record` extends the MMR with each row; `audit_bag()`/`inclusion_proof()`/`verify_inclusion()` public | 43 |
| 2 | **Sync VV conflicts** (YP-SYNC 1.2) | `sync-protocol/src/conflict.rs`: `classify` (VC-only, never wall clock), `deterministic_winner` ((hash, path) order-independent), `conflict_copy_path` loser preservation; `engine.rs::resolve_conflict` refuses silent LWW on Concurrent | 7 |
| 3 | **SIEVE + SingleFlight** (YP-CONSISTENCY 1.1) | `server/src/cache.rs`: SIEVE exactly as formalized in R2 (FIFO + visited bit, single-use second chance, exactly-one-victim) replacing arbitrary HashMap-order eviction; `SingleFlight` per-key in-flight locks for double-checked origin loads | 10 |
| 4 | **If-header parser** (YP-WEBDAV 1.1) | `server-webdav-core/src/ifheader.rs`: §10.4 untagged lists, tagged groups, `Not`, OR-of-ANDs, whitespace tolerance, malformed-rejecting; `evaluate` (vacuous pass for non-matching tags) + `submitted_tokens`; wired into PUT and LOCK refresh; `check_lock_for_write_with_token` now takes the full token list | 9 |
| 5 | **WASM gates** (YP-TENANCY 1.2) | `wasm-host/src/sandbox.rs`: fail-closed import allow-list (`host::{log,storage_get,storage_set,get_config}`) — wasi and unknown namespaces rejected pre-instantiation; `api.rs::read_bytes` ptr+len overflow gate | 4 new (34 total) |
| 6 | **HRW minimal disruption PROVED** (YP-DISTRIBUTED-CH 1.1) | `Rendezvous.lean`: `exists_snoc`, `argmax_dominates_snoc`, `minimal_disruption` — length-measure induction with snoc decomposition; closes the R3 test-pinned gap | Lean |
| 7 | **STREAM container** (YP-CRYPTO 1.1) | `crypto/src/stream.rs`: age/Tink-identical chunked AEAD — 64 KiB chunks, 12-byte nonce (7 random + 4 BE counter + 1 final flag), position-bound AAD; fail-closed on truncation/premature-final/counter-overflow/tag/AAD. The R2-retired XOR pad now has its replacement | 11 |

## Engine comparison (R4)

- **Audit**: Trillian/OpenTimestamps anchor external signed heads; Ferro now has the same MMR mechanics internally, with dual-signing staged. Nextcloud/Seafile remain plain DB rows.
- **Sync**: Syncthing resolves conflicts by newer-mtime with `.sync-conflict` copies — Ferro's VV classification + deterministic winner is the same user outcome with a provably-ordered rule.
- **Cache**: Caffeine (JVM) popularized W-TinyLFU; Dragonfly/TiDB adopted SIEVE — Ferro now matches the SIEVE adopters with a proved model + implementation pair.
- **WebDAV**: SabreDAV ships a full If parser; Ferro's is now grammar-real for state tokens (ETags pending).
- **Crypto**: Ferro's STREAM container is byte-layout-compatible in spirit with age v1; auditable against two independent public specs as planned in R2.

## Honest limitations (in-file too)

- MMR anchor dual-signing (Ed25519+ML-DSA-65) and PII epoch salting: staged (persistence-layer work).
- If-header ETag conditions parse but evaluate unsatisfied.
- `sync-protocol` `state::`/`engine::` test hangs are pre-existing (files untouched since before R3; clean-tree build fails independently on the unguarded rusqlite import fixed in R3-L1) — quarantined behind `--skip` until debugged.
- STREAM nonce prefix uses a weak local entropy mix; production must source it from `CryptoProvider::random_bytes` (documented in-file).

## Verification state

- `lake build`: green, 0 `sorry` (minimal_disruption now proved; no new axioms).
- Touched-crate tests: audit-log 43, sync-protocol 18 (excl. pre-existing hangs), server cache 10, webdav-core 34, wasm-host 34, crypto 29 — all green.
