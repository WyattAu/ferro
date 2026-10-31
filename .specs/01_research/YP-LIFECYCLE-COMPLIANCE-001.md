# YP-LIFECYCLE-COMPLIANCE-001: WORM, Legal Hold, Crypto-Shredding & Tamper-Evident Audit

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-LIFECYCLE-COMPLIANCE-001                |
| Domain       | Compliance / Data Lifecycle                |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/server-compliance/src/{worm,retention}.rs`, `crates/audit-log/src/chain.rs`, `crates/server-fips`, `crates/server-versioning` |
| Substrate    | TrueNAS / ZFS (snapshots + holds are a real enforcement layer) |

---

## YP-1: Ground Truth (Current Implementation)

| Area | File | Current state |
|---|---|---|
| WORM | `worm.rs:206-218` | `is_worm_protected()` = **advisory path-prefix match in SQLite**; no retain-until, no legal hold, no immutability mechanism (write-path bugs can overwrite "WORM" files; policy rows deletable via `delete_policy`, `worm.rs:104-119`) |
| Retention | `retention.rs:14-23` | `RetentionPolicy{path_prefix, max_age_seconds (default 90 d), ...}`; **deletion-driven, cannot hold**; no precedence resolution |
| Audit log | `chain.rs:24-52` | linear SHA-256 hash chain (`previous_hash`). **No Merkle tree, no signed epochs, no external anchor**; anyone with DB write access can rewrite the entire history and recompute all hashes |
| Crypto-shredding | - | **impossible-by-construction today**: originals plaintext in CAS; versions plaintext copies; restic backups replicate plaintext. Art. 17 erasure can only delete rows, leaving copies in versions, backups, ZFS snapshots, derivative caches |
| Audit PII | `chain.rs:43-47` | entries include IP addresses / User-Agents; personal data (ERASEABLE under Art. 17) sitting in an append-only log |

---

## YP-2: Executive Summary

### Problem Statement

Ferro's compliance surface is advisory, not defensible. "Defensible" operationally means: (a) WORM backed by an enforcement mechanism, (b) legal hold that beats retention that beats user delete, (c) erasure that is *complete* across every storage class, (d) audit logs that are tamper-evident even against DB-write-capable attackers, with PII separated out.

### Decisions

**A. Make WORM real with the substrate (ZFS):**
- `worm.rs` gains `mode: {governance, compliance}` + `retain_until`. Per write into a covered prefix: (1) CAS write; (2) periodic ZFS snapshot of the dataset; (3) `zfs hold ferro-worm-<policy_id>` until `retain_until`; (4) scheduled job releases the hold only on expiry. Compliance ⇒ hold + server-side refusal of overwrite APIs; governance ⇒ logged admin bypass (S3 `BypassGovernanceRetention` analog).
- **Legal hold:** new `holds(object_set, reason, created_by, created_at)` table; deletion/retention jobs MUST refuse any path matching an active hold (Google Vault / Purview pattern; maps 1:1 onto Art. 17(3)(f)).

**B. Crypto-shredding (make Art. 17 erasure complete):**
- Encrypt every user-content CAS chunk under a **per-user KEK** (the `server-fips` DK layer): `ct = AES-256-GCM(HKDF(user_KEK, chunk_id), nonce, chunk)`; requires the YP-CRYPTO-PQ-001 header/AEAD fixes.
- Erasure = destroy user KEK **and all backups of it** (escrow included); tombstone the audit entry with `key_id`, not content. Residual is ciphertext-only (AES-256 ⇒ ~2¹²⁸ practical quantum security).
- **Enumerate copy sites exhaustively** and route through the KEK: CAS chunks, `data_dir/versions`, thumbnails/derivatives (YP-MEDIA-DERIV-001), key-material backups (excluded from backup paths), DB PII (pseudonymize).
- **Deletion certificate:** audit entry `ErasureCompleted{user_pseudonym, key_ids[], timestamp, sig}`; auditable Art. 15/17 evidence.
- Be explicit: ZFS COW means destroyed-key ciphertext persists in old snapshots; acceptable because it is ciphertext; the guarantee is **computational, not physical**.

**C. Tamper-evident audit (upgrade `chain.rs`):**
- Keep the linear chain inside an epoch; at epoch close (24 h or 10k entries), compute an **RFC 6962/9162-style Merkle tree head**; sign `{head_hash, tree_size, epoch_id}` with **Ed25519 + ML-DSA-65 hybrid** (YP-CRYPTO-PQ-001); publish/replicate heads (ZFS snapshot, witness host, or Rekor if available).
- **Schneier-Kelsey key evolution between epochs** (`SK_i = H(SK_{i+1})`; publish hash of `SK_i` at epoch close) → forward integrity even against later signing-key compromise.
- Verification becomes O(log n) inclusion proofs + O(1) per-epoch head check, instead of "recompute the whole linear hash".
- **PII separation:** entries store only {pseudonymous actor_id, action, resource_id, content_hash, timestamps}; IP/UA dropped after `T_PII ≤ 30 d`; the Art. 17-satisfying separation ("hash the content, keep the fact").

**D. Precedence engine:** `hold > WORM > retention > user delete`, with every override decision recorded as an audit entry carrying the legal basis (e.g. `Art17(3)(f)`); deletion governance reproducible from the log alone.

**Parameters (50 users):** epoch 10,000 entries or 24 h; ZFS snapshot cadence 1 h on WORM datasets; hold horizon = `retain_until` (min 7 y default, configurable); monthly restic snapshot verification.

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| WORM (compliance vs governance) | Immutable-retention modes; compliance = nobody shortens; governance = logged privileged bypass. |
| Legal hold | Indefinite preservation flag independent of retention; blocks all deletion. |
| Crypto-shredding / cryptographic erasure | Rendering data unrecoverable by destroying its key (NIST SP 800-88 sanitization category). |
| Deletion certificate | Signed statement "KEK k destroyed at t, covering object set S", anchored in the audit log. |
| Merkle epoch | Time/size-bounded audit segment closed by a signed tree head. |
| Forward integrity | Past entries unforgable even given current key material (Schneier-Kelsey key evolution). |
| Witness | External party holding published epoch heads, making rewrite collusion-visible. |
| Pseudonymization | Replacing direct identifiers with per-subject pseudonyms; GDPR Art. 4(5). |

---

## YP-4: Theoretical Foundation

### THM-AUDIT-TAMPER-EVIDENT

Let `L` be an epoch-closed audit log with per-epoch Merkle head `h_i`, signed `(S₁, S₂)` under Ed25519 and ML-DSA keys whose public keys are held by a verifier/witness *before* epoch `i` closes. Any modification, insertion, or deletion of an entry in epoch `i` is detected with probability 1 by any verifier holding `{h_j}`; collision resistance of SHA-256 reduces undetected rewrite to finding a Merkle second-preimage/collision.

### THM-AUDIT-FORWARD-INTEGRITY (Schneier-Kelsey)

If epoch keys evolve backwards `SK_i = H(SK_{i+1})` and only `H(SK_i)` is committed at epoch close, then an adversary compromising the current signing state at time `t` can forge entries only for epochs ≥ `t`; all epochs < `t` remain unforgable given full current key material.

### THM-CRYPTO-SHRED

Let `D_K` denote encryption of dataset `D` under key `K` (AES-256-GCM). Given (i) all copies of `D_k` (CAS chunks, versions, derivatives, key-material backups) are encrypted under user-scoped KEK `k`, and (ii) `k` is destroyed from all key stores including escrow and backups; then for any adversary `A` with resource bound `B`: `Pr[A recovers any plaintext of D_k] ≤ Adv_AES-256(B)`. Erasure is complete up to AES-256 security.

### THM-ERASURE-COMPLETENESS

Erasure(w) is *complete* iff every storage class `s ∈ {CAS, versions, derivatives, DB, backups, snapshots}` satisfies `Enc(s, KEK(w))`. System invariant: **every new storage class must register an encryption adapter**; new classes default to unencrypted and fail CI, keeping completeness inductively maintainable.

### THM-WORM-HOLD-PRECEDENCE

For any delete request `d` against object `o`: `exec(d) ⇔ ¬hold(o) ∧ ¬WORM_active(o) ∧ (retention_expired(o) ∨ basis(d) ∈ {Art17(1), admin})`, and every evaluated predicate is itself an audit entry; the deletion decision is reproducible from the log alone (non-repudiable deletion governance).

### THM-GDPR-AUDIT-COMPATIBILITY

If audit entries store only {pseudonymous actor_id, action, resource_id, content_hash, timestamps} and PII-bearing fields (ip, UA) are dropped at age `T_PII ≤ 30 d`, then the log contains no erasable personal data subject to Art. 17(1), while 17(3)(f) permits indefinite retention of the remainder.

---

## YP-5: Algorithm Specification

### ALG-WORMHOLD-001: ZFS-Backed Retention

```
policy write into covered prefix →
  CAS put; snapshot(dataset) every 1h; zfs hold ferro-worm-<policy_id> until retain_until
