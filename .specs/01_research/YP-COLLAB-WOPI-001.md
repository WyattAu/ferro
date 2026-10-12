# YP-COLLAB-WOPI-001: Office Collaboration, WOPI Proof Security & Session Authorization

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-COLLAB-WOPI-001                         |
| Domain       | Collaboration / Document Editing Security  |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/server-wopi/src/lib.rs`, `crates/server/src/collab_ws.rs`, `crates/server/src/integration/crdt_integration.rs` |

---

## YP-1: Ground Truth (Current Implementation)

| Aspect | Ferro today | Location |
|---|---|---|
| Edit protocol | WebSocket + CRDT integration | `collab_ws.rs`, `crdt_integration.rs` |
| Office editing | WOPI host for Collabora (collabora.wyattau.com verified 200) | `server-wopi/src/lib.rs` |
| Token auth | HMAC-SHA256 scoped access tokens | `server-wopi/src/lib.rs` |
| **Request proof** | **NONE — no `X-WOPI-Proof`, no proof-key handling anywhere in the repo** | repo-wide grep |
| Sandbox posture | Delegated entirely to Collabora (`WSD → ForKit → per-doc jailed Kit`) | Collabora docs |

**CRITICAL SECURITY DEFECT (P0):** Ferro implements WOPI *access tokens* (authorization) but not the WOPI *request proof* (host authentication). A valid token is sufficient for any source that can reach the endpoint and knows the URL to make Collabora serve or accept writes. Same defect class as Cloudreve advisory `GHSA-c3jm-gv5r-9wcp` (WOPI token suffix comparison → view session performed writes).

**Secondary:** WebSocket "authenticate step" is not authorization — document-open must evaluate Cedar and re-evaluate on revocation; awareness messages are untrusted input; CRDT full-history resync is O(document) per client.

---

## YP-2: Executive Summary

### Problem Statement

Ferro embeds Collabora over WOPI and runs a WebSocket CRDT channel. Both must be: authenticated (WOPI proof + freshness), authorized per-operation (Cedar at open and on revocation), and bandwidth-bounded (state-vector delta sync, never full history).

### Decisions

- **P0 — Implement WOPI request proof** (proof-key pair with rotation, `X-WOPI-Proof` old‖current HMAC verification, ±20 min timestamp freshness, scope check before any file lookup; never fall through to token-only).
- **P1 — Bind sessions to authorization:** on every document-open, evaluate Cedar `Read`/`Write` for the document's space; record the decision in the session; re-evaluate on membership change and token expiry; reject updates from revoked sessions.
- **P2 — Never transmit full history:** per-document compacted update log; on connect `SyncStep1` (state vector) → server emits only the missing delta.
- **P3 — Treat awareness as hostile input:** cap frame size, rate-limit per session, never render unsanitized payloads, never derive authorization from awareness (y-protocols explicitly does not authenticate awareness).
- **P4 — Document the delegation boundary:** Ferro's isolation is Collabora's `ForKit → jailed Kit` boundary, not its own sandboxing; tune `WSD` concurrency against fork-bomb bursts.
- **P5 — Consider server-ordered relay for Office formats:** binary OOXML has no good CRDT; Google reached the same conclusion (Wave-era server sequencing).

### Guarantees

- Served WOPI requests are authenticated by a *fresh* proof.
- Invalid proofs never degrade to token-only acceptance.
- Served operations are within the session's permitted scope.
- All replicas of a collaboratively-edited document converge (given reliable delivery + associative/commutative/idempotent merge).
- No update from an unauthorized session is ever merged.

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| WOPI | MS-WOPI protocol: REST API by which an Office-like client (Collabora) reads/writes files in a host (Ferro). |
| Access token | Per-session bearer token the host issues to the WOPI client. |
| Request proof | `X-WOPI-Proof` (old‖current pair) + `X-WOPI-TimeStamp` HMAC authenticating the host to the client and vice versa; mandatory in restricted scenarios. |
| Proof-key rotation | Hosts publish current+old keys so key changes don't break in-flight sessions. |
| State vector | Compact per-replica clock describing what updates a replica already has. |
| Awareness | Ephemeral presence channel (cursors, selections); unauthenticated by y-protocols. |
| ForKit / Kit | Collabora's launcher / per-document jailed process pair. |

---

## YP-4: Theoretical Foundation

### THM-WOPI-PROOF-SAFETY

`∀ r : WopiRequest, k : ProofKey`: `Serve(r) = some response → ∃ t, v. VerifyProof(r, k) = some (t, v) ∧ Fresh(t) ∧ HMACValid(k, r)`. A request is served only under a valid, fresh proof.

### THM-WOPI-NO-FALLTHROUGH

`∀ r, k`: `VerifyProof(r, k) = none → Serve(r) = none`. Invalid proof never degrades to token-only auth. (Directly repairs the P0 defect.)

### THM-WOPI-SCOPE

`∀ r, t`: `TokenValid(t) → Serve(r) = some response → r.operation ∈ permittedOps(t.scope)`. The Cloudreve-class bug (view session performing writes) is excluded.

### THM-COLLAB-CONVERGENCE

`∀` replicas `n`, update log `u`: (reliable delivery ∧ merge associative/commutative/idempotent) `⟹ n.map(state) = merge(u)` — all replicas reach the same state.

### THM-COLLAB-AUTHZ

`∀ s : Session, u : Update, st : DocState`: `s.document ∉ permittedDocs(s) → merge(st, u) = st`. No update from an unauthorized session is merged.

### THM-COLLAB-SYNC-SOUND

State-vector sync delivers a *sufficient* delta: `∀ sv, s₁, s₂`: `sv subsumes s₁ → ApplyDelta(s₁, Diff(s₁, s₂)) = s₂` — and the delta is minimal (no full-history transmission).

### Quantitative note (why P2 is a requirement, not an optimization)

Kleppmann (*CRDTs: The Hard Parts*, Hydra 2020) measured a document at **146,406,415 bytes** of full-history JSON vs **6,132,895 bytes** compact binary vs **104,852 bytes** plain text — a ~1,400× spread. The frequently-quoted "1 MB document → 1 TB of metadata" figure was **not found** in that talk and must not be cited. [VERIFIED figures; VERIFIED ABSENCE of the 1TB quote]

---

## YP-5: Algorithm Specification

### ALG-WOPIPROOF-001: Request Proof Verification

```
setup: per Collabora registration generate proof_key_current, proof_key_old
per request:
  1. parse X-WOPI-Proof = ⟨oldValue, currentValue⟩; X-WOPI-TimeStamp (HHMMSS+NNNN UTC)
  2. verify HMAC-SHA256(current, url ‖ "\n" ‖ token ‖ "\n" ‖ timestamp)
  3. verify HMAC-SHA256(old, url ‖ "\n" ‖ token)                    # rotation tolerance
  4. verify timestamp freshness within ±20 min
  5. verify token scope for the requested operation (view ≠ write!)
  6. on any failure: 401 + X-WOPI-ItemError; NEVER fall through to token-only
