# Performance Requirements

**Document ID:** FERRO-PERF-001
**Date:** 2026-04-18
**Status:** Draft
**Reference:** Domain constraints (DC-TIMING-*, DC-LATENCY-*, DC-AUTH-TIMING-*, DC-AUTH-CEDAR-TIMING-*)

---

## Measurement Methodology

- All latency targets exclude network I/O (measured from handler entry to response serialization complete).
- Percentiles are measured over a 5-minute window with at least 10,000 samples.
- Throughput targets are measured with 10 concurrent clients for small operations, 1 client for large file transfers.
- Benchmarks run on reference hardware: 4 vCPU, 8 GB RAM, NVMe SSD, localhost network.

---

## WebDAV Subsystem (PERF-WEBDAV)

| ID | Operation | Target (p50) | Target (p99) | Max | Unit |
|----|-----------|-------------|-------------|-----|------|
| PERF-WEBDAV-001 | PROPFIND depth 0 (single resource) | 5ms | 20ms | 50ms | latency |
| PERF-WEBDAV-002 | PROPFIND depth 1 (100 items) | 10ms | 50ms | 100ms | latency |
| PERF-WEBDAV-003 | PROPFIND depth infinity (10K items) | 50ms | 200ms | 500ms | latency |
| PERF-WEBDAV-004 | GET 1KB file | 2ms | 10ms | 50ms | latency |
| PERF-WEBDAV-005 | GET 100MB file | — | — | 1GB/s | throughput |
| PERF-WEBDAV-006 | PUT 1KB file | 5ms | 20ms | 50ms | latency |
| PERF-WEBDAV-007 | PUT 100MB file | — | — | 500MB/s | throughput |
| PERF-WEBDAV-008 | MKCOL | 5ms | 20ms | 50ms | latency |
| PERF-WEBDAV-009 | LOCK | 5ms | 10ms | 50ms | latency |
| PERF-WEBDAV-010 | UNLOCK | 2ms | 5ms | 20ms | latency |

### Rationale

- **PROPFIND depth infinity (10K items):** DC-TIMING-001 requires p99 < 50ms for 10K items to avoid rclone pacer backoff. This requirement is relaxed to 200ms p99 for depth infinity since rclone typically sends depth 1. The 500ms max aligns with DC-TIMING-001 enforcement alerting at p99 > 100ms with headroom.
- **LOCK/UNLOCK:** DC-TIMING-003 requires p99 < 10ms for LOCK acquisition. Microsoft Office perceives lag when lock takes > 100ms.
- **XML streaming:** All PROPFIND targets assume streaming XML generation (DC-XML-001, DC-XML-004). DOM-based XML would exceed these targets by 10x for large collections.

### Enforcement

- SLO monitoring in production; alerting when p99 exceeds max threshold.
- CI regression gate: criterion benchmarks must not regress > 20% from baseline.

---

## Storage Subsystem (PERF-STOR)

| ID | Operation | Target | Max | Unit |
|----|-----------|--------|-----|------|
| PERF-STOR-001 | SHA-256 computation | 500MB/s | — | throughput |
| PERF-STOR-002 | CAS dedup check (content-addressable lookup) | 1ms | 10ms | latency |
| PERF-STOR-003 | Pre-signed URL generation | 1ms | 5ms | latency |
| PERF-STOR-004 | Metadata lookup (path -> CAS hash) | 1ms | 5ms | latency |

### Rationale

- **SHA-256 throughput:** DC-HASH-001 requires minimum 500MB/s throughput. sha2 crate on modern CPUs achieves ~1GB/s single-threaded; 500MB/s is conservative with async overhead.
- **CAS dedup check:** A metadata store lookup (SQLite/S3 HEAD) to check if a content hash already exists. Must be fast to avoid adding latency to every PUT.
- **Pre-signed URL:** DC-PRESIGN-001 defines TTL constraints but not latency. 1ms target assumes in-memory signing with cached credentials.
- **Metadata lookup:** Hot-path for every GET/PROPFIND. Indexed path lookup in SQLite should achieve < 1ms p50.

### Enforcement

- Microbenchmarks in CI for SHA-256 throughput.
- Metadata lookup latency measured in integration tests with populated database.

---

## Auth Subsystem (PERF-AUTH)

| ID | Operation | Target (p50) | Target (p99) | Max | Unit |
|----|-----------|-------------|-------------|-----|------|
| PERF-AUTH-001 | OIDC token validation | 2ms | 5ms | 10ms | latency |
| PERF-AUTH-002 | Cedar policy evaluation (100 policies) | 1ms | 5ms | 10ms | latency |
| PERF-AUTH-003 | JWKS refresh (cold fetch) | — | — | 500ms | one-time |

### Rationale

- **OIDC token validation:** DC-AUTH-TIMING-001 requires p99 < 5ms. With RSA-2048 signature verification (~1ms) and cached JWKS keys, p50 should be < 2ms. The 10ms max accounts for cache misses requiring JWKS refetch (mitigated by background refresh).
- **Cedar evaluation:** DC-AUTH-CEDAR-TIMING-001 requires p99 < 10ms for 100 policies. Cedar evaluation is CPU-bound; 100 policies with 2 conditions each should complete in < 1ms. DC-AUTH-END-END-001 budgets 15ms p99 for combined OIDC + Cedar.
- **JWKS refresh:** DC-AUTH-JWKS-001 specifies 24h cache TTL. Cold fetch latency depends on network to the OIDC provider. 500ms is generous for most deployments.

### Enforcement

- Token validation benchmarked with cached JWKS (warm path).
- Cedar evaluation benchmarked at 100, 500, and 1000 policy counts.
- End-to-end auth middleware latency monitored in production (DC-AUTH-END-END-001 target: 15ms p99).

---

## Server Infrastructure (PERF-SRV)

| ID | Metric | Target | Notes |
|----|--------|--------|-------|
| PERF-SRV-001 | Concurrent connections | 10,000 | With keep-alive; measured at HTTP layer |
| PERF-SRV-002 | Request throughput | 50,000 req/s | Mixed PROPFIND + GET + PUT workload |
| PERF-SRV-003 | Memory usage (idle, no connections) | < 50MB | After startup, before any requests |
| PERF-SRV-004 | Memory usage (10K concurrent connections) | < 2GB | Steady-state; excludes file transfer buffers |
| PERF-SRV-005 | Startup time | < 2s | From process start to accepting connections |

### Rationale

- **10K connections:** Typical for a small-to-medium deployment with rclone clients and Office users. Tokio's reactor can handle 100K+ connections; 10K is conservative.
- **50K req/s:** Achievable with hyper/h1 keep-alive and async handlers. Real-world throughput will be lower due to storage backend I/O.
- **Memory budget:** Rust's zero-cost abstractions keep idle memory low. 2GB for 10K connections assumes ~200KB per connection (request/response buffers, connection state).
- **Startup time:** < 2s allows for configuration loading, database migration check, and OIDC provider discovery.

### Enforcement

- Load test in CI/staging: `wrk` or `k6` against a staging deployment.
- Memory profiling with `jemalloc` or `tikv-jemallocator` for production monitoring.