release job: expire hold only when now ≥ retain_until
compliance mode: overwrite/shorten APIs refused server-side
governance mode: admin bypass allowed, ALWAYS audited with reason
holds table checked before every delete/retention action
precedence: hold > WORM > retention > user-delete; branch recorded with legal basis
```

### ALG-EPOCHLOG-001: Signed Merkle Audit Epochs

```
append: linear hash-chain within epoch (cheap, incremental)
epoch close (10k entries | 24 h):
  head := RFC6962-MTH(entries); sig := (Ed25519(head), ML-DSA-65(head))
  publish head to witness store (ZFS snapshot | second host | Rekor)
  publish H(SK_epoch) ; SK_next := H(SK_current)   # forward integrity
verify(entry): inclusion proof vs published head; verify BOTH signatures
PII: ip/UA columns purged at T_PII = 30 d; actor_id pseudonymized at write
```

### ALG-ERASE-001: Crypto-Shredding Workflow

```
Art.17 request(w):
  verify no hold(w) ∧ no WORM(w)        # else refuse with legal basis (17(3)(f))
  destroy KEK(w) in: server key store, escrow, backup key files
  tombstone audit: ErasureCompleted{pseudonym, key_ids, t, sig}
  residuals: ciphertext-only (computational guarantee; document ZFS COW caveat)
