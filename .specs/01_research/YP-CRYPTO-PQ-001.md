# YP-CRYPTO-PQ-001: Post-Quantum Readiness, Honest FIPS Posture & Crypto Agility

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-CRYPTO-PQ-001                           |
| Domain       | Cryptography / Compliance                  |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/server-fips/src/lib.rs`, `crates/crypto/src/{traits,ring_provider}.rs`, `crates/server-federation/src/lib.rs` |

---

## YP-1: Ground Truth (Current Implementation)

| Area | File | Current state |
|---|---|---|
| FIPS mode | `server-fips/src/lib.rs:39-51` | `FipsMode{Disabled, Enabled, Strict}`; 5 self-tests incl. SHA-256 KAT (FIPS 180-4 "abc"), HMAC-SHA-256 KAT (RFC 4231 TC2) |
| Key hierarchy | `lib.rs:323-464` | MK→KEK→DK, zeroize on drop, 24 h auto-rotation |
| **Key wrap** | `lib.rs:767-794` | `wrap_key()` = HKDF-SHA-256-derived **XOR pad + HMAC — not AES-KW (SP 800-38F), not an AEAD; not an approved confidentiality mode** |
| **Data encryption** | `lib.rs:847-880` | `EncryptedDataHeader::encrypt()` = HKDF-expand XOR pad; HKDF-expand capped at 255×32 = **8160 B — anything larger cannot be encrypted** (own test comments "Must be <= 8192", `:1229-1235`) |
| Provider | `crypto/src/traits.rs:8-29` | `CryptoProvider::is_fips_approved()`; only backend is `ring` — **ring is not FIPS-validated; the flag is decorative** |
| RNG self-test | `lib.rs:249-265` | statistical uniqueness check, not an SP 800-90A continuous/power-up test |

**Structural gaps:** (1) transport HNDL exposure — all handshakes are classical ECDHE today; (2) the "FIPS" crate is not FIPS — XOR-pad wrap is not approved, provider not validated; (3) no crypto agility — `EncryptedDataHeader` has `key_id`/`key_version` but **no algorithm identifier**, so any migration requires a header format change.

---

## YP-2: Executive Summary

### Problem Statement

Ferro advertises a FIPS posture and must be crypto-agile for the PQ transition, without over-claiming. The regulatory clock: NSA CNSA 2.0 2030–2033; US federal/UK NCSC/EU roadmap 2030/2035. Expert surveys put ≥50% odds of an RSA-2048 break within ~15 years; Gidney (2025) cut the RSA-2048 quantum resource estimate to <1M noisy qubits, pulling plausible Q-day ~7 years closer. Key agreement must move **now** (harvest-now-decrypt-later); signatures can trail to ~2030.

### Decisions

- **Transport (do now):** terminate TLS with **X25519MLKEM768** + X25519 fallback; TLS 1.3 only; 0-RTT off. If Caddy fronts Ferro, upgrade Caddy (Go crypto has ML-KEM default since Go 1.24). In-process `rustls`/`quinn`: build with **aws-lc-rs**; implement `CryptoProvider` for aws-lc-rs alongside ring and set `is_fips_approved()` **truthfully per module**.
- **At-rest (do next):** replace `wrap_key()` with **AES-256-GCM (SP 800-38D)** or **AES-KW-256 (SP 800-38F)** — both FIPS-approved; XOR-pad is neither. Add `alg: AlgorithmId` to `EncryptedDataHeader` (agility). Chunked data encryption: AES-256-GCM, 96-bit random nonce per chunk, 64–256 KiB chunks — also fixes the 8160-byte HKDF ceiling. Document `rotate_all_data_keys()` as *wrap-rotation* (re-wrap, not re-encrypt — correct and cheap).
- **PQ readiness (plan, don't rush):** agility enums first — `KemId::{X25519, MlKem768, X25519MlKem768}`, `SigId::{Rsa256, EcdsaP256, Ed25519, MlDsa65}` in `crypto::traits`; federation keys become keypairs with this enum. JOSE: pre-register an `alg` allowlist including ML-DSA codes so PQ JWT is config, not code (Keycloak timeline unknown; ML-DSA JWS is still I-D stage).
- **Audit-head signing (synergy with YP-LIFECYCLE-COMPLIANCE-001):** sign daily audit heads with **ML-DSA-65 hybrid Ed25519** (dual signature). Long-lived signatures are the IR 8547 "signed today must survive Q-day" case; 3309 B sigs are trivial at this scale. RustCrypto `ml-dsa` acceptable initially; prefer aws-lc ML-DSA once its CMVP cert lands.
- **FIPS positioning (be honest):** Ferro cannot *be* FIPS-validated (that requires its own module validation); it can be **FIPS-140-3-composed**: link a validated module (aws-lc-fips 3.x is in CMVP validation-in-progress per its README — re-check CMVP modules-in-process at deploy time) and state the boundary in SECURITY.md. Keep the power-on KATs; move RNG health testing to the provider's SP 800-90A tests. `FipsMode::Strict` MUST actually gate primitives (`THM-FIPS-APPROVED-ONLY`).

### Guarantees

- Channel keys combining X25519 + ML-KEM-768 are secure if *either* component is hard (hybrid).
- Strict mode only executes approved operations, aborting otherwise.
- Migration cost is bounded, resumable, and idempotent (content-addressed objects).

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| ML-KEM (FIPS 203) | Module-lattice KEM (ex-Kyber); parameter sets 512/768/1024. |
| ML-DSA (FIPS 204) | Module-lattice signatures (ex-Dilithium); 44/65/87; sigs 2420/3309/4627 B. |
| SLH-DSA (FIPS 205) | Stateless hash-based signatures (ex-SPHINCS+). |
| HNDL | Harvest-now-decrypt-later: recording ciphertext today for quantum decryption later. |
| Hybrid key exchange | Classical + PQ shared secrets combined in one KDF (X25519MLKEM768). |
| FIPS 140-3 | US/CMVP validation of a *cryptographic module* (not merely algorithms). |
| Composed FIPS | Product builds on a separately-validated module; boundary language documented. |
| Crypto agility | Runtime-selectable algorithm identifiers in every ciphertext/header. |
| Wrap rotation | Re-wrapping KEKs under a new MK without re-encrypting data. |

---

## YP-4: Theoretical Foundation

### THM-PQ-HYBRID-SEC

Let `A` be a PPT (resp. quantum) adversary against channel security under key schedule `K = KDF(ss_X25519 ‖ ss_MLKEM768, ctx)`. If either X25519-CDH or ML-KEM-768-IND-CCA is hard, then `A` distinguishes `K` from random with advantage ≤ negl. (Formalizes the PQXDH/PQ3 hybrid argument; matches Apple PQ3's Stebila reduction.)

### THM-FIPS-APPROVED-ONLY

In `FipsMode::Strict`, for every operation `op` issued through `CryptoProvider`: `op ∈ ApprovedOps` (AES-256-GCM, SHA-256/384, HMAC-SHA-256, HKDF per SP 800-56C, ML-KEM as guidance matures) — else the call MUST abort before processing sensitive data; power-on KAT failure ⇒ `is_valid() = false` ⇒ all key operations refused. (Closes the gap that `FipsMode::Strict` today doesn't actually gate primitives.)

### THM-CRYPTO-AGILITY-COST

Given header fields `{alg_id, key_id, key_version}` on every ciphertext and content-addressed immutable objects, full-migration re-encryption work is `O(N · C̄)` (N objects × mean chunk count) and is resumable/idempotent because objects are addressed by content hash, not by ciphertext.

### THM-HNDL-BOUND

Data transmitted over channels negotiated with X25519MLKEM768 after date `t` is confidential against adversaries whose quantum capability arrives after `t`, assuming both component hardnesses — classical-only traffic before `t` carries no such bound. **Motivation for enabling PQ now, before any content worth harvesting exists.**

---

## YP-5: Algorithm Specification

### ALG-TLS-001: PQ Transport Configuration

```
TLS 1.3 only; groups: X25519MLKEM768 (preferred), X25519 (fallback); 0-RTT disabled
rustls with aws-lc-rs provider; quinn inherits for the mesh transport (YP-MESH-TRANSPORT-001)
reverse proxy (Caddy): upgrade to a version with Go ≥ 1.24 (ML-KEM default)
```

### ALG-HEADER-001: Agile Ciphertext Header

```
EncryptedDataHeader {
  alg_id   : AeadId (Aes256Gcm | AesKw256 | ...)     # NEW field — agility
  key_id, key_version                          # existing
  nonce  : 96-bit random per chunk
}
chunk ct := AES-256-GCM(chunk_key, nonce, plaintext)   # replaces HKDF-XOR pad
key wrap := AES-KW-256(KEK, DK) under SP 800-38F       # replaces XOR-pad wrap
```

### ALG-AUDITSIG-001: Hybrid Audit-Head Signatures

```
daily: head := Merkle_root(epoch entries)      # see YP-LIFECYCLE-COMPLIANCE-001
sig  := (Ed25519_sign(head), ML-DSA-65_sign(head))     # dual, both attached
verify: BOTH must verify; classical survives PQ cryptanalysis of lattice, vice versa
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_infra.toml` — sections `crypto_*`.

Critical vectors: strict-mode rejection of non-approved ops; KAT retention (SHA-256 "abc", HMAC RFC 4231 TC2); AES-KW wrap/unwrap round trip (RFC 3394 vectors); GCM chunk round trips incl. >8160 B; header alg-id unknown → reject (forward compat); hybrid signature verify (each leg missing → reject); TLS group negotiation matrix.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_infra.toml`.

