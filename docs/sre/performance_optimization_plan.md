# Ferro Performance Optimization Plan

> **Version:** 1.0  
> **Author:** SRE Team  
> **Created:** 2026-07-12  
> **Status:** Proposed  
> **Duration:** 24 weeks (6 months)  
> **Target:** Production-ready performance for 10K+ concurrent users

---

## Executive Summary

This plan addresses performance bottlenecks across Ferro's Rust-based storage server. The codebase currently uses `AtomicU64` arrays for metrics (no labels), `DashMap`-based caching with O(n) LRU eviction, and manual Prometheus string formatting. This plan optimizes each layer systematically over 24 weeks with measurable targets.

---

## Current Architecture Analysis

### Metrics System (Current)
- **File:** `crates/server/src/prometheus_metrics.rs` — manual `format!` string building
- **File:** `crates/observability/src/registry.rs` — `Vec<(String, String, MetricEntry)>` with `RwLock`
- **File:** `crates/observability/src/exporter.rs` — `String::write!` per metric on every scrape
- **Issue:** No label support, O(n) registry scan on scrape, string allocation per request

### Cache Layer (Current)
- **File:** `crates/cache/src/cache.rs` — `DashMap<K, CacheEntry<V>>`
- **File:** `crates/cache/src/lru.rs` — `Vec<K>` + `HashMap<K, usize>` with O(n) index rebuild on eviction
- **Issue:** `record_access` does `order.remove(pos)` + iterates all indices to decrement — O(n)

### Storage Layer (Current)
- **File:** `crates/core/src/` — SQLite via `sqlx`, object_store for S3/GCS/Azure
- **File:** `crates/server/src/state/mod.rs:34` — `Arc<dyn StorageEngine>` trait object
- **Issue:** No connection pooling metrics, no query plan caching, no read-ahead

### HTTP Layer (Current)
- **File:** `crates/server/src/routes.rs:100` — single `build_router` function
- **File:** `crates/server/src/state/mod.rs:74-81` — atomic counters for request tracking
- **Issue:** No per-route latency breakdown, no request coalescing, no body streaming optimization

---

## Phase 1: Foundation & Benchmarking (Weeks 1-4)

### Week 1: Establish Performance Baseline

**Objective:** Create reproducible benchmarks and baseline measurements.

#### Changes

**1.1 Create benchmark harness baseline**
- **File:** `crates/server/benches/throughput.rs` (exists — extend)
- **File:** `crates/server/benches/latency.rs` (exists — extend)
- **New File:** `crates/server/benches/critical_paths.rs`
- **New File:** `crates/benchmarks/src/lib.rs` — shared benchmark utilities

```rust
// crates/server/benches/critical_paths.rs
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

fn bench_webdav_put_small(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("webdav_put");
    for size in &[1024, 64*1024, 1024*1024] {
        group.bench_with_input(
            BenchmarkId::new("in_memory", size),
            size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let state = create_test_app_state();
                        let app = create_test_router(state);
                        let body = generate_test_body(size);
                        make_request(&app, "PUT", "/bench.txt", body).await;
                    })
                })
            },
        );
    }
    group.finish();
}

fn bench_concurrent_read(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_read");
    for clients in &[10, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::new("dashmap", clients),
            clients,
            |b, &clients| {
                b.iter(|| {
                    rt.block_on(async {
                        let cache = create_populated_cache(10_000);
                        let mut handles = Vec::with_capacity(clients);
                        for i in 0..clients {
                            let cache = cache.clone();
                            handles.push(tokio::spawn(async move {
                                cache.get(&format!("key_{}", i % 10_000));
                            }));
                        }
                        for h in handles { h.await.unwrap(); }
                    })
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_webdav_put_small, bench_concurrent_read);
criterion_main!(benches);
```

**1.2 Add performance regression CI gate**
- **New File:** `.github/workflows/perf-regression.yml`
- **New File:** `scripts/bench_compare.sh` — compares against baseline JSON

```yaml
# .github/workflows/perf-regression.yml
name: Performance Regression
on:
  pull_request:
    paths:
      - 'crates/server/**'
      - 'crates/cache/**'
      - 'crates/core/**'
      - 'crates/observability/**'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run benchmarks
        run: |
          cargo bench --package ferro-server --bench throughput -- --output-format bencher | tee throughput.txt
          cargo bench --package ferro-server --bench latency -- --output-format bencher | tee latency.txt
      - name: Check regression
        run: scripts/bench_compare.sh baseline.json throughput.txt 5
```