CI invariant: storage-class registry; every class registered with an encryption adapter
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_infra.toml`; sections `compliance_*`.

Critical vectors: WORM-covered delete → refused + audited; governance bypass → audited with reason; hold beats retention beats user-delete (matrix); Merkle inclusion proof (valid/tampered); forged head → signature verify fails; forward-integrity (compromise at t cannot forge epoch < t); PII purge at T_PII; erasure → plaintext recovery attempt fails (post-shred read); storage-class registry CI failure on unregistered class.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_infra.toml`.

Summary: WORM MUST have an enforcement mechanism beyond an advisory flag (`DC-COMP-001`); legal hold MUST block all deletion paths (`DC-COMP-002`); erasure MUST cover every registered storage class (`DC-COMP-003`); audit epochs MUST be Merkle-headed and dual-signed with published heads (`DC-COMP-004`); PII MUST be purged from audit entries at T_PII (`DC-COMP-005`); every deletion decision MUST be auditable from the log alone (`DC-COMP-006`).

---

## YP-8: Bibliography

1. NIST. **SP 800-88 Rev. 1**, *Guidelines for Media Sanitization* (Dec 2014, DOI `10.6028/NIST.SP.800-88r1`); **WITHDRAWN 2025-09-26; superseded by Rev. 2**. Crypto-erase is the relevant sanitization category; re-baseline deletion docs against Rev. 2. [VERIFIED]
2. AWS S3, *Locking objects with Object Lock*: **Compliance mode** (nobody shortens, only account deletion escapes) vs **Governance mode** (`s3:BypassGovernanceRetention`); legal holds; per-version retention; variable retention/event hold; Cohasset SEC 17a-4/CFTC/FINRA assessment. [VERIFIED]
3. Schneier, B., Kelsey, J. "Secure Audit Logs to Support Computer Forensics." *ACM TISSEC* 2(2), 1999. DOI `10.1145/317087.317089`. Forward integrity via backward key evolution. [VERIFIED]
4. Pulls, T., Peeters, R. "Balloon: A Forward-Secure Append-Only Persistent Authenticated Data Structure." **ESORICS 2015**, ePrint `2015/007`. **Attribution corrected: "Kronblom, Mu … 2019" is wrong.** [VERIFIED]
5. **RFC 6962** (Certificate Transparency); **obsolete; superseded by RFC 9162** (CT 2.0). MTH with 0x00/0x01 domain separation, audit paths, consistency proofs, signed tree heads; the template for Ferro's audit epochs. [VERIFIED]
6. GDPR, Regulation (EU) 2016/679; **Art. 17**: erasure grounds 17(1)(a)–(f); 17(2) de-linking; **17(3) exemptions** incl. 17(3)(f) "establishment, exercise or defence of legal claims" (the hook reconciling immutable audit with erasure); Art. 15 (access), Art. 20 (portability). [VERIFIED full text]
7. Verifiable-deletion frontier (ePrint `2023/370` Bartusek-Khurana-Poremba; `2023/538` Kitagawa-Nishimaki-Yamakawa; `2023/559`; CRYPTO 2024 Bartusek-Raizes; EUROCRYPT 2025 Katz-Sela): all quantum-certified-deletion; **not applicable to classical servers; they confirm classical deletion = key destruction + copy control.** [VERIFIED]
8. IEEE **1619** (XTS-AES, block-storage crypto; relevant to disk encryption) vs IEEE **1667** (device authentication, not WORM). Titles from memory; verify editions before citing in formal docs. [UNCERTAIN]
9. ISO/IEC 27040 (storage security; 2024 revision believed); not fetched. [UNCERTAIN]
10. Google Vault (matters/holds), Microsoft Purview (records management, eDiscovery holds, unified audit log), Dropbox retention/team policies; product-documentation facts; UX template for Ferro's compliance tab. [UNCERTAIN; not re-fetched]
11. Nextcloud Retention + Privacy/Data Request apps; app names well-known; doc URLs moved; verify against current app store. [UNCERTAIN]
12. TrueNAS/ZFS `zfs snapshot|hold|release|destroy -d` semantics; hold-protected snapshots as a genuine WORM enforcement layer on Ferro's actual substrate. [VERIFIED mechanism]