```

### ALG-SESSIONAUTHZ-001: Bound Sessions to Cedar

```
on document-open: decision := cedar_eval(principal, document.space, {Read, Write})
  record in session; cache keyed by (principal, space, policy_set_version)
on membership change / token expiry: re-evaluate; revoke live sessions whose decision flips
per-update: reject if session.decision lacks Write for Write updates
```

### ALG-DELTASYNC-001: Bounded Update Sync

```
persist per-document compacted update log (SQLite/PG, WAL)
on connect: client SyncStep1(state_vector) → server emits Diff(state_vector, head) only
compaction: merge log when entries > threshold (2k) or age > 24 h; keep GC'd history in versions
awareness: frame ≤ 4 KiB, 10 msg/s per session, sanitize-escape on render, never innerHTML
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_platform.toml` — sections `wopi_*` and `collab_*`.

Critical vectors: missing proof → reject; stale timestamp (±20 min boundary) → reject; wrong URL in HMAC → reject; tampered token → reject; token-prefix collision pair (Cloudreve regression); view-scope write attempt → reject; delta-sync minimality; awareness overflow/flood; revoked-membership mid-session update → reject.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_platform.toml`.

Summary: no WOPI request is served without proof + freshness + scope (`DC-WOPI-001`); proof failure MUST NOT fall through (`DC-WOPI-002`); token comparison MUST be full-token (`DC-WOPI-003`); sessions MUST carry a Cedar decision re-evaluated on revocation (`DC-COLLAB-001`); full-document serialization to clients MUST NOT occur on sync (`DC-COLLAB-002`); awareness MUST be treated as untrusted input (`DC-COLLAB-003`).

---

## YP-8: Bibliography

1. `[MS-WOPI]` Microsoft Open Specifications — WOPI proof scheme (`X-WOPI-Proof` old‖current, `X-WOPI-TimeStamp`, HMAC-SHA256 over url‖token‖timestamp, freshness; mandatory in restricted scenarios). 2015-09-10 revision verified; current revision number unconfirmed. [VERIFIED scheme / UNCERTAIN current rev]
2. Cloudreve advisory `GHSA-c3jm-gv5r-9wcp` — WOPI token suffixes not compared; view session performed writes. Direct precedent for Ferro's defect class. [VERIFIED]
3. Kleppmann, M. "CRDTs: The Hard Parts." Hydra 2020. Measured metadata spread 146,406,415 / 6,132,895 / 104,852 bytes. [VERIFIED]
4. Yjs `y-protocols` `PROTOCOL.md`: `SyncStep1=0, SyncStep2=1, Update=2; SyncProtocol=0, AwarenessProtocol=1`; awareness ephemeral and unauthenticated. [VERIFIED]
5. Weiss, S., Urso, P., Molli, P. — YATA / Logoot line (Nicolaescu et al., *Web Engineering 2015*, DOI `10.1145/2957276.2957310`). Note: **`lean-yjs`** (Lean 4 formalisation in the yjs repo) states "the pseudocode in the original YATA paper contains errors"; preservation + commutativity proven so far. [VERIFIED]
6. Collabora architecture docs: `WSD → ForKit → per-document Kit`; jails with read-only mounts and `chroot`. [VERIFIED]
7. Google Wave / Google Docs OT (Jupiter): server-sequenced transforms; the canonical production OT system. Wave quota paper ("USENIX ATC 2014") — **not found in the ATC'14 program; do not cite.** [VERIFIED ABSENCE]
8. RFC `9297` (HTTP datagrams / capsule protocol) — relevant only if collaboration moves off WebSockets. [VERIFIED]
9. W3C Solid Web Access Control v1.0.0. [VERIFIED]
10. **Negative results:** "Just-Right Collaboration" arXiv:`2212.02618` resolves to *LIGS* (unrelated) — do not cite; Automerge MobiSys 2018 identifier unverified this session. [VERIFIED ABSENCE / UNCERTAIN]