**1.3 Instrument critical paths with tracing spans**
- **File:** `crates/server/src/handlers.rs` — add `#[tracing::instrument]` to top handlers
- **File:** `crates/server/src/storage.rs` — instrument storage operations
- **File:** `crates/cache/src/cache.rs` — instrument get/set with span events

#### Benchmark Targets (Week 1)
| Metric | Current | Target |
|--------|---------|--------|
| 1KB PUT throughput (seq) | Baseline | Measure |
| 1MB GET throughput (100 concurrent) | Baseline | Measure |
| Cache get latency (p99) | Baseline | Measure |
| Prometheus scrape latency | Baseline | Measure |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Benchmark flakiness on shared CI | High | Medium | Pin CPU governor, use `taskset`, run 10 iterations |
| Measurement noise from background tasks | Medium | Low | Use `criterion` statistical analysis, 95% CI |

#### Rollback Procedure
- Delete `critical_paths.rs` and `.github/workflows/perf-regression.yml`
- Revert tracing instrument additions (purely additive, zero risk)

---

### Week 2: Metrics Registry Optimization

**Objective:** Replace `Vec`-based registry with label-aware, lock-free structure.

#### Changes

**2.1 Redesign MetricsRegistry with label support**
- **File:** `crates/observability/src/registry.rs` — replace `Vec<(String, String, MetricEntry)>` with `DashMap<String, MetricEntry>`
- **File:** `crates/observability/src/counter.rs` — add `labels: DashMap<String, AtomicU64>` for labeled counters
- **File:** `crates/observability/src/histogram.rs` — add label dimension support

```rust
// crates/observability/src/registry.rs (new design)
use dashmap::DashMap;
use std::sync::Arc;

pub struct MetricsRegistry {
    counters: DashMap<String, Arc<LabeledCounter>>,
    gauges: DashMap<String, Arc<LabeledGauge>>,
    histograms: DashMap<String, Arc<LabeledHistogram>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: DashMap::new(),
            gauges: DashMap::new(),
            histograms: DashMap::new(),
        }
    }

    pub fn counter(&self, name: &str, help: &str) -> Arc<LabeledCounter> {
        self counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(LabeledCounter::new(name, help)))
            .clone()
    }

    pub fn histogram(
        &self,
        name: &str,
        help: &str,
        buckets: Vec<f64>,
    ) -> Arc<LabeledHistogram> {
        self.histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(LabeledHistogram::new(name, help, buckets)))
            .clone()
    }
}
```

**2.2 Add metric family aggregation**
- **File:** `crates/observability/src/exporter.rs` — group metrics by name, aggregate labels
- **New File:** `crates/observability/src/encoding.rs` — zero-allocation Prometheus text encoder

```rust
// crates/observability/src/encoding.rs
use std::fmt::Write;

pub struct PrometheusEncoder<'a> {
    output: &'a mut String,
}

impl<'a> PrometheusEncoder<'a> {
    pub fn new(output: &'a mut String) -> Self {
        Self { output }
    }

    pub fn encode_counter(&mut self, name: &str, help: &str, value: u64, labels: &[( &str, &str)]) {
        let _ = writeln!(self.output, "# HELP {} {}", name, help);
        let _ = writeln!(self.output, "# TYPE {} counter", name);
        if labels.is_empty() {
            let _ = writeln!(self.output, "{}_total {}", name, value);
        } else {
            let label_str = encode_labels(labels);
            let _ = writeln!(self.output, "{}_total{{{}}} {}", name, label_str, value);
        }
    }
}

fn encode_labels(labels: &[(&str, &str)]) -> String {
    let mut s = String::with_capacity(64);
    for (i, (k, v)) in labels.iter().enumerate() {
        if i > 0 { s.push(','); }
        s.push_str(k);
        s.push_str("=\"");
        s.push_str(v);
        s.push('\"');
    }
    s
}
```

**2.3 Add metrics tests**
- **File:** `crates/observability/src/tests/mod.rs` — add label correctness tests, encode/decode roundtrip

#### Benchmark Targets (Week 2)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Prometheus scrape (100 metrics) | Baseline | - | < 1ms |
| Metric registration (1000 metrics) | Baseline | - | < 10ms |
| Counter increment (labeled) | Baseline | - | < 50ns |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Breaking existing metric names | High | Critical | Run `promtool check metrics` on output |
| Label cardinality explosion | Medium | High | Add `max_labels` guard per metric family |

