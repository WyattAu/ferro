# Performance Optimization Strategy

**Document:** Performance Optimization Plan  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Current Baseline

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| p50 latency | 9.27ms | < 5ms | -48% |
| p99 latency | 1,550ms | < 200ms | -87% |
| Throughput | 48 req/s | > 200 req/s | +317% |
| Memory (idle) | 52MB | < 40MB | -23% |
| Clone count | 2,714 | < 1,900 | -30% |
| String allocs (hot path) | ~45/req | < 20/req | -56% |

---

## Phase 1: Quick Wins (Weeks 1-4)

### 1.1 Hashbrown for Hot Paths

Replace `std::collections::HashMap` with `hashbrown::HashMap` in hot paths:
- File metadata cache
- WebDAV property store
- Connection pool state

**Expected improvement:** 5-10% throughput increase (cache-friendly SIP hasher)

### 1.2 Pre-allocated Buffers

Pool frequently allocated buffers:
- `BytesMut` for request/response bodies
- `Vec<u8>` for XML serialization
- String buffers for path manipulation

**Expected improvement:** 10-15% reduction in allocation count

### 1.3 eliminate Redundant Clones

Priority targets:
- `tcp_transport.rs` (62 clones -> target < 20)
- `handler/*.rs` (387 clones across handler files)
- `state/mod.rs` (AppState cloning)

**Expected improvement:** 20-30% reduction in clone count

---

## Phase 2: Parser Optimization (Weeks 5-12)

### 2.1 Zero-Copy XML Parsing

Replace owned String parsing with borrowed `&str`:
- WebDAV PROPFIND/PROPPATCH responses
- CalDAV/iCal event parsing
- vCard contact parsing

**Implementation:**
```rust
// Before
fn parse_propfind(input: &str) -> Result<Vec<Property>> {
    let doc = Document::parse(input)?;  // Copies all strings
    // ...
}

// After
fn parse_propfind(input: &'input str) -> Result<Vec<Property<'input>>> {
    let doc = Document::parse_borrowed(input)?;  // Zero copies
    // ...
}
```

**Expected improvement:** 30-40% reduction in parsing allocations

### 2.2 Cow<String> for Path Operations

Use `Cow<'_, str>` for path operations that may or may not need allocation:
```rust
fn normalize_path(path: &str) -> Cow<'_, str> {
    if path.contains("//") || path.contains("/./") {
        Cow::Owned(path.replace("//", "/").replace("/./", "/"))
    } else {
        Cow::Borrowed(path)
    }
}
```

**Expected improvement:** 20-30% reduction in path-related allocations

---

## Phase 3: Memory Optimization (Weeks 13-16)

### 3.1 Arena Allocation for Request Processing

Use `bumpalo` arena allocator for request-scoped data:
```rust
let arena = bumpalo::Bump::new();
let request_data = arena.alloc_slice_copy(&raw_body);
// All allocations in this scope use the arena
// Freed in bulk when request completes
```

**Expected improvement:** 40-50% reduction in per-request allocation count

### 3.2 Object Pooling

Pool expensive objects:
- TLS sessions (connection reuse)
- XML serializer/deserializer instances
- SHA-256 hasher instances

**Expected improvement:** 15-20% reduction in object creation overhead

### 3.3 Memory-Mapped Files for Large Reads

Use `memmap2` for large file reads:
```rust
let file = File::open(path)?;
let mmap = unsafe { Mmap::map(&file)? };
// Serve file content directly from mmap
```

**Expected improvement:** 50-70% reduction in memory usage for large file downloads

---

## Phase 4: Concurrency Optimization (Weeks 17-20)

### 4.1 Lock-Free Event Bus

Replace `tokio::sync::broadcast` with lock-free MPMC queue:
- Use `crossbeam::queue::ArrayQueue` for event distribution
- Epoch-based reclamation for subscriber lifecycle

**Expected improvement:** >10x throughput for event distribution

### 4.2 Sharded State

Replace single `DashMap` with sharded state:
```rust
struct ShardedState {
    shards: [DashMap<Key, Value>; 64],  // 64 shards
}

impl ShardedState {
    fn get_shard(&self, key: &Key) -> &DashMap<Key, Value> {
        let hash = ahash(key);
        &self.shards[hash % 64]
    }
}
```

**Expected improvement:** 20-30% reduction in lock contention

### 4.3 Batch Processing

Batch database operations:
- Batch audit log writes (buffer 100 entries, flush every 5s)
- Batch metadata updates (buffer 50 updates, flush every 2s)
- Batch notification dispatch

**Expected improvement:** 30-40% reduction in I/O overhead

---

## Phase 5: I/O Optimization (Weeks 21-24)

### 5.1 Sendfile for Large Downloads

Use `tokio::io::copy` with sendfile for large file downloads:
```rust
// For files > 1MB, use zero-copy sendfile
if file_size > 1_000_000 {
    let file = File::open(path).await?;
    tokio::io::copy(&mut ReaderStream::new(file), &mut writer).await?;
}
```

**Expected improvement:** 40-60% reduction in CPU usage for large downloads

### 5.2 Connection Pool Tuning

Optimize connection pool parameters:
- SQLite: WAL mode, busy timeout 5000ms, page size 4096
- Connection pool: max 100 connections, min idle 10
- Prepared statement cache: 100 statements

**Expected improvement:** 20-30% reduction in database latency

### 5.3 Response Compression Tuning

Tune compression based on content type:
- Text (HTML, JSON, XML): gzip level 6 (balance speed/ratio)
- Binary (already compressed): no compression
- Large files (>1MB): streaming compression

**Expected improvement:** 15-25% reduction in bandwidth usage

---

## Profiling Strategy

### Tools

| Tool | Purpose | Frequency |
|------|---------|-----------|
| `cargo-flamegraph` | CPU profiling | Weekly |
| `dhat` | Allocation profiling | Per PR |
| `tokio-console` | Async task profiling | Development |
| `perf` | System-level profiling | Monthly |
| `massif` | Memory profiling | Monthly |

### CI Integration

```yaml
# Performance regression detection
- name: Benchmark
  run: cargo bench -p ferro-benchmarks -- --output-format bencher > bench.txt
- name: Compare
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: cargo
    output-file-path: bench.txt
    alert-threshold: '105%'
    fail-on-alert: true
```

---

## Measurement Methodology

### Latency Measurement

```rust
// Middleware to measure request latency
async fn measure_latency(req: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed();
    
    tracing::info!(
        duration_ms = duration.as_millis() as u64,
        "Request completed"
    );
    
    response
}
```

### Allocation Measurement

```rust
// Per-request allocation tracking
#[cfg(dhat)]
struct AllocTracker {
    start: dhat::Stats,
}

#[cfg(dhat)]
impl AllocTracker {
    fn new() -> Self {
        Self { start: dhat::get_stats() }
    }
    
    fn report(&self) -> (u64, u64) {
        let end = dhat::get_stats();
        (end.curr_blocks - self.start.curr_blocks, end.curr_bytes - self.start.curr_bytes)
    }
}
```

---

## Success Criteria

| Metric | Target | Measurement |
|--------|--------|-------------|
| p50 latency | < 5ms | Criterion benchmarks |
| p99 latency | < 200ms | Criterion benchmarks |
| Throughput | > 200 req/s | k6 load test |
| Memory (idle) | < 40MB | Process monitoring |
| Memory (1000 concurrent) | < 200MB | Load test |
| Clone count | < 1,900 | cargo-clippy |
| Allocation count per request | < 20 | dhat profiling |
| CPU utilization (1000 req/s) | < 50% | perf monitoring |
