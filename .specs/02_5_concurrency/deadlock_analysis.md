# Phase 2.5: Deadlock Analysis

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | CONC-DA-001                                |
| Domain       | Concurrency Engineering                    |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | Concurrency Engineer (Phase 2.5)           |
| Date         | 2026-04-18                                 |
| Methodology  | Wait-For Graph (WFG) + Resource Ordering   |
| Compliance   | IEEE 1016-2009 (Software Design Descriptions) |

---

## 1. Definitions

### Wait-For Graph (WFG)

A **Wait-For Graph** G = (T, E) where:
- T = {t₁, t₂, ..., tₙ} is the set of active tasks (threads/async tasks).
- E = {(tᵢ, tⱼ)} is a directed edge meaning task tᵢ is waiting for a resource held by task tⱼ.

**Deadlock theorem**: A deadlock exists iff the WFG contains a directed cycle.

### Resource Ordering Protocol

A system is deadlock-free if all tasks acquire resources in a **global total order**. If every task acquires locks in order R₁ < R₂ < ... < Rₙ, then no circular wait can form.

---

## 2. LockManager Deadlock Scenarios

### Scenario 1: Parent-Child Lock Ordering Inversion

**Description**: Two clients attempt to lock resources in opposite hierarchical order.

```
Client A: LOCK /docs/          (parent)
Client B: LOCK /docs/file.txt  (child)

Timeline:
  t₁: Client A acquires lock on /docs/
  t₂: Client B acquires lock on /docs/file.txt
  t₃: Client A needs lock on /docs/file.txt (depth:infinity propagation)
  t₄: Client B needs lock on /docs/ (check ancestors for conflicts)
  → Client A waits for Client B, Client B waits for Client A
  → CYCLE: t₃ → t₄ → t₃
```

**WFG**:
```
Task A ──waits──→ Task B ──waits──→ Task A
```

**Resolution**: Enforce **canonical path ordering**. All locks within a single operation must be acquired in lexicographic order of the normalized path string.

```
/docs/ < /docs/file.txt  (lexicographic: "/" < "f")
```

If Client A holds `/docs/` and needs `/docs/file.txt`, it acquires them in order `/docs/` then `/docs/file.txt`. If Client B holds `/docs/file.txt` and needs `/docs/`, it must acquire `/docs/` first — but Client A holds it, so Client B waits. No cycle.

**Implementation rule**: Before acquiring multiple locks, sort the target paths lexicographically and acquire in that order.

### Scenario 2: Lock Refresh During Concurrent Acquisition

**Description**: A client refreshes an existing lock while another client is checking conflicts for a new lock acquisition.

```
Client A: LOCK /docs/file.txt (refresh existing lock)
Client B: LOCK /docs/file.txt (new acquisition, checks conflicts)

Timeline:
  t₁: Client B reads lock table, sees no active lock (A's lock just expired)
  t₂: Client A sends refresh request, updates timeout
  t₃: Client B proceeds with acquisition, finds no conflict
  → Two clients hold locks on the same resource
```

**This is not a deadlock** but a **lost update / race condition**. It violates PROP-WEBDAV-002 (LOCK Exclusivity).

**Resolution**: Use atomic compare-and-swap for lock refresh:

```rust
fn refresh_lock(&self, token: &LockToken, new_timeout: Instant) -> Result<LockInfo, LockError> {
    self.lock_table.compute_if_exists(token, |_, info| {
        if info.is_expired() {
            None  // Lock has expired; remove it
        } else {
            Some(LockInfo { timeout: new_timeout, ..info.clone() })
        }
    }).ok_or(LockError::NoSuchLock)
}
```

DashMap's `compute_if_exists` provides atomic read-modify-write semantics per shard, preventing the lost update.

### Scenario 3: Depth:Infinity Lock Propagation Creates Implicit Lock Set

**Description**: A `Depth::Infinity` lock on `/docs/` implicitly locks all descendants. If two clients attempt to lock overlapping subtrees, the implicit lock sets may intersect in complex ways.

