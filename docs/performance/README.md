# Performance

## Overview

Ferro is designed for high performance with low latency and high throughput. This document covers benchmarks, profiling, and optimization strategies.

## Benchmarks

### Running Benchmarks

```bash
# Run all benchmarks
./scripts/benchmark.sh

# Run specific benchmarks
cargo bench --package ferro-benchmarks --bench crypto        # Hash + XML escape
cargo bench --package ferro-benchmarks --bench crypto_ops    # Password, HMAC, SHA256
cargo bench --package ferro-benchmarks --bench dav           # iCal + vCard parse/serialize
cargo bench --package ferro-benchmarks --bench dav_parsing   # DAV XML query parsing
cargo bench --package ferro-benchmarks --bench storage       # In-memory storage ops
cargo bench --package ferro-benchmarks --bench webdav_ops    # Path normalization, metadata
cargo bench --package ferro-benchmarks --bench dav_protocol  # DAV protocol operations
cargo bench --package ferro-benchmarks --bench auth_totp     # TOTP authentication
cargo bench --package ferro-benchmarks --bench owncloud_sync # ownCloud sync protocol
```

### Benchmark Suites

#### Crypto (`bench/crypto.rs`)
- Content hash (1KB, 1MB data)
- XML escape (plain, special, mixed strings)

#### Crypto Ops (`bench/crypto_ops.rs`)
- Password hashing (Argon2id)
- Password verification
- HMAC-SHA256 signing
- SHA-256 hashing

#### DAV (`bench/dav.rs`)
- iCalendar parse (small, medium calendars)
- iCalendar serialize
- vCard parse (small, complex contacts)
- vCard serialize

#### Storage (`bench/storage.rs`)
- Put (1KB, 10KB, 100KB)
- Get (10KB)
- List (100 files)
- Delete
- Exists (hit/miss)
- Head

## Optimization

### Memory Pools (`crates/common/src/pools.rs`)

Ferro uses several memory pooling strategies:

```rust
use ferro_common::pools::{RequestArena, BufferPool, StringInterner, GlobalPools};

// Request-scoped arena allocator (bulk-free on drop)
let arena = RequestArena::with_capacity(4096);
let data = arena.alloc_slice_copy(&[1u8; 1024]);
let s = arena.alloc_str("temporary string");

// Buffer pool for network I/O
let pool = BufferPool::new(64, 8192);
let buf = pool.get();  // Reuse pre-allocated buffer
pool.put(buf);         // Return to pool

// String interning for deduplication
let interner = StringInterner::new();
let s1 = interner.intern("repeated-string");
let s2 = interner.intern("repeated-string");
assert!(Arc::ptr_eq(&s1, &s2));  // Same allocation

// Global singleton
let pools = GlobalPools::instance();
```

### SIMD Acceleration (`crates/common/src/simd/`)

XML escape detection uses SIMD on x86_64 for large ASCII strings:

```rust
use ferro_common::xml_escape::escape_xml;

// Automatically uses SIMD for ASCII detection
let result = escape_xml("<unsafe>content</unsafe>");
// Zero-copy (Cow::Borrowed) when no escaping needed
let safe = escape_xml("safe content");
```

### Streaming Content Hash

For large files, use streaming to avoid loading everything into memory:

```rust
use ferro_common::metadata::ContentHash;
use std::fs::File;

let file = File::open("large-file.bin")?;
let hash = ContentHash::compute_reader(file)?;
```

## Profiling

### Cargo Flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin ferro-server
```

### perf

```bash
# Record profile
perf record -g target/release/ferro-server

# Analyze profile
perf report
```

### Valgrind

```bash
# Memory profiling
cargo install cargo-valgrind
cargo valgrind
```

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| p50 latency | <10ms | 9.27ms |
| p99 latency | <100ms | 1.55s |
| Throughput | >1000 req/s | 48 req/s |
| Memory usage | <512MB | ~256MB |
| CPU usage | <50% | ~30% |