#### Rollback
- Keep old `export_prometheus` function behind `#[cfg(feature = "legacy-metrics")]`
- Feature gate new registry: `features = ["labeled-metrics"]`

---

### Week 3: LRU Eviction Optimization

**Objective:** Replace O(n) LRU with O(1) doubly-linked list + HashMap.

#### Changes

**3.1 Replace Vec-based LRU with linked-list LRU**
- **File:** `crates/cache/src/lru.rs` — complete rewrite
- **New File:** `crates/cache/src/lru_linked.rs` — O(1) LRU implementation

```rust
// crates/cache/src/lru_linked.rs
use std::collections::HashMap;
use std::ptr::NonNull;
use parking_lot::Mutex;

struct Node<K> {
    key: K,
    prev: Option<NonNull<Node<K>>>,
    next: Option<NonNull<Node<K>>>,
}

pub struct LruCache<K: Eq + std::hash::Hash> {
    map: HashMap<K, NonNull<Node<K>>>,
    head: Option<NonNull<Node<K>>>,
    tail: Option<NonNull<Node<K>>>,
    capacity: usize,
    len: usize,
}

impl<K: Eq + std::hash::Hash + Clone> LruCache<K> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            head: None,
            tail: None,
            capacity,
            len: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> bool {
        if let Some(&mut node_ptr) = self.map.get_mut(key) {
            unsafe { self.move_to_front(node_ptr); }
            true
        } else {
            false
        }
    }

    pub fn insert(&mut self, key: K) -> Option<K> {
        if let Some(&node_ptr) = self.map.get(&key) {
            unsafe { self.move_to_front(node_ptr); }
            return None;
        }

        let evicted = if self.len >= self.capacity {
            self.evict_tail()
        } else {
            None
        };

        let node = Box::new(Node { key: key.clone(), prev: None, next: None });
        let node_ptr = Box::into_raw_non_null(node);

        unsafe {
            (*node_ptr.as_ptr()).next = self.head;
            if let Some(head) = self.head {
                (*head.as_ptr()).prev = Some(node_ptr);
            }
            self.head = Some(node_ptr);
            if self.tail.is_none() {
                self.tail = Some(node_ptr);
            }
        }

        self.map.insert(key, node_ptr);
        self.len += 1;
        evicted
    }

    unsafe fn move_to_front(&mut self, mut node_ptr: NonNull<Node<K>>) {
        if self.head == Some(node_ptr) { return; }

        let node = &mut *node_ptr.as_ptr();
        if let Some(prev) = node.prev {
            (*prev.as_ptr()).next = node.next;
        }
        if let Some(next) = node.next {
            (*next.as_ptr()).prev = node.prev;
        }
        if self.tail == Some(node_ptr) {
            self.tail = node.prev;
        }

        node.prev = None;
        node.next = self.head;
        if let Some(head) = self.head {
            (*head.as_ptr()).prev = Some(node_ptr);
        }
        self.head = Some(node_ptr);
    }

    fn evict_tail(&mut self) -> Option<K> {
        let tail_ptr = self.tail?;
        let tail = &*tail_ptr.as_ptr();
        let key = tail.key.clone();

        self.tail = tail.prev;
        if let Some(prev) = self.tail {
            (*prev.as_ptr()).next = None;
        } else {
            self.head = None;
        }

        self.map.remove(&key);
        self.len -= 1;
        drop(Box::from_raw(tail_ptr.as_ptr()));
        Some(key)
    }
}
```

**3.2 Add cache metrics**
- **File:** `crates/cache/src/stats.rs` — add hit_rate, eviction_rate, memory_pressure gauges
- **File:** `crates/cache/src/cache.rs` — wire metrics to observability registry

```rust
// crates/cache/src/stats.rs (enhanced)
pub struct CacheMetrics {
    pub hits: Arc<AtomicU64>,
    pub misses: Arc<AtomicU64>,
    pub evictions: Arc<AtomicU64>,
    pub size_bytes: Arc<AtomicU64>,
    pub entry_count: Arc<AtomicU64>,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed);
        let m = self.misses.load(Ordering::Relaxed);
        if h + m == 0 { 0.0 } else { h as f64 / (h + m) as f64 }
    }
}
```

**3.3 Add cache benchmark**
- **New File:** `crates/cache/benches/cache_bench.rs`

