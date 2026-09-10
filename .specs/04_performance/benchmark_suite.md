# Benchmark Suite

**Document ID:** FERRO-PERF-BENCH-001
**Date:** 2026-04-18
**Status:** Draft
**Framework:** Criterion.rs

---

## Setup

- Benchmarks live in `ferro-benches/` crate with `[[bench]]` entries in `Cargo.toml`.
- Criterion configuration in `ferro-benches/criterion.toml` with:
  - `measurement_time = 10` (10s per benchmark)
  - `warm_up_time = 3` (3s warmup)
  - `sample_size = 100` (minimum samples)
- CI runs benchmarks on push to main and on PRs; fails if any benchmark regresses > 20% from baseline.
- Baseline stored in `ferro-benches/.criterion/` and committed to the repository.

### Test Data

- **Small file:** 1KB of deterministic pseudo-random data (seeded RNG).
- **Medium file:** 1MB of deterministic pseudo-random data.
- **Large file:** 10MB of deterministic pseudo-random data (100MB for SHA-256 throughput bench).
- **Directory (100 items):** 100 small files in a flat directory.
- **Directory (10K items):** 10,000 small files in a flat directory.
- Storage backend for benchmarks: in-memory (`object_store::memory::InMemory`) to isolate handler performance from I/O.

---

## 1. WebDAV Benchmarks

### `bench_propfind_depth0`

- **Description:** PROPFIND request for a single resource (Depth: 0).
- **Setup:** Create one collection with `displayname` set.
- **Method:** Send PROPFIND Depth: 0 with `<allprop/>`. Measure handler duration.
- **Target:** p50 < 5ms, p99 < 20ms.

### `bench_propfind_depth1_100`

- **Description:** PROPFIND request for a directory with 100 items (Depth: 1).
- **Setup:** Create a collection with 100 child resources (mix of files and sub-collections).
- **Method:** Send PROPFIND Depth: 1 with `<allprop/>`. Measure handler duration.
- **Target:** p50 < 10ms, p99 < 50ms.

### `bench_propfind_depth_infinity_10k`

- **Description:** PROPFIND request for a directory tree with 10,000 items (Depth: infinity).
- **Setup:** Create a collection with 10,000 descendant resources.
- **Method:** Send PROPFIND Depth: infinity with `<allprop/>`. Measure handler duration.
- **Target:** p50 < 50ms, p99 < 200ms.
- **Validation:** Assert peak heap allocation < 10MB (streaming XML must not build DOM).

### `bench_get_small`

- **Description:** GET request for a 1KB file.
- **Setup:** PUT a 1KB file, then GET it.
- **Method:** Send GET request. Measure handler duration (excluding network transfer).
- **Target:** p50 < 2ms, p99 < 10ms.

### `bench_get_large`

- **Description:** GET request for a 10MB file, measuring throughput.
- **Setup:** PUT a 10MB file, then GET it.
- **Method:** Send GET request. Measure throughput (bytes/second).
- **Target:** > 1GB/s (memory backend limited).

### `bench_put_small`

- **Description:** PUT request for a 1KB file.
- **Setup:** Generate 1KB of random data.
- **Method:** Send PUT request with body. Measure handler duration.
- **Target:** p50 < 5ms, p99 < 20ms.

### `bench_put_large`

- **Description:** PUT request for a 10MB file, measuring throughput.
- **Setup:** Generate 10MB of random data.
- **Method:** Send PUT request with body. Measure throughput (bytes/second).
- **Target:** > 500MB/s.

### `bench_mkcol`

- **Description:** MKCOL request to create a collection.
- **Setup:** Target path does not exist.
- **Method:** Send MKCOL request. Measure handler duration.
- **Target:** p50 < 5ms, p99 < 20ms.

### `bench_lock_unlock`

- **Description:** LOCK + UNLOCK cycle on a single resource.
- **Setup:** Resource exists. No prior lock.
- **Method:** Send LOCK (exclusive, 60s timeout), then UNLOCK with lock token. Measure combined duration.
- **Target:** Combined p50 < 7ms, p99 < 30ms.

