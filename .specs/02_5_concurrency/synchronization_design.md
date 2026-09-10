# Phase 2.5: Synchronization Design

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | CONC-SD-001                                |
| Domain       | Concurrency Engineering                    |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | Concurrency Engineer (Phase 2.5)           |
| Date         | 2026-04-18                                 |
| Compliance   | IEEE 1016-2009 (Software Design Descriptions) |

---

## 1. MetadataLock — Path-Based Read/Write Locking

### Purpose

Provide fine-grained, deadlock-free locking for metadata operations (COPY, MOVE, DELETE, MKCOL) that need to coordinate access to overlapping path hierarchies.

### Design

```rust
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

pub struct MetadataLock {
    inner: Arc<RwLock<BTreeMap<String, Arc<RwLock<()>>>>>,
}

impl MetadataLock {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub async fn acquire_read(&self, path: &str) -> OwnedRwLockReadGuard<()> {
        let guard = self.inner.read().await;
        let lock = guard
            .entry(path.to_owned())
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone();
        drop(guard);
        lock.read_owned().await
    }

    pub async fn acquire_write(&self, path: &str) -> OwnedRwLockWriteGuard<()> {
        let guard = self.inner.read().await;
        let lock = guard
            .entry(path.to_owned())
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone();
        drop(guard);
        lock.write_owned().await
    }

    pub async fn acquire_sorted(
        &self,
        paths: &[String],
        exclusive: bool,
    ) -> Vec<OwnedRwLockReadGuard<()>> {
        let mut sorted: Vec<&String> = paths.iter().collect();
        sorted.sort();

        let mut guards = Vec::with_capacity(sorted.len());
        for path in sorted {
            if exclusive {
                guards.push(self.acquire_write(path).await);
            } else {
                guards.push(self.acquire_read(path).await);
            }
        }
        guards
    }
}
```

### Properties

| Property                     | Value                                                |
|------------------------------|------------------------------------------------------|
| Granularity                  | Per-path                                            |
| Concurrency                  | Concurrent readers on same path; exclusive writer    |
| Deadlock-free guarantee      | Via `acquire_sorted` (BTreeMap lexicographic order)  |
| Memory overhead              | O(unique locked paths) × sizeof(Arc<RwLock<()>>)     |
| Lock cleanup                 | Lazy: entries remain in BTreeMap after release (small memory cost) |

### Usage Pattern

```rust
async fn move_path(storage: &StorageEngine, src: &str, dst: &str) -> Result<()> {
    let mut paths = vec![src.to_owned(), dst.to_owned()];
    paths.sort();

    let _locks = storage.metadata_lock.acquire_sorted(&paths, true).await;

    let hash = storage.metadata.resolve_path(src).await?.ok_or(NotFound)?;
    storage.metadata.add_reference(dst, &hash).await?;
    storage.metadata.remove_reference(src).await?;

    Ok(())
}
```

---

## 2. Connection Pool Configuration

### SQLx (PostgreSQL / LibSQL)

```rust
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(num_cpus::get() * 4)
    .min_connections(2)
    .acquire_timeout(Duration::from_secs(5))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

| Parameter        | Value              | Rationale                                           |
|------------------|--------------------|------------------------------------------------------|
| max_connections  | CPU cores × 4      | Balance concurrency vs. DB load                      |
| min_connections  | 2                  | Keep warm connections for low-traffic periods        |
| acquire_timeout  | 5s                 | Fail fast under pool exhaustion; trigger backpressure |
| idle_timeout     | 10 min             | Reclaim unused connections                           |
| max_lifetime     | 30 min             | Prevent stale connections after DB restart           |

### object_store (S3 / GCS / Azure / Local FS)

`object_store` manages its own connection pool internally. No explicit configuration needed for connection pooling. Relevant tunables:

| Parameter              | Value  | Rationale                                      |
|------------------------|--------|-------------------------------------------------|
| multipart_chunk_size   | 64 MB  | Balance memory usage vs. upload parallelism     |
| multipart_threshold    | 128 MB | Files above this use multipart upload          |
| retry_max_attempts     | 3      | Transient failure recovery                     |

### Cedar Authorizer

Single `Arc<Authorizer>` instance. No pooling needed — Cedar evaluation is CPU-bound and synchronous. The `Authorizer` is `Send + Sync` and safe for concurrent use.

---

## 3. Async Task Design

### Task Categories

| Category     | Executor                        | Use Case                                         |
|--------------|---------------------------------|--------------------------------------------------|
| I/O-bound    | Tokio default (multi-thread)    | HTTP handlers, DB queries, object store ops       |
| CPU-bound    | Tokio blocking pool             | SHA-256 hashing, XML parsing (> 64 KB)           |
| WASM         | Dedicated thread pool           | Plugin execution (Phase 5)                       |
| Background   | Tokio spawn (detached)          | Lock expiry cleanup, JWKS refresh, GC sweep       |

### I/O-Bound Tasks (Default)

All async operations (storage, metadata, lock management) run on Tokio's default multi-threaded executor. The default worker thread count is `num_cpus`.

```rust
// Default behavior — no annotation needed
async fn handle_propfind(state: State<WebDavState>, req: Request) -> Response {
    let items = state.storage.list(prefix).await?;  // I/O-bound, runs on Tokio
    // ...
}
```

### CPU-Bound Tasks (spawn_blocking)

SHA-256 hashing and large XML parsing must not block the async executor.

```rust
use tokio::task;