#### Benchmark Targets (Week 3)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| LRU record_access (10K entries) | O(n) ~10μs | O(1) | < 100ns |
| Cache get (contended, 100 threads) | Baseline | - | < 200ns p99 |
| Cache eviction burst (1000 evictions) | Baseline | - | < 500μs |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Unsafe pointer bugs in LRU | Medium | Critical | Add `#[cfg(test)]` invariant checks, Miri in CI |
| Memory leak from un-freed nodes | Low | High | Add Drop impl, run leak sanitizer |

#### Rollback
- Keep old `lru.rs` behind `#[cfg(feature = "legacy-lru")]`
- Feature gate: `cache = ["lru-linked"]` (default) vs `cache = ["lru-vec"]`

---

### Week 4: Connection Pool & Database Optimization

**Objective:** Optimize SQLite/PostgreSQL connection handling and query patterns.

#### Changes

**4.1 Add connection pool metrics**
- **File:** `crates/server/src/connection_pool.rs` — instrument pool stats
- **New File:** `crates/server/src/db_metrics.rs` — DB-specific metrics

```rust
// crates/server/src/db_metrics.rs
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct DbMetrics {
    pub queries_total: Arc<AtomicU64>,
    pub query_duration_sum_us: Arc<AtomicU64>,
    pub connections_active: Arc<AtomicU64>,
    pub connections_idle: Arc<AtomicU64>,
    pub slow_queries_total: Arc<AtomicU64>,  // > 100ms
}

impl DbMetrics {
    pub fn record_query(&self, duration_us: u64) {
        self.queries_total.fetch_add(1, Ordering::Relaxed);
        self.query_duration_sum_us.fetch_add(duration_us, Ordering::Relaxed);
        if duration_us > 100_000 {
            self.slow_queries_total.fetch_add(1, Ordering::Relaxed);
        }
    }
}
```

**4.2 Add query plan caching for SQLite**
- **File:** `crates/core/src/` — add prepared statement cache

**4.3 Optimize bulk operations**
- **File:** `crates/server/src/batch.rs` — use transactions for batch ops
- **File:** `crates/server/src/bulk.rs` — parallel bulk with bounded concurrency

#### Benchmark Targets (Week 4)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Single SELECT latency | Baseline | - | < 50μs |
| Bulk INSERT (1000 rows) | Baseline | - | < 10ms |
| Connection pool exhaustion recovery | Baseline | - | < 1s |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Prepared statement cache invalidation | Medium | Medium | TTL-based eviction, schema change detection |
| Transaction deadlock under load | Low | High | Add timeout, instrument deadlock count |

#### Rollback
- Feature gate query caching: `sqlite-cache` vs `sqlite-direct`

---

## Phase 2: Core Optimizations (Weeks 5-12)

### Week 5-6: Zero-Copy Request Processing

**Objective:** Eliminate unnecessary body copies in upload/download paths.

#### Changes

**5.1 Streaming upload with backpressure**
- **File:** `crates/server/src/upload.rs` — replace `Bytes::from` with `Stream`-based body
- **File:** `crates/server/src/streaming.rs` — add adaptive chunk sizing

```rust
// crates/server/src/streaming.rs
pub struct AdaptiveChunker {
    min_chunk: usize,
    max_chunk: usize,
    target_latency_ms: u64,
    current_chunk: usize,
}

impl AdaptiveChunker {
    pub fn new(min: usize, max: usize, target_ms: u64) -> Self {
        Self {
            min_chunk: min,
            max_chunk: max,
            target_latency_ms: target_ms,
            current_chunk: min,
        }
    }

    pub fn adjust(&mut self, actual_latency_ms: u64) {
        if actual_latency_ms < self.target_latency_ms / 2 {
            self.current_chunk = (self.current_chunk * 2).min(self.max_chunk);
        } else if actual_latency_ms > self.target_latency_ms * 2 {
            self.current_chunk = (self.current_chunk / 2).max(self.min_chunk);
        }
    }

    pub fn chunk_size(&self) -> usize {
        self.current_chunk
    }
}
```

**5.2 Range request optimization**
- **File:** `crates/server/src/storage.rs` — add `read_range` with minimal allocation
- **File:** `crates/server-storage-ops/src/range_get.rs` — optimize partial reads

**5.3 Response body compression**
- **File:** `crates/server/src/routes.rs` — add per-route compression config

