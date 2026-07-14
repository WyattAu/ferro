# ADR-005: Concurrency Model

**Status:** Accepted
**Date:** 2026-07-12
**Deciders:** Wyatt (Sole developer)

## Context

Ferro is a high-throughput file server handling concurrent WebDAV requests, CalDAV sync, federation webhooks, WASM worker execution, WebSocket notifications, and background tasks (retention policies, antivirus scanning, indexing). The server must handle 184+ req/s sustained (per 24h soak test) with P99 < 200ms for local operations.

The codebase uses async Rust throughout, with tokio as the runtime. Key concurrency challenges include:
- SQLite access (single-writer, concurrent readers via WAL mode)
- Filesystem I/O (blocking operations in async context)
- WASM execution (sandboxed but CPU-bound)
- Shared state (DashMap in-memory caches, AppState across 58 crates)
- Lock management (WebDAV exclusive locks with timeout and refresh)

## Decision

### Async Runtime

**Tokio** is the sole async runtime for all server components.

- Runtime flavor: `multi-thread` (default, work-stealing scheduler)
- Features: `full` (all features enabled across workspace)
- Rationale: Tokio is the de facto Rust async runtime; the `tokio` dependency is already at `1` across the workspace with `full` features

```toml
# Workspace Cargo.toml
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
```

### Thread Pool Sizing

| Pool | Size | Purpose |
|------|------|---------|
| Tokio runtime | `num_cpus` (auto) | Async task scheduling (default work-stealing) |
| Blocking pool | `512` max threads | File I/O, SQLite writes, antivirus TCP, compression |
| WASM spawn_blocking | `64` max threads | WASM worker execution (CPU-bound, fuel-limited) |

Configuration:
```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(num_cpus::get())
    .max_blocking_threads(512)
    .enable_all()
    .build()?;
```

### Blocking Operations in Async Context

All blocking operations MUST use `tokio::task::spawn_blocking()`:

| Operation | Current Pattern | Required Pattern |
|-----------|----------------|------------------|
| File read/write | `tokio::fs` (already async) | `tokio::fs` or `spawn_blocking` for sync fs |
| SQLite queries | `spawn_blocking` (SQLx async) | `spawn_blocking` (SQLx already does this) |
| Compression | `spawn_blocking` | `spawn_blocking` |
| Thumbnail generation | `spawn_blocking` | `spawn_blocking` |
| WASM execution | `spawn_blocking` with fuel | `spawn_blocking` (already implemented) |
| ClamAV TCP | `spawn_blocking` | `spawn_blocking` (already implemented) |

**Rule:** No `std::thread::spawn()` in server code. All blocking work goes through Tokio's blocking pool to maintain backpressure.

### Backpressure Strategy

| Layer | Mechanism | Configuration |
|-------|-----------|---------------|
| Connection acceptance | Tokio TCP listener (bounded by OS) | `SO_BACKLOG = 128` |
| Request body | `--max-body-size` flag | Default: 1GB |
| Concurrent requests | Rate limiter middleware (token bucket) | `--rate-limit` default: 10,000 req/min/IP |
| Blocking tasks | Tokio blocking pool (`max_blocking_threads`) | 512 max |
| WASM workers | Fuel limit + timeout + memory cap | 1B fuel, 30s timeout, 64MB memory |
| File locks | `DashMap` with timeout-based eviction | Lock timeout: configurable |
| SQLite writes | WAL mode (concurrent readers, serial writers) | Single-writer serialized via SQLx pool |
| Memory | No explicit limiter; relies on Tokio + OS | OOM killer as last resort |

### Shared State Model

```
AppState (defined in crates/server/src/state.rs)
├── StorageEngine (Arc<dyn StorageEngine>) -- trait object, behind RwLock where needed
├── DbHandle (Option<DbHandle>) -- SQLx pool (PgPool or SqlitePool)
├── SearchEngine (Option<SearchEngine>) -- Tantivy index, behind RwLock
├── DashMap<String, ...> -- in-memory caches (session, rate limit, locks)
├── BroadcastSender -- WebSocket event broadcast
├── Config (Arc<ServerConfig>) -- immutable after startup
└── FederationState (Arc<FederationState>) -- federation keys, client
```

