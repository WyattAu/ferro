# Phase 2.5: Thread Safety Analysis

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | CONC-TSA-001                               |
| Domain       | Concurrency Engineering                    |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | Concurrency Engineer (Phase 2.5)           |
| Date         | 2026-04-18                                 |
| Compliance   | IEEE 1016-2009 (Software Design Descriptions) |

---

## 1. StorageEngine (COMP-STORAGE-001)

### Status: SAFE

**Shared state analysis:**

| Field            | Type                    | Thread Safety Mechanism                    |
|------------------|-------------------------|--------------------------------------------|
| `object_store`   | Backend trait object    | Internally synchronized (Arc + async I/O)  |
| `metadata_store` | SQLx PgPool / LibSqlPool| Connection pooling (thread-safe by design) |
| `sha2` hasher    | Stateless function      | No shared state; pure computation           |

**CPU-bound concern — SHA-256 hashing:**

SHA-256 computation (`sha2::Sha256`) is CPU-bound and blocks the Tokio async executor if run directly on the async runtime. The `put_content` path (ALG-CAS-PUT-001) hashes the entire content stream before storage.

- **Mitigation**: Wrap SHA-256 computation in `tokio::task::spawn_blocking` to offload to Tokio's blocking thread pool (default: 512 threads).
- **Streaming variant**: For large files (> multipart_threshold), compute hash in chunks via `spawn_blocking` with a channel-based streaming hasher to avoid buffering the entire payload.
- **Verification**: REQ-STOR-006 (concurrent uploads) requires correct serialization; the CAS dedup path (DC-DEDUP-001) uses `PutMode::Create` which is atomic at the backend level.

**No shared mutable state:**

`StorageEngine` holds only `Arc` references to its dependencies. All mutation goes through the metadata database (serialized by PostgreSQL MVCC) or the object store backend (serialized per-object). Therefore, `StorageEngine` is thread-safe by design — no `Mutex`, `RwLock`, or atomic primitives needed within the struct itself.

**Risk — Connection pool exhaustion:**

Under high concurrency (e.g., 10K concurrent PROPFIND with per-resource `head` calls), the SQLx connection pool may become a bottleneck.

- **Mitigation**: Configure `max_connections` to `num_cpus * 4` (default). Use `PgPoolOptions::acquire_timeout()` to fail fast rather than queue indefinitely.
- **Monitoring**: Expose `pool.size()` and `pool.num_idle()` as Prometheus metrics.

---

## 2. LockManager (COMP-LOCK-002)

### Status: CONDITIONAL

**Shared mutable state:**

The lock table is an in-memory `HashMap<LockToken, LockInfo>` with concurrent read/write access from all request handlers. This requires explicit synchronization.

**Primary synchronization primitive:**

```
Arc<RwLock<HashMap<LockToken, LockInfo>>>
```

Or, for better concurrent read performance under low write contention:

```
Arc<DashMap<LockToken, LockInfo>>
```

**ADR-003 (from BP-WEBDAV-HANDLER-001) selects DashMap** for sub-microsecond lock acquisition. DashMap provides shard-level locking (default 16 shards), allowing concurrent access to different lock entries without global contention.

**Lock acquisition flow (ALG-LOCK-001 ACQUIRE):**

1. Read lock on the lock table (or DashMap shard).
2. Check for conflicting locks on the target path and all ancestors (for `Depth::Infinity`).
3. If no conflict: acquire write lock, insert `LockInfo`, release.
4. If conflict: return `LockError::ConflictingLock` (HTTP 423).

**Risks and mitigations:**

| Risk                              | Impact                     | Mitigation                                           |
|-----------------------------------|----------------------------|------------------------------------------------------|
| Lock manager becomes bottleneck   | Latency spike under high contention | Shard by path prefix (e.g., first 2 path segments map to shard index) |
| Depth:infinity lock check is O(depth) | Linear scan of ancestors  | Cache parent lock existence in a secondary index: `HashMap<String, Vec<LockToken>>` per path |
| Lock refresh during acquisition   | Race condition on timeout  | Atomic compare-and-swap on `LockInfo.timeout` field  |
| Lock table grows unbounded         | Memory leak from expired locks | Periodic cleanup task (`cleanup_expired`) evicts entries past `timeout + grace_period` |
| Single-node only                  | Cannot scale horizontally  | Acceptable for Phase 1-2; distributed lock coordination deferred to Phase 3+ |

**Condition for SAFE classification:**