#### Benchmark Targets (Weeks 5-6)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| 10MB upload throughput | Baseline | - | > 500 MB/s |
| 10MB download throughput | Baseline | - | > 800 MB/s |
| Range request (1KB of 1GB) | Baseline | - | < 1ms |
| Memory per concurrent upload | Baseline | - | < 64KB |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Backpressure deadlock | Medium | High | Bounded channel with timeout |
| Adaptive chunker oscillation | Low | Low | EMA smoothing, min/max bounds |

#### Rollback
- Feature gate streaming: `streaming-upload` (default on)
- Keep fallback `Bytes::from` path behind `legacy-upload`

---

### Week 7-8: Object Store Optimization

**Objective:** Optimize S3/GCS/Azure backend performance.

#### Changes

**7.1 Request coalescing**
- **File:** `crates/core/src/` — add dedup layer for concurrent identical GETs

```rust
// crates/core/src/request_coalescer.rs
use std::collections::HashMap;
use tokio::sync::broadcast;
use parking_lot::Mutex;

pub struct RequestCoalescer<V: Clone> {
    in_flight: Mutex<HashMap<String, broadcast::Sender<V>>>,
}

impl<V: Clone + Send + 'static> RequestCoalescer<V> {
    pub async fn coalesce<F, Fut>(&self, key: &str, f: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        let mut in_flight = self.in_flight.lock();

        if let Some(tx) = in_flight.get(key) {
            let mut rx = tx.subscribe();
            drop(in_flight);
            return rx.recv().await.unwrap();
        }

        let (tx, _) = broadcast::channel(1);
        in_flight.insert(key.to_string(), tx.clone());
        drop(in_flight);

        let result = f().await;

        let _ = self.in_flight.lock().remove(key);
        let _ = tx.send(result.clone());
        result
    }
}
```

**7.2 Multipart upload optimization**
- **File:** `crates/server/src/upload.rs` — parallel part uploads with `futures::join_all`

**7.3 Presigned URL caching**
- **File:** `crates/server/src/presigned.rs` — cache presigned URLs with TTL

#### Benchmark Targets (Weeks 7-8)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Concurrent GET (same key) | 100 reqs | 1 req to S3 | 99% reduction |
| Multipart 100MB upload | Baseline | - | < 5s |
| Presigned URL generation | Baseline | - | < 10μs |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Coalescer memory leak (dropped key) | Medium | High | TTL eviction, max in-flight limit |
| Presigned URL stale cache | Low | Medium | TTL < AWS min expiry, refresh on 403 |

#### Rollback
- Feature gate coalescer: `request-coalesce`
- Presigned caching behind `presigned-cache`

---

### Week 9-10: WASM Runtime Optimization

**Objective:** Reduce WASM dispatch latency and memory overhead.

#### Changes

**9.1 WASM pre-instantiation pool**
- **File:** `crates/core/src/wasm.rs` — pool pre-instantiated modules

```rust
// crates/core/src/wasm/pool.rs
pub struct ModulePool {
    modules: DashMap<String, Arc<wasmtime::Module>>,
    max_modules: usize,
}

impl ModulePool {
    pub async fn get_or_compile(&self, engine: &wasmtime::Engine, path: &str) -> Result<Arc<wasmtime::Module>> {
        if let Some(m) = self.modules.get(path) {
            return Ok(m.clone());
        }
        let wasm_bytes = tokio::fs::read(path).await?;
        let module = wasmtime::Module::new(engine, &wasm_bytes)?;
        let module = Arc::new(module);
        self.modules.insert(path.to_string(), module.clone());
        Ok(module)
    }
}
```

**9.2 Fuel-based execution limits**
- **File:** `crates/core/src/wasm.rs` — adjust fuel per worker type

**9.3 Worker serialization optimization**
- **File:** `crates/server/src/workers.rs` — use `rmp-serde` for binary serialization

#### Benchmark Targets (Weeks 9-10)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| WASM cold start | Baseline | - | < 5ms |
| WASM dispatch latency | Baseline | - | < 1ms |
| WASM memory per instance | Baseline | - | < 4MB |

#### Risk Assessment
| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Module pool memory growth | Medium | Medium | LRU eviction, max_modules config |
| Fuel exhaustion false positives | Low | High | Per-worker-type fuel budgets |

#### Rollback
- Feature gate pool: `wasm-pool`
- Keep single-instance path as fallback

---

### Week 11-12: Memory & Allocation Optimization

**Objective:** Reduce heap allocations in hot paths.

