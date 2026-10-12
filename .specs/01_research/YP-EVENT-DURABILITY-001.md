# YP-EVENT-DURABILITY-001: Transactional Outbox, Idempotent Effects & Durable Jobs

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-EVENT-DURABILITY-001                    |
| Domain       | Distributed Systems / Reliability          |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/server/src/events.rs`, `crates/event-bus/src/bus.rs`, `crates/server-automation/src/{webhooks,workflows}.rs`, `crates/audit-log` |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| Dual-write: metadata op commits, then `dispatch_post_op_impl` fires webhooks; email/push/`bus.publish` via fire-and-forget `tokio::spawn` | `crates/server/src/events.rs:107, 127-133, 143-152, 176-178` |
| Bus is 100% in-memory (DashMap handlers, optional in-memory EventStore, bounded in-memory DLQ default 1000) | `crates/event-bus/src/bus.rs:87-93, 150-176, 185-192` |
| `replay()` re-invokes ALL handlers for stored events with **no dedup/watermark** — crash-recovery replay duplicates side effects | `bus.rs:322-341` |
| Webhook retry: 5 attempts, 2 s base / 300 s cap backoff, from `tokio::spawn` | `server-automation/src/webhooks.rs:14-15, 126` |
| `Schedule{cron}` workflows have no persistent scheduler; restart silently skips ticks; `MAX_WORKFLOWS=100` | `workflows.rs:16, 31-38, 219-267` |
| Metadata DB: rusqlite bundled SQLite; Postgres via optional `pg` feature | `server/Cargo.toml:16,70` |

**Consequence:** a crash between DB commit and `bus.publish` loses the event; a crash mid-handler leaves partial work; replay duplicates notifications. GDPR-erasure and backup jobs are exactly the side effects that must be neither duplicated nor dropped.

---

## YP-2: Executive Summary

### Problem Statement

Ferro commits metadata and dispatches effects as independent steps with no transaction spanning them — the canonical dual-write problem. The 2026 machine-checked result (Andreakis) proves an **impossibility**: no source-only recovery policy can achieve exactly-once; durability must move *into* the source's transaction, with sink-side dedup records making recovery computable.

### Decision (the maximally strong special case)

> **Do not adopt Temporal/DBOS/NATS for 50 users on one host. Implement a transactional outbox in the existing SQLite database — one box, one transaction.**

- **Schema:** `outbox(id INTEGER PRIMARY KEY, seq INTEGER UNIQUE, event_type, payload BLOB, created_at, not_before, attempts, lease_until, status)` + `handler_watermarks(handler TEXT PK, last_seq)`. `PRAGMA journal_mode=WAL, busy_timeout=5000, synchronous=NORMAL`.
- **Atomicity rule:** metadata write and `INSERT INTO outbox` share one `rusqlite` transaction. Refactor `dispatch_post_op_impl` to enqueue, not dispatch.
- **Relay:** single tokio task, poll 1 s, batch ≤ 100, claim via `UPDATE ... SET lease_until = now+60 WHERE id IN (...) AND status='pending' RETURNING *` (crash-safe lease). At-least-once.
- **Idempotency:** every effect keyed `job_id:handler`; Stripe-style dedup table, **7-day retention** (≥ 3× the retry horizon: 8 attempts × backoff 2ⁿ capped 300 s ≈ 10.6 min/event). GDPR-erasure and backup jobs keyed and reentrant.
- **Backoff:** keep 2 s/300 s; attempts 5 → **8**; then persistent DLQ **table** (replace in-memory DLQ) with admin re-drive API.
- **Replay fix:** `replay()` consults `handler_watermarks`, dispatches only `seq > watermark`.
- **Cron:** persist `next_due_at`; on boot coalesce missed ticks into one run (catch-up), jitter ±30 s.
- **Audit coupling:** `outbox.seq` shares the transaction with the audit-chain position — "every committed op appears in the audit chain" becomes checkable.
- **API:** accept `Idempotency-Key` (≤ 255 chars) on all mutating endpoints (Stripe semantics; the IETF draft is expired, use as design guidance).

### Guarantees

- An enqueued event is dispatched iff its transaction committed; no aborted-transaction event is ever dispatched.
- Every external effect occurs exactly once within the dedup lifetime R.
- Replay never re-applies events below a handler's watermark.

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| Dual-write problem | Committing state and publishing an event as separate non-atomic steps. |
| Transactional outbox | Storing the event in the same transaction as the state change; a relay publishes later. |
| At-least-once | Delivery guarantee allowing duplicates; requires idempotent consumers. |
| Exactly-once effect | The *observable* effect occurs once — achieved as at-least-once delivery + keyed dedup (no broker does better). |
| Handler watermark | Per-consumer high-water sequence number; replay dispatches only above it. |
| Lease | Time-bounded claim on a job row, making crashes safe (expired lease = re-claimable). |
| DLQ | Dead-letter queue; here a persistent table with re-drive. |
| Dedup lifetime R | Window for which idempotency records are retained; duplicates possible only outside R. |

---

## YP-4: Theoretical Foundation

### THM-DUALWRITE-IMPOSSIBLE (inherits Andreakis 2026)

Any recovery policy for Ferro that inspects only SQLite's committed state after a crash cannot achieve exactly-once effect dispatch: there exist two reachable post-crash states indistinguishable to SQLite where any policy duplicates an effect in one or loses it in the other. **Consequence:** durability must move into SQLite (outbox), with sink-side dedup records making recovery computable.

### THM-OUTBOX-ATOMICITY

If (a) metadata write `W` and outbox insert `E` share one transaction; (b) the relay is the sole dispatcher — then: `E` is dispatched iff `W` committed; no event generated by an aborted transaction is ever dispatched.

### THM-EXACTLY-ONCE-EFFECT

At-least-once delivery + per-effect idempotency keys `K` with dedup retention `R ≥ max recovery horizon` + reentrant handlers ⟹ every external effect occurs exactly once within lifetime `R`.

### THM-DEDUP-BOUND

With dedup window `R`, duplicates are possible only for effects whose last delivery attempt precedes `now − R`; the guarantee is exactly R-scoped (bounded-state version of Andreakis's bounded-deduplication result). 7 days is a *policy choice*, not a solution — document it.

### THM-REPLAY-NO-DUPLICATION

Replay guarded by per-handler watermark `w_h` (dispatch only `seq > w_h`; commit watermark after handler returns) delivers each event to each handler at most once per replay invocation and never re-applies events below `w_h` — eliminating the duplicate-effect bug at `bus.rs:322-341`.

---

## YP-5: Algorithm Specification

### ALG-OUTBOX-001: Enqueue-in-Transaction

```
fn commit_with_events(tx, metadata_op, events):
    apply(metadata_op)                      # in tx
    seq := next_seq(tx)
    for e in events: INSERT INTO outbox(seq, ...)        # in tx
    INSERT INTO audit_chain(pos = seq, ...)                 # in tx — coupled positions
    COMMIT