```
Client A: LOCK /docs/reports/  Depth:Infinity
Client B: LOCK /docs/           Depth:Infinity

Both need to check the entire subtree for conflicts.
```

**Resolution**: Instead of acquiring individual locks on every descendant, the LockManager maintains a **path index**:

```
HashMap<String, Vec<LockToken>>  // path → locks that cover this path
```

When checking conflicts for a new lock on path P:
1. Check direct locks on P.
2. Check all ancestor paths for `Depth::Infinity` locks.
3. Check all descendant paths for any locks (they conflict with a parent lock).

This is an O(depth) check per lock acquisition, not O(N) where N is total lock count.

**Deadlock prevention**: The path index is read-only during conflict checks (using DashMap's read API). Write operations (insert/remove lock) use DashMap's write API. Since DashMap's shard-level locking is non-reentrant and we never hold multiple shard write locks simultaneously, no deadlock can occur.

---

## 3. StorageEngine Deadlock Scenarios

### Scenario 4: Concurrent MOVE Operations with Overlapping Paths

**Description**: Two clients issue MOVE operations that involve overlapping source/destination paths.

```
Client A: MOVE /docs/reports/2025/ → /docs/archive/2025/
Client B: MOVE /docs/reports/2025/q1.pdf → /docs/archive/2025/q1.pdf

Timeline:
  t₁: Client A begins MOVE, acquires metadata lock on /docs/reports/2025/
  t₂: Client B begins MOVE, acquires metadata lock on /docs/reports/2025/q1.pdf
  t₃: Client A needs to update /docs/archive/2025/ (destination)
  t₄: Client B needs to update /docs/archive/2025/q1.pdf (destination)
```

**WFG (if locks acquired in inconsistent order)**:
```
Task A ──waits for──> Task B (holds /docs/archive/2025/q1.pdf metadata)
Task B ──waits for──> Task A (holds /docs/reports/2025/ metadata)
```

**Resolution**: MOVE operation acquires metadata locks on source and destination in canonical order.

```
sort(["/docs/archive/2025/", "/docs/reports/2025/"])
= ["/docs/archive/2025/", "/docs/reports/2025/"]
```

Both Client A and Client B sort their lock targets before acquisition. Since the canonical order is total, no circular wait can form.

### Scenario 5: Copy-Then-Delete Race in MOVE

**Description**: The MOVE operation is implemented as copy metadata + delete source metadata. If the delete fails, the system must rollback the copy (per BP-WEBDAV-HANDLER-001 §BP-5 Route: MOVE, error 424).

```
MOVE /a → /b:
  1. Begin transaction
  2. INSERT metadata for /b (copy from /a)
  3. DELETE metadata for /a
  4. Commit transaction

If step 3 fails:
  424 Failed Dependency → rollback step 2
```

**This is not a deadlock** but requires **transactional atomicity**.

**Resolution**: Use SQLx transaction:

```rust
async fn move_path(&self, src: &UserPath, dst: &UserPath) -> Result<()> {
    let mut tx = self.pool.begin().await?;

    let hash = tx.resolve_path(src).await?.ok_or(StorageError::NotFound)?;

    tx.add_reference(dst, &hash).await?;
    tx.remove_reference(src).await?;
    tx.commit().await?;

    Ok(())
}
```

PostgreSQL's MVCC ensures serializability. The database itself prevents deadlocks via its own deadlock detection (which aborts one transaction with error 40P01).

**Mitigation for DB deadlocks**: Retry the transaction up to 3 times with exponential backoff when SQLSTATE 40P01 is received.

### Scenario 6: Concurrent PUT to Same Path (REQ-STOR-006)

**Description**: Two clients PUT different content to the same path simultaneously.

```
Client A: PUT /docs/file.txt  (content: "hello")
Client B: PUT /docs/file.txt  (content: "world")

Both compute SHA-256, both call put_content, both call add_reference.
```

**This is not a deadlock**. The CAS layer (PROP-CAS-001) ensures dedup correctness via `PutMode::Create`. The metadata layer resolves via last-writer-wins within a database transaction.

**Resolution**: `add_reference` uses SQL `INSERT ... ON CONFLICT (path) DO UPDATE SET content_hash = $2, modified_at = NOW()` (upsert). PostgreSQL serializes the conflicting writes; one succeeds, the other retries.

---

## 4. Cross-Component Deadlock Scenarios

### Scenario 7: LockManager + StorageEngine Interaction

**Description**: A handler acquires a WebDAV lock, then performs a storage operation that internally acquires a metadata lock. Meanwhile, another handler performs a storage operation first, then tries to acquire a WebDAV lock.

```
Handler A: check_lock(/path) → put(path, data)
Handler B: put(/path, data) → check_lock(/path)
```

**Lock acquisition order**:
- Handler A: LockManager → StorageEngine
- Handler B: StorageEngine → LockManager

**WFG**:
```
Handler A ──waits for StorageEngine lock──→ Handler B
Handler B ──waits for LockManager lock──→ Handler A
```

**Resolution**: Enforce a **component-level lock ordering protocol**:

```
1. LockManager locks (WebDAV locks)
2. StorageEngine locks (metadata locks)
3. CedarAuthorizer (no locks, read-only evaluation)
```

All handlers MUST acquire locks in this order. The Axum middleware chain naturally enforces this: auth/cedar middleware runs before handlers, and handlers check WebDAV locks before performing storage operations.

**Invariant**: No handler acquires a StorageEngine metadata lock before checking/acquiring a WebDAV lock.

---

## 5. Resource Ordering Protocol

### Canonical Ordering Definition

All lockable resources in Ferro are ordered as follows:

```
Level 0: Cedar PolicySet (highest — rarely locked, never held long)
Level 1: JWKS cache (write lock on refresh)
Level 2: LockManager entries (WebDAV locks)
Level 3: MetadataStore rows (database-level locks)
Level 4: object_store objects (backend-level locks)
```

### Acquisition Rules

1. **Single-resource operations** (GET, HEAD, DELETE): Acquire only the needed resource. No ordering concern.

2. **Multi-resource operations** (COPY, MOVE, LOCK Depth:Infinity):
   - Sort all resource identifiers within the same level lexicographically.
   - Acquire across levels in ascending order (Level 0 → Level 4).
   - Never acquire a lower-level resource before releasing a higher-level one unless all lower-level resources in the set are already acquired.

3. **Deadlock proof sketch**:
   - Assume a deadlock exists. Then the WFG contains a cycle t₁ → t₂ → ... → tₙ → t₁.
   - Each edge tᵢ → tᵢ₊₁ means tᵢ waits for a resource held by tᵢ₊₁.
   - Since all tasks acquire resources in the same total order, if tᵢ holds R_a and waits for R_b, then R_a < R_b in the ordering.
   - Following the cycle: R₁ < R₂ < ... < Rₙ < R₁, which is a contradiction.
   - Therefore, no deadlock can exist. QED.

---

## 6. Summary

| Scenario | Type        | Components Involved      | Resolution                                    |
|----------|-------------|--------------------------|-----------------------------------------------|
| 1        | Deadlock    | LockManager              | Canonical path ordering (lexicographic)       |
| 2        | Race        | LockManager              | Atomic CAS for lock refresh                   |
| 3        | Deadlock    | LockManager (depth:inf)  | Path index + DashMap shard-level locking      |
| 4        | Deadlock    | StorageEngine (MOVE)     | Canonical path ordering + metadata transaction |
| 5        | Atomicity   | StorageEngine (MOVE)     | SQLx transaction with rollback                |
| 6        | Race        | StorageEngine (PUT)      | Upsert + last-writer-wins                    |
| 7        | Deadlock    | LockManager + StorageEngine | Component-level lock ordering protocol      |