### `bench_copy`

- **Description:** COPY a 1KB file within the same repository.
- **Setup:** Source file exists with known content.
- **Method:** Send COPY request (Destination header). Verify content at destination. Measure handler duration.
- **Target:** p50 < 10ms, p99 < 50ms.

### `bench_move`

- **Description:** MOVE a 1KB file within the same repository.
- **Setup:** Source file exists with known content.
- **Method:** Send MOVE request (Destination header). Verify content at destination and 404 at source. Measure handler duration.
- **Target:** p50 < 5ms, p99 < 20ms.

---

## 2. CAS / Storage Benchmarks

### `bench_hash_1kb`

- **Description:** SHA-256 computation on 1KB of data.
- **Setup:** Pre-generate 1KB buffer.
- **Method:** Hash the buffer using `sha2::Sha256`. Measure throughput.
- **Target:** > 500MB/s effective throughput (trivially met for 1KB; this validates micro-overhead).

### `bench_hash_1mb`

- **Description:** SHA-256 computation on 1MB of data.
- **Setup:** Pre-generate 1MB buffer.
- **Method:** Hash the buffer using `sha2::Sha256`. Measure throughput.
- **Target:** > 500MB/s.

### `bench_hash_100mb`

- **Description:** SHA-256 computation on 100MB of data (streaming).
- **Setup:** Generate 100MB via repeated writes to avoid memory allocation.
- **Method:** Hash using streaming `update()` calls. Measure throughput.
- **Target:** > 500MB/s.

### `bench_dedup_hit`

- **Description:** CAS deduplication check for existing content (cache hit path).
- **Setup:** Content with known hash already stored. Metadata index populated.
- **Method:** Compute hash, check metadata store for existence. Measure combined duration.
- **Target:** p50 < 1ms.

### `bench_dedup_miss`

- **Description:** CAS deduplication check for new content (cache miss path).
- **Setup:** Content with unknown hash. Metadata index populated with other entries.
- **Method:** Compute hash, check metadata store for existence (returns not found). Measure combined duration.
- **Target:** p50 < 2ms.

---

## 3. Concurrency Benchmarks

### `bench_concurrent_propfind`

- **Description:** 100 concurrent PROPFIND requests (depth 1, 100 items each).
- **Setup:** 10 directories each with 100 items.
- **Method:** Spawn 100 async tasks, each sending PROPFIND Depth: 1 to a random directory. Await all. Measure total wall-clock time and per-request latency distribution.
- **Target:** Median per-request latency within 2x of single-request benchmark.

### `bench_concurrent_put`

- **Description:** 100 concurrent PUT requests (1KB files, unique content).
- **Setup:** Pre-generate 100 unique 1KB payloads.
- **Method:** Spawn 100 async tasks, each sending PUT with unique content. Await all. Measure total wall-clock time and per-request latency distribution.
- **Target:** Median per-request latency within 3x of single-request benchmark (CAS contention overhead).

### `bench_concurrent_lock`

- **Description:** 100 concurrent LOCK requests on distinct resources.
- **Setup:** 100 resources exist, no prior locks.
- **Method:** Spawn 100 async tasks, each sending LOCK on a distinct resource. Await all. Measure total wall-clock time and per-request latency distribution.
- **Target:** Median per-request latency within 2x of single-request benchmark.

---

## CI Integration

```toml
# .github/workflows/bench.yml (conceptual)
# Runs on: push to main, pull_request
# Steps:
#   1. cargo build --release
#   2. cargo bench --package ferro-benches -- --save-baseline main
#   3. On PRs: cargo bench --package ferro-benches -- --baseline main 2>&1 | grep "regressed"
#   4. Fail CI if any benchmark regresses > 20%
```

### Regression Policy

- A benchmark regresses if its estimated mean increases by > 20% compared to the `main` baseline.
- False positives (noise) are handled by Criterion's statistical significance check (p < 0.05).
- Regression PRs must include a justification comment or a fix in the same PR.
