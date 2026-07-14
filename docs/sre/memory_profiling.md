# Memory Allocation Profiling

**Document:** Memory Profiling Configuration  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Overview

Memory allocation profiling tracks allocation patterns per request to identify hot paths with excessive allocations. Uses `dhat` for allocation profiling in integration tests and development builds.

---

## Configuration

### Build with profiling

```bash
# Enable dhat profiling
RUSTFLAGS="--cfg dhat" cargo test -p ferro-server

# Profile output: dhat-heap.json (viewable in dhat viewer)
```

### Feature flag

Add to `Cargo.toml`:
```toml
[features]
profiling = ["dhat"]

[dependencies]
dhat = { version = "0.3", optional = true }
```

### Global allocator (profiling builds only)

```rust
#[cfg(dhat)]
#[global_allocator]
static GLOBAL: dhat::Alloc = dhat::Alloc;

#[cfg(dhat)]
#[ctor::ctor]
fn init() {
    let profiler = dhat::Profiler::builder().file_name("dhat-heap.json").build();
    // Profiler is dropped at exit, writing the profile
}
```

---

## Allocation Budgets

| Path | Max Allocations | Target | Current (estimated) |
|------|-----------------|--------|---------------------|
| PUT (upload) | < 50 allocs | 30 | ~45 |
| GET (download) | < 20 allocs | 10 | ~18 |
| PROPFIND | < 30 allocs | 15 | ~25 |
| Auth (token validation) | < 10 allocs | 5 | ~8 |
| WebDAV parse | < 20 allocs | 10 | ~22 |

---

## Monitoring

### Per-request allocation tracking

```rust
// Middleware to track allocations per request
struct AllocationTracker {
    start_stats: dhat::Stats,
}

impl AllocationTracker {
    fn new() -> Self {
        Self {
            start_stats: dhat::get_stats(),
        }
    }

    fn report(&self) -> AllocationReport {
        let end_stats = dhat::get_stats();
        AllocationReport {
            allocations: end_stats.curr_blocks - self.start_stats.curr_blocks,
            bytes: end_stats.curr_bytes - self.start_stats.curr_bytes,
        }
    }
}
```

### CI Integration

```yaml
# In CI workflow
- name: Run allocation profiling
  run: |
    RUSTFLAGS="--cfg dhat" cargo test -p ferro-server --lib
    # Parse dhat-heap.json for allocation counts
```

---

## Optimization Strategies

| Strategy | Impact | Example |
|----------|--------|---------|
| Arena allocation | Reduce per-request allocs | Use `bumpalo` for request-scoped data |
| Pre-allocated buffers | Eliminate runtime allocs | Reuse `BytesMut` from pool |
| Zero-copy parsing | Reduce string allocs | Use `Cow<'_, str>` in parsers |
| Object pooling | Reduce allocation frequency | Cache connection state |