#### Changes

**11.1 Arena allocation for request processing**
- **New File:** `crates/server/src/arena.rs` — bump allocator for request lifetime

**11.2 String interning for paths**
- **New File:** `crates/server/src/path_interner.rs` — intern frequently accessed paths

```rust
// crates/server/src/path_interner.rs
use dashmap::DashSet;
use std::sync::Arc;

pub struct PathInterner {
    strings: DashSet<Arc<str>>,
}

impl PathInterner {
    pub fn intern(&self, s: &str) -> Arc<str> {
        if let Some(existing) = self.strings.get(s) {
            return existing.clone();
        }
        let arc: Arc<str> = Arc::from(s);
        self.strings.insert(arc.clone());
        arc
    }
}
```

**11.3 Bytes buffer pool**
- **New File:** `crates/server/src/buffer_pool.rs` — recycle `BytesMut` buffers

#### Benchmark Targets (Weeks 11-12)
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Allocations per request | Baseline | - | -50% |
| RSS at 1000 concurrent | Baseline | - | -20% |
| P99 latency jitter | Baseline | - | -30% |

#### Rollback
- All additions are opt-in via feature flags
- Keep default path unchanged

---

## Phase 3: Advanced Optimizations (Weeks 13-18)

### Week 13-14: HTTP/2 & Connection Optimization

**Objective:** Enable HTTP/2 multiplexing and optimize connection handling.

#### Changes

**13.1 HTTP/2 support**
- **File:** `crates/server/src/main.rs` — add `hyper-util` HTTP/2 support
- **File:** `crates/server/src/tls.rs` — configure ALPN

**13.2 Connection keepalive tuning**
- **File:** `crates/server/src/routes.rs` — add connection pool middleware

#### Benchmark Targets
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| HTTP/2 multiplexed streams | N/A | - | 100+ per connection |
| Connection setup latency | Baseline | - | < 5ms |

---

### Week 15-16: Storage Tiering & Caching

**Objective:** Implement hot/warm/cold storage tiers.

#### Changes

**15.1 Tiered storage metadata**
- **New File:** `crates/server/src/storage_tier.rs` — metadata for tier classification

**15.2 Read-ahead cache**
- **File:** `crates/server-integrations/src/read_cache.rs` — add sequential detection + prefetch

```rust
// crates/server-integrations/src/read_cache.rs (enhanced)
pub struct ReadAheadCache {
    inner: TimedCache<String, Bytes>,
    sequential_threshold: usize,
    prefetch_size: usize,
    access_pattern: DashMap<String, AccessPattern>,
}

struct AccessPattern {
    sequential_count: usize,
    last_offset: u64,
}

impl ReadAheadCache {
    pub fn maybe_prefetch(&self, path: &str, offset: u64, size: u64) -> Option<Vec<u8>> {
        let mut pattern = self.access_pattern.entry(path.to_string()).or_insert(AccessPattern {
            sequential_count: 0,
            last_offset: 0,
        });

        if offset == pattern.last_offset + size {
            pattern.sequential_count += 1;
        } else {
            pattern.sequential_count = 0;
        }
        pattern.last_offset = offset;

        if pattern.sequential_count >= self.sequential_threshold {
            // Trigger background prefetch
            Some(vec![0u8; self.prefetch_size]) // placeholder
        } else {
            None
        }
    }
}
```

#### Benchmark Targets
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Sequential read throughput | Baseline | - | +30% |
| Cache hit rate (production workload) | Baseline | - | > 85% |

---

### Week 17-18: Distributed Optimization

**Objective:** Optimize CRDT sync and federation performance.

#### Changes

**17.1 CRDT merge optimization**
- **File:** `crates/crdt/src/` — batch merge operations

**17.2 Federation sync batching**
- **File:** `crates/server-infra/src/federation_sync.rs` — batch sync operations

#### Benchmark Targets
| Metric | Before | After | Target |
|--------|--------|-------|--------|
| CRDT merge (1000 ops) | Baseline | - | < 50ms |
| Federation sync throughput | Baseline | - | +50% |

---

## Phase 4: Production Hardening (Weeks 19-24)

### Week 19-20: Load Testing & Tuning

**Objective:** Validate all optimizations under realistic load.

#### Changes

**19.1 k6 load test expansion**
- **File:** `benchmarks/k6/webdav-load.js` — add realistic workloads
- **File:** `benchmarks/k6/rest-load.js` — add mixed read/write patterns

