# YP-MESH-TRANSPORT-001: Mesh Sync — Transport, Discovery & Peer Security

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-MESH-TRANSPORT-001                      |
| Domain       | Distributed Systems / P2P                  |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/desktop/src/sync/mesh.rs`, `crates/server/src/mesh_peers.rs`, `docs/mesh-sync.md` |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| `register()` posts **full block-hash inventory** to a central registry, TTL 600 s | `mesh.rs` (`REGISTRATION_TTL_HINT = "600"`) |
| `query_block_holders(hash)` — hash→holders lookup via registry | `mesh.rs` |
| `fetch_block_from_peer()` — plain HTTP, no peer authentication | `mesh.rs` |
| `discover_local_addresses()` enumerates *own* addresses — no peer discovery | `mesh.rs` |
| Registry: DashMap, 10-min TTL | `crates/server/src/mesh_peers.rs` |

**Five concrete problems:**
1. Registry is a hub-and-spoke SPOF — this is not a mesh, it is an index with peer links.
2. `register(full_inventory)` is an **active, by-design existence oracle**: worse than Dropbox's passive leak — Ferro hands the registry a complete content map every 10 minutes.
3. Hash→IP routing does not scale past ~100 peers, leaks, and breaks when the registry is down.
4. No peer discovery exists on a LAN (self-address enumeration is not discovery).
5. No peer authentication: the mesh is an unauthenticated cross-tenant data oracle.

---

## YP-2: Executive Summary

### Problem Statement

Ferro's mesh must let peers exchange blocks without (a) leaking content inventories, (b) depending on a central registry for correctness, (c) trusting peers for integrity, or (d) failing when NATs block direct paths.

### Decisions

1. **Delete inventory registration.** Replace with Bloom-filter announcements (≈24 KB at 13k blocks, 1% FPR) every 1800 s; registry stores `(instance_id → {addrs, expires_at, last_seen})` only — never a hash. ~100× reduction vs current and structurally eliminates the registry-side leak.
2. **Discovery:** RFC 6762 mDNS on the LAN (`_ferro-sync._tcp`, TXT `id=`, `p=`), registry as a WAN hint every 1800 s, manual `ferro://join/<code>` as the mandatory fallback (corporate networks block mDNS).
3. **Transport:** QUIC via `quinn` (TLS 1.3), RFC 9221 DATAGRAM considered for block bodies with explicit per-block reliability. Peer identity = `base32(SHA-256(cert))` (Syncthing model) — the ID is *deduced from the certificate*, never transmitted. Do NOT adopt libp2p (≈50k-line stack; Ferro needs none of DHT/gossipsub/autonat at 50 users). Noise is NOT needed (QUIC = TLS 1.3 covers it; cite only).
4. **NAT traversal:** DCUtR shape — relay-first, synchronised hole punch as optimisation. Budget against the measured 70% ± 7.1% conditional success (M17); ~29% of attempts fail *before* punching (relay reservation / address discovery), so the relay path must be genuinely good.
5. **Integrity:** every block from any peer re-hashed (BLAKE3) and discarded on mismatch. A malicious mesh can waste bandwidth; it cannot corrupt data.

### Guarantees

- **Authenticity:** connections only accepted from accept-listed certificate-derived device IDs.
- **Integrity:** accepted blocks always hash-match their key.
- **No existence oracle:** registry and holder queries leak nothing cross-tenant.
- **Inventory efficiency:** O(log(1/δ)) hashes per distinct element + fixed Bloom overhead.
- **Degradation:** registry-down does not break LAN peers (mDNS path independent).

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| Bloom announcement | Fixed-size bit-vector summary of a peer's block set; FPR-tunable. |
| DCUtR | Direct Connection Upgrade through Relay (libp2p): relay reserve → synchronised punch → direct upgrade. |
| Device ID | `base32(SHA-256(peer certificate))`; self-certifying, cert-derived. |
| Relay-first | Design stance: relay is the default data path; hole punching is an optimisation. |
| RFC 5780 | NAT behaviour discovery via STUN (`OTHER-ADDRESS`, `CHANGE-REQUEST`) — classify NAT before punching. |
| Constant-shape answer | Response whose size and timing are independent of the sensitive predicate. |
| mDNS reflector | Enterprise multicast boundary device; forwards mDNS across L2 domains. |