```

### ALG-RELAY-001: Leased Dispatch Loop

```
loop every 1 s:
  batch := UPDATE outbox SET lease_until = now()+60, status='dispatching'
           WHERE status='pending' AND not_before <= now()
           AND id IN (SELECT id FROM outbox WHERE ... ORDER BY seq LIMIT 100)
           RETURNING *
  for e in batch:
    for h in handlers(e.event_type):
      if e.seq <= watermark(h): continue
      deliver(e, h) with retries (2s base, 300s cap, 8 attempts)
      on final failure: INSERT INTO dlq(...); status='dead'
      else: status='done'; watermark(h) := e.seq
  reclaim: rows with expired lease → status='pending'
```

### ALG-IDEMPOTENCY-001: API Effect Keys

```
Idempotency-Key header (≤255 chars, V4-UUID recommended) on all mutating endpoints
first request: execute, store (key → status+body) in dedup table, 7-day retention
replay with same key: return stored response
same key + different params: 409 (Stripe semantics)
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_durability.toml` — sections `events_*`.

Critical vectors: crash between commit and dispatch → event delivered after restart; crash mid-handler → no duplicate after lease reclaim; replay with watermark → zero duplicates; aborted transaction → no event; 8-attempt backoff → DLQ; boot coalesce of missed cron ticks → exactly one run; Idempotency-Key replay/409 semantics.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_durability.toml`.

