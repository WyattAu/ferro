# Market Position — honest gap analysis vs. incumbent tools (2026-10-25)

Scored after R3–R5. Dimensions: [+] ahead, [=] at parity, [–] behind (R6 candidate).

## vs. Nextcloud / ownCloud (groupware DAV suites)
- [=] WebDAV Class 1/2/3, CalDAV/CardDAV, WOPI — parity on protocol surface; Ferro adds formal lock-safety proofs (Nextcloud: none).
- [+] Search security (ACL-filtered Tantivy + proved RRF), multi-tenant isolation (proved prefix containment) — Nextcloud relies on app-level checks without formal models.
- [–] Apps ecosystem (Talk, Office suites, mail), plugin marketplace maturity, LDAP group sync depth, mobile clients at parity with NC apps.

## vs. Syncthing (P2P sync)
- [=] VV-based conflict handling semantics; Ferro's deterministic winner + loser preservation matches Syncthing's `.sync-conflict` outcome with a proved rule.
- [+] Tamper-evident sync log roots (Merkle op-log) and replay-gated federation inbox — Syncthing has neither.
- [–] NAT traversal maturity (Syncthing: relay + discovery network, years of hardening), BEP protocol battle-testing, global discovery.

## vs. Immich (photo vault)
- [=] Content-addressed media storage, dedup; thumbnail/album derivative pipeline is spec'd (YP-MEDIA 1.1) but Immich's ML features (face/object recognition) are far ahead.
- [+] CAS GC with age cutoff + bit-rot verify now shipped (R5); Immich-compatible API is an open integration surface via aperture-vault.

## vs. MinIO (object storage)
- [=] Erasure coding with layered local parity (R5); S3/GCS/Azure backends; presigned URLs.
- [+] Per-shard SHA-256 + formal GC/verify contracts; WASM extension model (MinIO: none).
- [–] K8s operator maturity, multi-tenant IAM (MinIO policy engine), throughput at scale, replication SLAs.

## vs. Filebrowser / WebDAV servers (lightweight class)
- [+] Ahead on every dimension that requires proof: audit chain (MMR + signed anchors vs. plain rows), rate limiting (GCRA proved), plugin sandbox gates, streaming chunked AEAD.
- [–] Simplicity of single-file deploy (Ferro is a 79-crate workspace).

## Cross-cutting
- [+] Only tool in class with formal verification (Lean 4: 110+ theorems, 0 sorry, covering sync, locks, cache, rate limits, audit, crypto containers, placement, EC).
- [–] Mobile/desktop client maturity (Tauri early), e2ee per-file sharing links, multi-server clustering at Nextcloud scale, i18n/accessibility depth.
- [–] Known open items: MMR ML-DSA half of dual signing, If-header ETag conditions live, sync-protocol test hangs (investigation documented), litmus suite run.

## Verdict
Ferro surpasses incumbents on **verifiable correctness, auditability, and extension safety** — dimensions none of them address. It trails on **ecosystem breadth, client polish, and scale hardening**. R6 priorities: mobile/desktop depth, NAT traversal, clustering, litmus + ETag conditions.