async fn compute_hash(content: &[u8]) -> ContentHash {
    let content = content.to_vec();
    task::spawn_blocking(move || {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&content);
        format!("{:x}", hasher.finalize())
    })
    .await
    .expect("hash task panicked")
}
```

**Blocking thread pool configuration:**

```rust
#[tokio::main]
async fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .max_blocking_threads(num_cpus::get() * 2)
        .enable_all()
        .build()
        .unwrap();
    // ...
}
```

| Parameter            | Value           | Rationale                                        |
|----------------------|-----------------|--------------------------------------------------|
| worker_threads       | num_cpus        | Match CPU cores for I/O-bound work               |
| max_blocking_threads | num_cpus × 2    | Sufficient headroom for hashing + XML parsing    |

### WASM Execution (Phase 5)

WASM plugins run on a dedicated thread pool with resource limits:

```rust
use std::sync::Semaphore;

pub struct WasmExecutor {
    pool: tokio::task::ThreadPool,
    memory_semaphore: Arc<Semaphore>,
    fuel_limit: u64,
}

impl WasmExecutor {
    pub fn new(max_threads: usize, max_memory_bytes: usize, fuel_limit: u64) -> Self {
        Self {
            pool: tokio::task::Builder::new()
                .max_blocking_threads(max_threads)
                .thread_name("wasm-worker")
                .build()
                .expect("failed to create WASM thread pool"),
            memory_semaphore: Arc::new(Semaphore::new(max_memory_bytes / (64 * 1024))),
            fuel_limit,
        }
    }

    pub async fn execute(&self, module: &WasmModule, input: &[u8]) -> Result<Vec<u8>> {
        let _permit = self.memory_semaphore.acquire().await?;
        let module = module.clone();
        let input = input.to_vec();
        let fuel = self.fuel_limit;

        self.pool.spawn_blocking(move || {
            let mut store = Store::new(&module.engine(), input);
            store.add_fuel(fuel)?;
            module.instantiate(&mut store)?.call_main(&mut store)
        }).await?
    }
}
```

| Parameter         | Default Value   | Constraint                                  |
|-------------------|-----------------|---------------------------------------------|
| max_threads       | 4               | Isolate WASM execution from async executor  |
| max_memory_bytes  | 256 MB total    | REQ-WASM-003 (sandboxing)                   |
| fuel_limit        | 10_000_000      | ~100ms equivalent CPU time (REQ-WASM-004)   |
| wall_clock_timeout| 30s             | Absolute timeout regardless of fuel         |

### Background Tasks

```rust
// Lock expiry cleanup — runs every 60s
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        lock_manager.cleanup_expired("/").await;
    }
});

// JWKS refresh — runs every 24h
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        let _ = oidc_validator.refresh_jwks();
    }
});

// GC sweep — runs every 1h
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        storage.gc_orphaned_objects().await;
    }
});
```

All background tasks use `tokio::spawn` (not `spawn_blocking`) because they are I/O-bound (database queries, HTTP fetches).

---

## 4. Backpressure Mechanisms

### Tower Layers

| Layer                  | Configuration           | Purpose                                           |
|------------------------|-------------------------|----------------------------------------------------|
| ConcurrencyLimitLayer  | max = 10_000            | Bound in-flight requests                           |
| TimeoutLayer           | timeout = 30s           | Prevent hung requests from consuming resources     |
| LoadShedLayer          | overload threshold      | Reject requests with 503 when overloaded           |

```rust
use tower::ServiceBuilder;
use tower_http::limit::ConcurrencyLimitLayer;
use tower::timeout::TimeoutLayer;

let app = Router::new()
    .route("/*path", get(handle_get).put(handle_put))
    .layer(
        ServiceBuilder::new()
            .layer(ConcurrencyLimitLayer::new(10_000))
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
            .into_inner(),
    )
    .with_state(state);
```

### Graceful Shutdown

```rust
let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

tokio::select! {
    _ = axum::serve(listener, app.into_make_service()) => {}
    _ = shutdown_rx.recv() => {
        tracing::info!("shutdown signal received");
    }
}

// Drain in-flight requests
tokio::time::timeout(Duration::from_secs(30), drain_handler).await;
```

---

## 5. Synchronization Primitive Summary

| Primitive                      | Used By          | Purpose                              | Deadlock-Free |
|--------------------------------|------------------|--------------------------------------|---------------|
| `Arc`                          | All components   | Shared ownership without mutation    | Yes           |
| `DashMap`                      | LockManager      | Concurrent lock table access         | Yes (shard-level) |
| `RwLock<BTreeMap<...>>`        | MetadataLock     | Path-based read/write coordination   | Yes (sorted acquisition) |
| `Arc<RwLock<PolicySet>>`       | CedarAuthorizer  | Hot-reloadable policy set            | Yes (clone-before-eval) |
| `Arc<RwLock<Jwks>>`            | OidcValidator    | Cached JWKS key set                  | Yes (infrequent writes) |
| SQLx transaction               | StorageEngine    | Atomic multi-row metadata operations | Yes (DB deadlock detection + retry) |
| `Semaphore`                    | WasmExecutor     | Memory/capacity limiting             | Yes (no nesting) |
| `tokio::sync::broadcast`       | Shutdown         | Coordinated graceful shutdown        | Yes           |