**19.2 Capacity planning model**
- **New File:** `scripts/capacity_model.py` — predict resource needs

#### Benchmark Targets
| Metric | Target |
|--------|--------|
| 10K concurrent users | p99 < 200ms |
| 1M files indexed | search < 100ms |
| 100GB storage | memory < 2GB |

---

### Week 21-22: Monitoring & Alerting

**Objective:** Production-grade observability for all optimizations.

#### Changes

**21.1 Performance dashboards**
- **File:** `monitoring/grafana/dashboards/performance.json` — new dashboard

**21.2 SLO-based alerts**
- **File:** `monitoring/prometheus/alerts.yml` — add latency SLO alerts

```yaml
# Performance SLO alerts
- alert: LatencySLOViolation
  expr: |
    histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m])) > 0.5
  for: 10m
  labels:
    severity: critical
  annotations:
    summary: "p99 latency SLO violation"
    description: "p99 is {{ $value }}s, SLO is 500ms"

- alert: CacheHitRateLow
  expr: ferro_read_cache_hits_total / (ferro_read_cache_hits_total + ferro_read_cache_misses_total) < 0.7
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "Cache hit rate below 70%"
    description: "Current hit rate: {{ $value | humanizePercentage }}"
```

---

### Week 23-24: Documentation & Rollout

**Objective:** Document all changes and roll out to production.

#### Changes

**23.1 Performance guide**
- **New File:** `docs/performance-tuning.md` — operator guide

**23.2 Migration runbook**
- **New File:** `docs/runbooks/performance-migration.md` — step-by-step

**23.3 Gradual rollout**
- Canary deployment to 10% traffic
- Monitor for 48 hours
- Full rollout

---

## Success Metrics Summary

| Category | Metric | Baseline | Week 12 | Week 24 |
|----------|--------|----------|---------|---------|
| Throughput | 1KB PUT rps | TBD | +50% | +100% |
| Throughput | 1MB GET rps | TBD | +30% | +60% |
| Latency | p99 request | TBD | -30% | -50% |
| Memory | RSS at 1K conns | TBD | -20% | -40% |
| Cache | Hit rate | TBD | > 80% | > 90% |
| Observability | Metric scrape | TBD | < 5ms | < 2ms |
| WASM | Cold start | TBD | < 10ms | < 5ms |

---

## Rollback Strategy

Every optimization is behind a feature flag:

```toml
[features]
default = ["perf-v2"]
perf-v2 = [
    "lru-linked",
    "labeled-metrics",
    "streaming-upload",
    "request-coalesce",
    "wasm-pool",
    "presigned-cache",
]
legacy-lru = []
legacy-metrics = []
```

To rollback any optimization:
```bash
cargo build --no-default-features --features legacy-lru
```

---

## Appendix A: File Change Summary

| Phase | New Files | Modified Files | Risk Level |
|-------|-----------|----------------|------------|
| Phase 1 | 8 | 12 | Low |
| Phase 2 | 6 | 15 | Medium |
| Phase 3 | 4 | 10 | Medium |
| Phase 4 | 5 | 8 | Low |

## Appendix B: Dependencies to Add

```toml
# crates/cache/Cargo.toml
[dependencies]
slab = "0.4"          # For intrusive LRU
ahash = "0.8"         # Faster hashing

# crates/observability/Cargo.toml
[dependencies]
itoa = "1"            # Fast integer formatting
ryu = "1"             # Fast float formatting

# crates/server/Cargo.toml
[dependencies]
hyper-util = { version = "0.1", features = ["http2"] }
rmp-serde = "1"       # Binary serialization
```

## Appendix C: Feature Flag Registry

| Flag | Default | Description | Rollback |
|------|---------|-------------|----------|
| `lru-linked` | Yes | O(1) LRU eviction | Switch to `legacy-lru` |
| `labeled-metrics` | Yes | Label-aware metrics | Switch to `legacy-metrics` |
| `streaming-upload` | Yes | Zero-copy uploads | Disable flag |
| `request-coalesce` | Yes | Dedup concurrent GETs | Disable flag |
| `wasm-pool` | Yes | Pre-instantiated WASM | Disable flag |
| `presigned-cache` | Yes | Cache presigned URLs | Disable flag |
| `http2` | Yes | HTTP/2 multiplexing | Disable flag |
| `read-ahead` | Yes | Sequential prefetch | Disable flag |