---

## YP-9: Revision 1.1: Hardening Pass (Iteration R2-2, 2026-10-14)

### R2-2.1: Frontier Finding: MMR beats linear chain and CT-style trees for the epoch log

**Source:** Todd, P. "Merkle Mountain Ranges"; OpenTimestamps server design doc (`opentimestamps-server/doc/merkle-mountain-range.md`). [VERIFIED; fetched this session]

The YP-5 audit head design left the intra-epoch structure unspecified. Three candidates:

| Structure | Append cost | Per-event inclusion proof | On-disk behaviour |
|-----------|-------------|---------------------------|-------------------|
| Linear hash chain | O(1) | **impossible without full prefix** (prefix evidence only) | append-only |
| CT 2.0 / Trillian tree (RFC 9162) | O(log n) w/ careful chunking | O(log n) | needs staged tree write-out |
| **Merkle Mountain Range** | O(1) amortized | **O(log n)** | **strictly append-only array; n−1 intermediate hashes; "bag the peaks" gives one signable digest** |

**Decision (upgrade):** the epoch audit log is an MMR. Peaks live in a SQLite append-only table; at epoch rollover the bagged peak digest is dual-signed (Ed25519 + ML-DSA-65 per YP-5) and the signature is published (RFC 9162-style log). This gives every single audit event an O(log n) inclusion proof against a signed public head; the property a linear chain fundamentally cannot provide, and the reason Azure-style linear ledgers are insufficient for per-event compliance queries.

**New theorems (added to scope; all proved in Lean companion):**