Summary: no side effect MAY be dispatched outside the outbox relay (`DC-EVT-001`); metadata writes and outbox inserts MUST share one transaction (`DC-EVT-002`); all handlers MUST be idempotent and keyed (`DC-EVT-003`); DLQ MUST be persistent (`DC-EVT-004`); transactions MUST stay < 10 ms (relay starvation) (`DC-EVT-005`); webhook receivers receive `Event-Id` and are documented to dedup (`DC-EVT-006`).

---

## YP-8: Bibliography

1. Andreakis, A. "Machine-Checked Dual-Write Recovery from a Commit Log." arXiv:`2608.00501v5` (2026-08-01); Isabelle/HOL artifact DOI `10.5281/zenodo.22700396`. Source-only recovery impossibility; outbox/CDC analysis; bounded-dedup limits. [VERIFIED]
2. **Negative result:** "Putting the Dual Writes in the Same Box" (attributed Bruni/Fischlin/Peters, PODC 2019) — zero hits on arXiv/Crossref/web; do not cite. [VERIFIED ABSENCE]
3. Richardson, C. "Pattern: Transactional outbox." microservices.io (retrieved 2026-10). At-least-once relay ⇒ consumers must be idempotent; companions: polling publisher, transaction log tailing. [VERIFIED — pattern doc]
4. **Negative result:** "Exactly-Once Semantics Are Possible" (Bentley, van Renesse, ICDCS 2016) — not found; do not cite without locating a copy. [VERIFIED ABSENCE]
5. Jena, J., Dalal, S. "The Idempotency-Key HTTP Header Field." `draft-ietf-httpapi-idempotency-key-header-07` — **expired I-D** (rev 2025-10-15), not an RFC; design guidance only. [VERIFIED]
6. Stripe, "Idempotent requests" (docs.stripe.com): key ≤ 255 chars, ≥ 24 h pruning, parameter-mismatch error, V4-UUID recommended. [VERIFIED — vendor doc]
7. Carbone, P., Fóra, G., Ewen, S., Haridi, S., Tzoumas, K. "Lightweight Asynchronous Snapshots for Distributed Dataflows." arXiv:`1506.08603` (2015). Async barrier snapshotting — Flink's exactly-once core. [VERIFIED]
8. Chandy, K. M., Lamport, L. "Distributed Snapshots: Determining Global States of Distributed Systems." *ACM TOCS* 7(1):63–75, 1985. [VERIFIED]
9. Burckhardt, S., Gillum, C., Justo, D., Kallas, K., McMahon, C., Meiklejohn, C. S. "Serverless Workflows with Durable Functions and Netherite." arXiv:`2103.00033` (2021). Event-sourced state + deterministic replay; best verified academic anchor for durable execution. [VERIFIED]
10. **[UNCERTAIN]** DBOS Transact / PVLDB 2022 — engineering claims widely documented; no verified identifier this session; verify before citing.
11. Nextcloud admin manual, *Background jobs* (Server 36): cron/systemd 5-min ticks; AJAX mode default; webcron ≈ 288 jobs/day; maintenance window. At-least-once-by-rerun, no per-consumer dedup. [VERIFIED]
12. Temporal / AWS Step Functions / Kafka EOS (KIP-129, KIP-447) — vendor docs, not verified this session. [UNCERTAIN]
13. In-repo: `crates/audit-log/src/chain.rs` hash chain — outbox seq and audit positions must couple (see YP-LIFECYCLE-COMPLIANCE-001). [VERIFIED]