The LockManager is SAFE provided:
1. All lock table mutations go through DashMap's atomic API.
2. Lock acquisition checks are performed within a consistent snapshot (DashMap's `read` method provides this per-shard).
3. Expired locks are cleaned up before conflict checks.

---

## 3. CedarAuthorizer (COMP-CEDAR-002)

### Status: SAFE

**Shared state analysis:**

| State Component         | Type                          | Thread Safety                                   |
|-------------------------|-------------------------------|-------------------------------------------------|
| `authorizer`            | `Arc<Authorizer>`             | Cedar's Authorizer is internally synchronized  |
| `entity_store`          | `Arc<EntityStore>`            | Read-heavy; mutations via `add_entity`/`remove_entity` |
| `active_policy_set`     | `Arc<RwLock<PolicySet>>`      | Atomic swap on policy reload                   |
| `entity_cache`          | `Arc<RwLock<HashMap<...>>>`   | Principal attribute cache                      |

**Policy evaluation path (ALG-CEDAR-EVAL-001):**

1. Read current `PolicySet` via `Arc<RwLock>::read()`.
2. Load required entities from `EntityStore`.
3. Call `authorizer.is_authorized(request, policy_set, entities)`.
4. Return `AuthResponse`.

**Risk — Read lock held during evaluation:**

If the policy set is large (up to 10K policies per DC-AUTH-CEDAR-MAX-001), holding the read lock during `is_authorized()` could block a concurrent `load_policies()` call that needs a write lock.

- **Mitigation**: Clone the `PolicySet` before evaluation. Cedar's `PolicySet` uses `Arc` internally for policy sharing, making `clone()` O(1) — it copies only the Arc pointers, not the policy AST.
- **Pattern**:
  ```rust
  let policy_set = self.active_policy_set.read().await.clone();
  drop(read_guard);
  let response = self.authorizer.is_authorized(principal, action, resource, context, &policy_set);
  ```

**Risk — Entity store contention:**

Entity loading (for principal attributes, resource attributes) hits the entity store on every request.

- **Mitigation**: Per-request entity cache with TTL (DC-AUTH-PRINCIPAL-CACHE-001). Cache key: `(EntityUid, last_modified_version)`.
- **Cache invalidation**: Evict on `add_entity` / `remove_entity` calls.

---

## 4. OidcValidator (COMP-AUTH-001)

### Status: SAFE

**Shared state analysis:**

| State Component | Type                       | Thread Safety                             |
|-----------------|----------------------------|-------------------------------------------|
| JWKS cache      | `Arc<RwLock<JwksKeySet>>`  | Read-heavy; write on refresh (24h TTL)    |
| OIDC config     | Immutable after startup    | No synchronization needed                 |
| Nonce store     | Delegated to SessionManager| See SessionManager analysis                |

**Token validation (ALG-OIDC-VALIDATE-001) is a pure function** over the token string and cached JWKS. No shared mutable state is mutated during validation.

**JWKS refresh is the only write path:**

- Background task refreshes every 24h (DC-AUTH-JWKS-001).
- On-demand refresh when `kid` is unknown.
- Write lock is held only during the `RwLock` swap, which is O(number of keys) ≈ O(10).

**Risk — Clock skew handling:**

Token validation compares `exp`/`nbf` against the current time. Different threads may observe slightly different `Instant::now()` values, but the 60-second clock skew tolerance (DC-AUTH-CLOCK-001) makes this irrelevant.

---

## 5. Axum Server (COMP-WEBDAV-001 + Router)

### Status: SAFE

**Axum's concurrency model:**

- Each inbound HTTP request spawns an independent Tokio task.
- `State` is cloned via `Arc` — zero-copy reference counting.
- The Tower middleware chain is composable: each layer processes the request asynchronously, releasing the executor between `.await` points.

**WebDavState shared components:**

```rust
pub struct WebDavState {
    pub storage: Arc<dyn StorageEngine>,         // SAFE (see §1)
    pub lock_manager: Arc<LockManager>,           // CONDITIONAL (see §2)
    pub property_store: Arc<dyn PropertyStore>,   // SAFE (delegates to storage)
    pub config: WebDavConfig,                     // Immutable after startup
}
```

**Risk — Slow middleware blocks the executor:**

Any `.await`-free synchronous computation in middleware (e.g., path normalization, XML parsing of small bodies) blocks the Tokio worker thread for the duration.

- **Mitigation**: CPU-bound middleware work (XML parsing of request bodies > 64 KB) should use `tokio::task::spawn_blocking`.
- **Timeout layer**: Add `tower::timeout::TimeoutLayer` with a 30s default to prevent hung requests from consuming worker threads indefinitely.
- **Concurrency limit**: `tower::limit::ConcurrencyLimitLayer` to bound the number of simultaneous in-flight requests.

---

## 6. SessionManager (COMP-SESSION-003)

### Status: CONDITIONAL

**Shared state analysis:**

| State Component     | Type                  | Thread Safety                              |
|---------------------|-----------------------|--------------------------------------------|
| Session store       | Redis / in-memory     | Redis: thread-safe client; In-memory: requires `DashMap` or `RwLock<HashMap>` |
| Refresh token state | Redis                 | Atomic SET NX with TTL                     |

**Condition for SAFE classification:**

- **Redis-backed**: SAFE — Redis serializes all operations. The `reqwest::Client` used to communicate with Redis is `Clone + Send + Sync`.
- **In-memory**: CONDITIONAL — requires `DashMap<SessionId, Session>` with atomic expiry enforcement via a background Tokio task.

---

## Summary Table

| Component              | Status      | Key Synchronization                          | Risks                                        |
|------------------------|-------------|----------------------------------------------|----------------------------------------------|
| StorageEngine          | SAFE        | None (delegates to pool/backend)             | Connection pool exhaustion                    |
| LockManager            | CONDITIONAL | DashMap (sharded lock table)                 | Bottleneck under high lock contention         |
| CedarAuthorizer        | SAFE        | `Arc<RwLock<PolicySet>>` + clone-before-eval | Entity store contention                       |
| OidcValidator          | SAFE        | `Arc<RwLock<Jwks>>`                          | None significant                              |
| Axum Server            | SAFE        | Arc<State> cloning                           | Slow middleware blocking executor             |
| SessionManager (Redis) | SAFE        | Redis atomic operations                      | None                                         |
| SessionManager (mem)   | CONDITIONAL | DashMap + background expiry                  | Memory growth, expiry race                    |