Summary: `FipsMode::Strict` MUST gate every primitive call (`DC-CRYPTO-001`); every ciphertext MUST carry `{alg_id, key_id, key_version}` (`DC-CRYPTO-002`); key wrap MUST use an approved mode (`DC-CRYPTO-003`); TLS MUST prefer the PQ hybrid group (`DC-CRYPTO-004`); `is_fips_approved()` MUST reflect the actual provider's status (`DC-CRYPTO-005`); ML-DSA private keys stored as seeds (RFC 9881 recommendation) (`DC-CRYPTO-006`).

---

## YP-8: Bibliography

1. NIST. **FIPS 203**: Module-Lattice-Based Key-Encapsulation Mechanism Standard. Published **2024-08-13**. DOI `10.6028/NIST.FIPS.203`. [VERIFIED]
2. NIST. **FIPS 204**: Module-Lattice-Based Digital Signature Standard. 2024-08-13. DOI `10.6028/NIST.FIPS.204` (sizes cross-confirmed in RFC 9881 §10.1). [VERIFIED]
3. NIST. **FIPS 205**: Stateless Hash-Based Digital Signature Standard. 2024-08-13. [VERIFIED]
4. NIST. **IR 8547 (ipd)**: Transition to Post-Quantum Cryptography Standards. 2024-11-12. DOI `10.6028/NIST.IR.8547.ipd`. Moody, Perlner, Regenscheid, Robinson, Cooper. [VERIFIED]
5. Cloudflare. "State of the post-quantum Internet in 2025" (`blog.cloudflare.com/pq-2025/`, Oct 2025): **>50% of human-initiated traffic uses PQ key agreement**; X25519MLKEM768 ready (RFC pending); regulator timelines (CNSA 2.0 2030–2033, federal 2035, NCSC 2035, EU 2030/2035); HQC backup KEM (Mar 2025); Gidney 2025: RSA-2048 < 1M noisy qubits (arXiv:`2505.15917`). [VERIFIED]
6. OpenSSL **3.5.0 NEWS** (2025-04-08): ML-KEM/ML-DSA/SLH-DSA support; **default keyshares changed to X25519MLKEM768 + X25519**; server-side QUIC. [VERIFIED]
7. IETF. **RFC 9881**: Internet X.509 PKI — Algorithm Identifiers for ML-DSA (Oct 2025, ex `draft-ietf-lamps-dilithium-certificates`). Seed-only private keys RECOMMENDED; HashML-DSA disallowed in PKIX. [VERIFIED]
8. Signal. "Quantum Resistance and the Signal Protocol" (PQXDH), 2023-09-19 + spec (`signal.org/docs/specifications/pqxdh`): X25519 + Kyber hybrid for initial key establishment only. [VERIFIED]
9. Apple Security Research. "iMessage with PQ3." 2024-02-21: Kyber-1024 initial, Kyber-768 ratchet ≈ every 50 messages / ≥ every 7 days; HKDF-SHA384; independent analyses (Stebila game-based; Basin/Linker/Sasse Tamarin). [VERIFIED]
10. IETF JOSE WG (fetched Oct 2026): **RFC 9864** Fully-Specified Algorithms (Oct 2025); `draft-ietf-jose-pq-composite-sigs-05` (2026-10-08); `draft-ietf-jose-hpke-pq-pqt-01`; `draft-ietf-jose-pqc-kem-06`. **No ML-DSA JWS/JWT RFC yet.** [VERIFIED]
11. aws/aws-lc-rs README: `aws-lc-fips-sys` binds AWS-LC-FIPS 3.x — validation testing complete, **submitted to NIST, not yet certified**; re-check CMVP at deploy time. (Counter-example verified: NIST cert #5101 = Apple corecrypto 14.1, FIPS 140-3 L1, 2025-12-02 — software modules ARE getting 140-3 certs now.) [VERIFIED]
12. RustCrypto `ml-kem` v0.3.2 (2026-09-25): pure-Rust FIPS 203; explicit "never independently audited" warning. [VERIFIED]
13. **Negative result:** no public Dropbox statement on PQ deployment found — do not accept "Dropbox PQ" claims in parity matrices without a source. [VERIFIED ABSENCE]
14. restic backup format — classical symmetric (AES-256-CTR + Poly1305), scrypt KDF; no PQ action needed at rest; backup *transfer* rides PQ TLS. [UNCERTAIN — re-verify restic docs]
15. Sigstore Rekor / Google Trillian — referenced infrastructure for future audit-head anchoring; deployment option, not dependency. [VERIFIED existence]