**State access pattern (per ADR-001 trait-based decomposition):**
```rust
// Handler signature
async fn handler<S: HasStorage + HasDb>(State(state): State<S>) -> impl IntoResponse { ... }

// Access storage through trait
let storage = state.storage();
```

**Lock policy:**
- Prefer `DashMap` (sharded RwLock) for concurrent caches
- Use `std::sync::Mutex` only for short-held, non-async-critical sections (documented in `SAFETY` comments per TD-016)
- Avoid `tokio::sync::Mutex` (use only when lock must be held across `.await` points)
- Never hold locks across I/O boundaries

### Concurrency Testing

| Test Type | Tool | Frequency |
|-----------|------|-----------|
| Race condition detection | `cargo miri test` | Monthly |
| Concurrency stress | Custom integration tests (100 concurrent clients) | Per release |
| Lock contention | `tracing` span timing in debug builds | On-demand |
| Deadlock detection | Tokio console (tokio-console crate) | Development only |
| Soak testing | 24h sustained load (184 req/s) | Per major release |

## Consequences

### Positive
- Tokio work-stealing scheduler efficiently handles mixed I/O and compute workloads
- `spawn_blocking` prevents blocking the async runtime (verified in existing codebase)
- DashMap provides lock-free reads for hot-path cache lookups
- WAL-mode SQLite handles concurrent reads without contention
- Backpressure via rate limiting prevents overload

### Negative
- `max_blocking_threads = 512` may be excessive for low-traffic deployments; wastes memory on idle threads
- DashMap is in-memory only; restart loses all cached state (documented in TD-002)
- `std::sync::Mutex` in async code requires careful SAFETY documentation (TD-016)
- WASM execution is single-threaded per worker (fuel limit prevents parallelism)

### Risks
- Tokio blocking pool exhaustion under extreme load (512 threads saturated)
- SQLite single-writer bottleneck under heavy write load (mitigated by WAL + write coalescing)
- DashMap memory growth under memory pressure (no eviction policy, relies on OS OOM)
- Deadlock risk from nested locks (DashMap + std::sync::Mutex) -- mitigated by lock ordering discipline

## Alternatives Considered

### Async-std
- **Description:** Alternative async runtime to Tokio
- **Pros:** Simpler API, smaller binary size
- **Cons:** Smaller ecosystem, fewer integrations (SQLx, axum, tower all assume Tokio), no work-stealing scheduler
- **Why Rejected:** Axum, SQLx, and the entire tower ecosystem are Tokio-native; switching would require rewriting all async infrastructure

### Single-Threaded Tokio
- **Description:** Use `current_thread` runtime instead of `multi_thread`
- **Pros:** Simpler reasoning, no cross-thread synchronization, smaller memory footprint
- **Cons:** Cannot utilize multiple cores; one blocked task blocks everything; insufficient for 184 req/s sustained
- **Why Rejected:** Performance requirements demand multi-core utilization

### Rayon for Parallelism
- **Description:** Use Rayon's thread pool for CPU-bound work instead of Tokio's blocking pool
- **Pros:** Work-stealing for CPU-bound tasks, better fit for parallel iterators
- **Cons:** Introduces a second thread pool; Rayon tasks don't integrate with Tokio's backpressure; adds complexity
- **Why Rejected:** Tokio's blocking pool is sufficient for current CPU-bound workloads (WASM, compression, thumbnails); Rayon can be added later if needed

## Related ADRs
- [ADR-008](ADR-008-server-crate-decomposition.md) -- Server Crate Decomposition (trait-based state access enables concurrency-safe handlers)
- [ADR-004](ADR-004-security-review-process.md) -- Security Review Process (concurrency bugs are security-relevant)

## References
- Tokio runtime configuration: https://tokio.rs/tokio/tutorial/runtime
- Tokio blocking pool: https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.max_blocking_threads
- DashMap: https://docs.rs/dashmap/
- SQLx SQLite WAL mode: https://docs.rs/sqlx/ (SQLite WAL mode enabled by default)
- Ferro soak test: 24h soak, 184 req/s, zero errors (2026-07-04)
- Ferro load test: 69 req/s, 20 VUs, 0% failure