| ID | Statement | Formal status |
|----|-----------|---------------|
| `THM-MMR-APPEND-STABILITY` | When the newest stored peak's height differs from the new entry, append is pure: existing peaks are untouched (append-only on-disk property) | **Proved**; Lean `mmrAppend_no_merge` |
| `THM-MMR-MERGE-DETERMINISM` | Peak merges are a pure function of the two children digests | **Proved**; Lean `mmrAppend_merge` |
| `THM-MMR-NO-DROP` | Append strictly increases the covered base-entry count; no entry can be silently dropped | **Proved**; Lean `countBase_strict` |
| `THM-AUDIT-TAMPER-EVIDENT` (model) | Dropping the newest entry of an epoch changes the bagged anchor; silent truncation is detected | **Proved**; Lean `anchor_tamper_evident` |
| `THM-KEY-EVOLUTION-STRICT` | Forward-secure epoch keys strictly advance and are injective; a compromised key is never equal to another epoch's key; recovery of K_{e−1} from K_e is delegated to one-wayness of H (external assumption, stated) | **Proved (strict+injective)**; Lean `keyEv_strict`, `keyEv_injective`, `keyEv_no_revisit` |
| `THM-PII-EPOCH-UNLINKABLE` | Stored PII values from different epochs are never equal; no cross-epoch linkage is computable from the stored representation (GDPR Art. 17/25 compatibility, model level) | **Proved**; Lean `pii_epoch_unlinkable` |
| `THM-WORM-HOLD-PRECEDENCE` (model) | Deletion under an active hold is a no-op and idempotent | **Proved**; Lean `hold_preserves`, `hold_delete_idempotent` |

### R2-2.2: Comparative: what incumbents actually ship

Nextcloud admin audit: plain DB rows, no hashing. Seafile: no audit chain. S3: CloudTrail delivers to a bucket; integrity is the bucket's job (Object Lock), not the log's. QLDB/Azure SQL ledger: linear chains (prefix evidence only). **None of the self-hosted file-cloud incumbents ship per-event inclusion proofs against a signed head.** Ferro's MMR + dual-sig heads would be ahead of the entire category; the PII per-epoch salt additionally makes audit retention GDPR-compatible by construction rather than by purge cron.

### R2-2.3: Lean Proof Companion

`formal/Ferro/AuditChain.lean` (new; imported by `Ferro.lean`). MMR model: peaks newest-first, one merge pass per append; toy hash `d₁·33 + d₂ + 7` stands in for SHA-256 with collision resistance/one-wayness as documented external assumptions.

### R2-2.4: Bibliography Additions

13. Todd, P. "Merkle Mountain Ranges." OpenTimestamps design doc, 2019–present. Append-only, O(log n) inclusion proofs, bag-the-peaks signing. [VERIFIED]

---

## R4-1 Addendum: MMR Audit Chain: IMPLEMENTED

`crates/audit-log/src/mmr.rs` implements ALG-MMRLOG-001's structural contract against the Lean model (`AuditChain.lean`): append-only MMR over entry hashes; older peaks stable under append; bagged epoch anchor (deterministic, truncation-changing); O(log n) per-event inclusion proofs with verification; persistence rebuild on open (leaves = hex-decoded entry hashes, matching live and rebuilt MMRs exactly). `AuditLog::record` extends the MMR transactionally with the SQLite row; `audit_bag()` / `inclusion_proof()` / `verify_inclusion()` are public. 43 tests green. Still staged: dual-signing the anchor (Ed25519+ML-DSA-65) and PII epoch salting at the persistence layer.

---

## R5 Addendum: Epoch Anchor Signing: IMPLEMENTED (Ed25519 half)

`crates/audit-log/src/anchor.rs`: Ed25519-signed epoch anchors over the bagged MMR digest (ring; PKCS#8 root key). `AuditLog::sign_epoch_anchor` / `verify_epoch_anchor` bind the anchor to the live bag; new entries invalidate old anchors (tamper/truncation evidence). The YP's dual signature carries a reserved `mldsa65_signature` slot; the ML-DSA-65 half is staged pending a PQ dependency. 49 audit-log tests green.

---

## R6 Addendum: Dual Signature COMPLETE (Ed25519 + ML-DSA-65)

The ML-DSA-65 half is live: `crates/audit-log/src/anchor.rs` now implements `sign_anchor_dual` / `verify_anchor_dual` over the FIPS 204 ML-DSA-65 parameter set (`ml-dsa = "0.1.1"`, RustCrypto; deterministic variant, seed-based keys via `generate_mldsa65_seed`). `AuditLog::sign_epoch_anchor_dual` produces both signatures over the bagged MMR digest; `verify_epoch_anchor` enforces the dual contract whenever the PQ half is present (a tampered or truncated ML-DSA signature fails even when the Ed25519 half is valid; tested). Classical-only anchors remain verifiable on the Ed25519 half for backward compatibility. 54 audit-log tests green.