---

## YP-4: Theoretical Foundation

### THM-MESH-AUTHENTICITY

`∀` peers `p, q`: if `p` accepts a connection from `q` then `DeviceID(q) = base32(SHA-256(cert_q))` and `DeviceID(q) ∈ AcceptList(p)`. Symmetrically for the registry: an announce for id `X` is honoured only if the TLS client certificate presented yields `DeviceID = X`. (Syncthing Global Discovery v3's property: the ID is deduced from the certificate — never transmitted, never trusted separately.)

### THM-MESH-INTEGRITY

`∀` blocks `b` accepted from any peer `q` by peer `p`: `BLAKE3(b) = k ⟹` the canonical bytes served for key `k` equal `b`. Implementation MUST re-verify every block on receipt and discard mismatches. Corollary: a fully malicious mesh causes unbounded bandwidth waste but **cannot** cause data corruption (IPFS CIDs-as-verification, Syncthing behaviour).

### THM-MESH-NO-EXISTENCE-ORACLE

For tenant `T` and hash `h`, the distribution of (response body, latency, connection behaviour) of the CAS, the registry, and the holder query is statistically independent of whether any `T' ≠ T` stores `h`. Satisfied by: registry stores no hashes; per-tenant namespace; constant-size/constant-time answers; Bloom-filter-only inventory exchange. **VIOLATED today** by `register(full_inventory)` every 600 s.

### THM-INVENTORY-EFFICIENCY

For inventories `I_a, I_b` with `|I_a ∩ I_b| / |I_a ∪ I_b| ≥ 1 − δ`, inventory exchange transfers `O(|I_a ∪ I_b| · H(δ))` bits (H = binary entropy) plus a fixed Bloom overhead `k·m/8` bytes with `k = (m/|I|)·ln 2`, `m ≈ |I|·14.4` bits for 1% FPR. Concretely at `|I| = 13,000` (13 GB @ 1 MiB chunks): `m ≈ 23.4 KB`, `k = 10` → **~24 KB per announce** vs ~2.5 MB of raw hashes today → **~100× reduction**.

### THM-MESH-DEGRADATION

If the registry is unavailable for duration `Δ`, then for any two peers `p, q` on the same L2 broadcast domain with mDNS enabled, `p` and `q` SHALL discover each other and complete a block transfer within 5 s of each becoming ready. (One query/response round trip; RFC 6762 §5 flood controls make this safe at Ferro's peer count.) Precondition: L2 domain does not filter `224.0.0.251` / `FF02::FB`.

---

## YP-5: Algorithm Specification

### ALG-ANNOUNCE-001: Bloom Inventory Exchange

```
every 1800 s (WAN registry) and on mDNS TXT (LAN, optional compact form):
  bf ← BloomFilter(m = 14.4·|I| bits, k = 10)
  for h in inventory: bf.add(h)
  announce(instance_id, bf, listen_addrs, lease=600s)
registry stores ONLY: instance_id → {addrs, expires_at, last_seen}
peer-to-peer inventory reconciliation (optional refinement):
  Merkle-DAG subtree exchange over sorted hash set; only differing leaves transfer
```

### ALG-DISCOVER-001: Discovery Ladder

```
1. mDNS (RFC 6762): service _ferro-sync._tcp, TXT {id=<device_id>, p=<quic_port>}
   flood controls ON: known-answer suppression, duplicate-question suppression,
   unicast-response bit, records-per-advertisement cap
2. registry hint (WAN): announce every 1800 s; lookup returns addr candidates only
3. manual: ferro://join/<invite-code>   # mandatory fallback; corporate networks block mDNS
```

### ALG-TRANSPORT-001: QUIC + Cert-Derived Identity

```
quinn endpoint, TLS 1.3; self-signed X.509 per device at first boot
on connect: DeviceID(remote) := base32(SHA-256(remote_cert))
  reject unless DeviceID(remote) ∈ AcceptList
block fetch: reliable QUIC streams (default) | RFC 9221 DATAGRAM + per-block retry (opt-in)
on receipt: BLAKE3(bytes) == requested key else discard + audit
registry: cert-pinned by DeviceID (?id=...), never CA-bundle (self-hosted, no CA)
congestion control: BBR or CUBIC; Prague/L4S LAN-only option; NEVER L4S on WAN
  (RFC 9331 §4.3.1: up to 25:1 L4S-vs-Classic imbalance)
QUIC multipath (draft-ietf-quic-multipath-21, RFC-Editor queue): DEFER; keep MeshTransport
  trait abstraction so it can be added (per-path CC has documented fairness problems)
```

### ALG-NAT-001: Relay-First Traversal

```
1. maintain relay path always (Ferro server is natural relay for its own users)
2. STUN (RFC 5389) address discovery; optional RFC 5780 NAT classification
3. synchronised hole punch: both sides send to peer's mapped address in a
   ±jitter window (RTT-based synchronisation — the mechanism behind the
   measured TCP/QUIC parity result)
4. on failure: fall back to relay permanently for that peer pair; record it
budget: 70% ± 7.1% conditional success; ~29% fail BEFORE punching → relay must be excellent
symmetric NAT: prefer relay over port-prediction (~170k packets for 99.9% — noisy, not worth it)
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_sync_stack.toml` — sections `mesh_*`.

Critical vectors: Bloom FPR at stated (m, k) over fixture inventories; announce size bound; registry-down LAN discovery within 5 s; cert-derived ID equality/rejection; block-verify discard; constant-shape answer timing (statistical test); mDNS flood-control activation.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_sync_stack.toml`.

Summary: registry MUST NOT store block hashes (`DC-MESH-001`); every fetched block MUST be re-verified before persistence (`DC-MESH-002`); peer identity MUST derive from the TLS certificate (`DC-MESH-003`); block-holder answers MUST be constant-shape (`DC-MESH-004`); mDNS MUST NOT be a hard dependency (`DC-MESH-005`); L4S/Prague MUST NOT be enabled on WAN paths (`DC-MESH-006`); mesh authorisation ("peer A may fetch block h for tenant T") MUST be evaluated server-side — **the biggest unsolved gap in the current design** (`DC-MESH-007`).

---

## YP-8: Bibliography

1. DeCandia, G., et al. "Dynamo: Amazon's Highly Available Key-value Store." *SOSP '07*. Quorum + vector-clock siblings + hinted handoff. [VERIFIED]
2. Ford, H., et al. "Availability in Globally Distributed Storage Systems." *OSDI '10*. [VERIFIED]
3. Hu, R., Arpaci-Dusseau, A., Arpaci-Dusseau, R. "A Unified, Practical, and Understandable Model of Non-transactional Consistency Levels in Distributed Replication." arXiv:`2409.01576`. Recommended consistency-model frame for the mesh. [VERIFIED]
4. **RFC 9000** (QUIC) — critical text: *"Path validation is not designed as a NAT traversal mechanism."* Do not assume QUIC gives hole punching for free. [VERIFIED]
5. **RFC 9001** (TLS for QUIC); **RFC 9002** (loss recovery/CC); **RFC 9221** (unreliable datagram); **RFC 9114** (HTTP/3); **RFC 9369** (QUIC v2). [VERIFIED]
6. `draft-ietf-quic-multipath-21` (2026-03-17, RFC-Editor queue): explicit Path IDs, per-path AEAD/CC/RTT/PMTU. [VERIFIED]
7. TUM NET (Nov 2025), "Congestion Control Schemes for Multipath QUIC": per-path default CC has fairness problems; LIA/OLIA coupling needed. [VERIFIED]
8. `draft-ietf-quic-load-balancers-15` (QUIC-LB) — still an Internet-Draft; do not cite as an RFC. [VERIFIED]
9. **RFC 9331/9332/9330** (L4S architecture, DualQ coupled AQM, ECN): dCTCP-style CC; **25:1 worst-case imbalance hazard** (RFC 9331 §4.3.1). [VERIFIED]
10. `draft-briscoe-iccrg-prague-congestion-control` (Prague CC; Apple QUIC implementation experience). [VERIFIED]
11. Huitema, C., Mahy, R. "Peer-to-Peer Communication Across Network Address Translators." (`pdos.csail.mit.edu/papers/p2pnat.pdf`): 82% of NATs UDP-hole-punch compatible; ~64% TCP. [VERIFIED]
12. ⭐ "Challenging Tribal Knowledge — Large Scale Measurement Campaign on Decentralized NAT Traversal" / DCUtR in IPFS. arXiv:`2510.27500` (2025) & arXiv:`2604.12484` (2026): 4.4M attempts, 85k networks, 167 countries — **70% ± 7.1% conditional hole-punch success; TCP ≈ QUIC (refutes "UDP is superior"); ~29% fail before punching**. [VERIFIED]
13. "Mass Adoption of NATs: Survey and experiments on carrier-grade NATs." arXiv:`2311.04658`: 75% penetration across 4 Dutch carriers; symmetric-NAT port-prediction ≈ 54k packets for 50%, ~170k for 99.9%. [VERIFIED]
14. **RFC 8445** (ICE), **RFC 5389** (STUN), **RFC 5780** (NAT behaviour discovery), **RFC 4787** (NAT UDP requirements, BCP 127). [VERIFIED]
15. **RFC 6762** (mDNS): `224.0.0.251`/`FF02::FB`, port 5353; known-answer suppression, duplicate-question suppression, unicast-response bit; end-to-end crypto caveat. [VERIFIED]
16. Cisco (Jan 2023), "Multicast DNS (mDNS) – Still Flooding?": enterprise mDNS flooding; reflectors as the answer. [VERIFIED]
17. Perrin, T. "The Noise Protocol Framework," rev 34, 2018. Deployed in WireGuard/Lightning/WhatsApp. **Cite for rationale; do not implement (QUIC suffices).** [VERIFIED]
18. IPFS CID spec; IPNS self-certifying records; libp2p docs (Noise/TLS 1.3 default encryption; built-in hole punching, AutoNAT, relay). [VERIFIED]
19. Benet, J. "IPFS - Content Addressed, Versioned, P2P File System." arXiv:`1407.3561`. [VERIFIED]
20. Syncthing security model docs: Device ID = SHA-256(cert) BASE32, accept-list, 30-min global discovery, documented discovery-server and LAN-eavesdropper leaks. [VERIFIED]
21. Syncthing Global Discovery v3: device ID deduced from the presented certificate (never in payload); cert pinning via `?id=`. [VERIFIED]
22. Nextcloud Federation: OCM share pull model; `oc:share-permissions`; advisory locks. [VERIFIED]
23. "WebRTC Metadata and IP Leakage in Modern Browsers." arXiv:2510.16168 (Oct 2025): mDNS obfuscation is session-stable pseudonymity, not anonymity. [VERIFIED]
24. **Negative results:** "Siena" (SOSP '09) and "PBFS/opportunistic file system" — no primary records verified this session; do not cite. [VERIFIED ABSENCE]

---

## YP-9: Revision 1.1 — Hardening Pass (Iteration R2-4, 2026-10-14)

### R2-4.1: Frontier Finding — close DC-MESH-007 with attenuated capability chains (UCAN-shaped)

**Source:** Zelenka, B. (ed.), Gozalishvili, I., Holmgren, D., Krüger, P. "User Controlled Authorization Networks (UCAN) Specification," v1.0.0, `github.com/ucan-wg/spec`. [VERIFIED — fetched this session]

DC-MESH-007 ("peer A may fetch block h for tenant T") was flagged the biggest unsolved gap. The ACL reflex — evaluate every mesh fetch server-side — reintroduces the coordinator the mesh exists to avoid. The capability alternative, concretized from UCAN:

- **Root of trust = the tenant owner.** Root capability: subject = `tenant:{T}`, command = `/ferro/block`, delegated to each user's devices.
- **Delegation is attenuation-only** (UCAN §Attenuation): every link must restate or narrow `(subject, command)` and nest the validity interval. Command specialization is prefix-ordered: `/ferro` proves `/ferro/block` proves `/ferro/block/read`; `/` (top) proves everything and MUST be delegated sparingly.
- **A mesh peer presenting a block-fetch invocation validates:** chain signatures (Ed25519, DAG-CBOR-canonical), adjacent attenuation links, `subject == tenant-of(h)`, invocation-time ∈ leaf interval, CID uniqueness (replay), revocation status. No server round-trip; the check is local and self-certifying.
- **Failure mode:** absent/invalid chain ⇒ deny (fail-closed). Chains are the *only* mesh authority — no ambient identity grants fetch rights (UCAN FAQ: no ambient authority; least authority throughout).

**New theorems (added to scope; all proved in Lean companion):**

| ID | Statement | Formal status |
|----|-----------|---------------|
| `THM-CMD-ORDER-PARTIAL` | The command proof order (prefix) is reflexive, antisymmetric, transitive — a partial order | **Proved** — Lean `cmd_proves_refl/antisym/trans` |
| `THM-CAP-CHAIN-SOUNDNESS` | Attenuation is transitive: leaf authority is derivable from root authority by adjacent-link checks alone | **Proved** — Lean `attenuates_trans` |
| `THM-CAP-NO-AMPLIFICATION` | Any invocation authorized by a leaf was authorized by the root — delegation narrows, never widens | **Proved** — Lean `no_amplification` |
| `THM-CAP-TENANT-CONTAINMENT` | Attenuation preserves subject: a chain rooted at tenant T structurally cannot authorize operations on tenant T′ — DC-MESH-007 closed by construction | **Proved** — Lean `tenant_containment` |
| `THM-CAP-INTERVAL-CONTAINMENT` | The leaf validity interval is contained in the root's — a chain is never valid outside its weakest link | **Proved** — Lean `interval_containment` |

**Rejected:** server-side ACL evaluation for mesh fetches (defeats partition tolerance; the YP's own relay-first design would become a sync bottleneck); macaroons-style third-party caveats (caveat discharge requires the discharger online — same coordinator problem). Biscuit tokens (Datalog caveats, offline verification) remain a viable alternative *format*; the Lean model above is format-independent and applies to either.

### R2-4.2: Comparative

Syncthing: device-ID allow-list, all-or-nothing per folder — no per-tenant attenuation, no delegation. IPFS/IPLD: raw content addressing, authorization out of scope. Nextcloud federation (OCM): server-to-server share tokens, coordinator required. **UCAN-style attenuation + tenant-anchored roots is the frontier: public-key verifiable, partition-tolerant, and provably non-amplifying — no incumbent self-hosted sync ships it.**

### R2-4.3: Lean Proof Companion

`formal/Ferro/Capability.lean` (new; imported by `Ferro.lean`). Commands modelled as segment lists; proof order = suffix-existence (equivalent to UCAN's `/`-prefix order); capabilities = `(subject, cmd, nbf, exp)`. Signature verification and hash one-wayness are external assumptions.

### R2-4.4: Bibliography Additions

25. Zelenka, B. (ed.), et al. "UCAN Specification" v1.0.0, UCAN Working Group, `ucan-wg/spec`. Attenuation-only delegation, `/`-prefix command ordering, chain validity intervals, CID replay prevention. [VERIFIED]
