# Optimization Roadmap

**Document ID:** FERRO-PERF-OPT-001
**Date:** 2026-04-18
**Status:** Draft

---

## Phase 1 Optimizations (MVP)

These optimizations are applied during initial implementation. They are architectural choices, not retrofits.

### OPT-001: Streaming XML Generation

- **Subsystem:** WebDAV
- **Technique:** Use quick-xml `Writer` to emit XML events directly to the response body via `tokio::io::AsyncWrite`.
- **Impact:** Eliminates DOM allocation for PROPFIND responses. A 10K-item multistatus response would require ~50MB as a DOM tree vs ~0 heap with streaming.
- **Target benchmarks:** `bench_propfind_depth_infinity_10k`, `bench_propfind_depth1_100`
- **Requirement reference:** DC-XML-001, DC-XML-004

### OPT-002: Connection Pooling for Metadata Store

- **Subsystem:** Storage
- **Technique:** SQLx connection pool with `SqlitePoolOptions::max_connections(10)`. For cloud backends, use object_store's built-in connection management.
- **Impact:** Eliminates per-request connection setup overhead (~5ms for SQLite WAL checkpoint on first connection).
- **Target benchmarks:** `bench_propfind_depth1_100`, `bench_get_small`, `bench_put_small`
- **Requirement reference:** REQ-STOR-001

### OPT-003: Async SHA-256 with spawn_blocking

- **Subsystem:** Storage
- **Technique:** Offload SHA-256 computation to `tokio::task::spawn_blocking` to avoid blocking the async runtime. Use `sha2::Sha256` with streaming `update()` for large files.
- **Impact:** Prevents hash computation from blocking other requests. 100MB hash at 500MB/s = 200ms; without spawn_blocking, this blocks the tokio thread for 200ms.
- **Target benchmarks:** `bench_hash_100mb`, `bench_put_large`
- **Requirement reference:** DC-HASH-001

### OPT-004: Hyper/h1 Keep-Alive Connections

- **Subsystem:** Server
- **Technique:** Configure hyper with `http1::Builder::keep_alive(true)`. Set `max_idle_connections` and `idle_timeout` appropriately.
- **Impact:** Eliminates TCP handshake overhead for rclone (which sends many sequential PROPFIND requests during sync) and Office (which sends OPTIONS then LOCK then GET).
- **Target benchmarks:** `bench_concurrent_propfind`, PERF-SRV-001 (concurrent connections)
- **Requirement reference:** PERF-SRV-001, PERF-SRV-002

---

## Phase 2 Optimizations

These optimizations are applied during the metadata/authorization phase.

### OPT-005: Cedar Policy Caching (LRU)

- **Subsystem:** Auth
- **Technique:** LRU cache for recent Cedar authorization decisions, keyed by (principal, action, resource) tuple. Cache size: 10,000 entries. TTL: 60s (matches shortest reasonable policy change interval).
- **Impact:** Avoids re-evaluating Cedar policies for repeated requests to the same resource. Especially effective for rclone sync (repeated PROPFIND on same directories) and Office (repeated LOCK on same file).
- **Target benchmarks:** `bench_concurrent_propfind`, PERF-AUTH-002
- **Requirement reference:** DC-AUTH-CEDAR-HOT-001 (hot-reload must invalidate cache)

### OPT-006: JWKS Caching (24h TTL)

- **Subsystem:** Auth
- **Technique:** In-memory cache of JWKS keyset with 24h TTL (DC-AUTH-JWKS-001). On token validation, check cache first; on cache miss or unknown `kid`, fetch from `jwks_uri`.
- **Impact:** Eliminates network round-trip to OIDC provider on every token validation. Reduces OIDC validation from ~50ms (network) to ~1ms (cached RSA verification).
- **Target benchmarks:** PERF-AUTH-001
- **Requirement reference:** DC-AUTH-JWKS-001, DC-AUTH-JWKS-002

### OPT-007: SQLx Prepared Statement Caching

- **Subsystem:** Storage
- **Technique:** SQLx automatically caches prepared statements for SQLite when using `query_as` with typed parameters. Ensure all metadata queries use parameterized statements (no string interpolation).
- **Impact:** Avoids re-parsing SQL on every request. SQLite statement preparation takes ~0.1ms; at 50K req/s, this saves ~5s of CPU time per second.
- **Target benchmarks:** `bench_propfind_depth1_100`, `bench_get_small`, `bench_put_small`

### OPT-008: Metadata Index on Path Column

- **Subsystem:** Storage
- **Technique:** Create a B-tree index on the metadata table's `path` column. For SQLite: `CREATE INDEX idx_metadata_path ON metadata(path)`. For cloud metadata stores, use the backend's native indexing.
- **Impact:** Path lookup changes from O(n) full scan to O(log n) index lookup. Critical for PROPFIND depth infinity which may perform thousands of path lookups.
- **Target benchmarks:** `bench_propfind_depth_infinity_10k`, `bench_dedup_hit`, `bench_dedup_miss`
- **Requirement reference:** PERF-STOR-004

---

## Phase 3 Optimizations

These optimizations are applied during the frontend phase.

### OPT-009: Leptos SSR Streaming (Chunked HTML)

- **Subsystem:** Web Frontend
- **Technique:** Use Leptos's streaming SSR mode to emit HTML in chunks as components resolve, rather than waiting for the full page to render. Combine with `Transfer-Encoding: chunked`.
- **Impact:** Reduces Time to First Byte (TTFB) for large directory views. The browser can begin parsing and rendering the HTML header/nav while the file list is still being generated.
- **Target benchmarks:** REQ-WEB-002 (initial render < 2s for 10K items)

### OPT-010: Virtualized Scrolling for Large Directory Views

- **Subsystem:** Web Frontend
- **Technique:** Use a virtualization library (e.g., `leptos-virtual-scroll` or custom `web-sys` intersection observer) to render only visible rows in directory listings. Pool DOM nodes to avoid allocation/GC churn during scrolling.
- **Impact:** Reduces DOM node count from 10K+ to ~50 visible rows. Eliminates layout thrashing and keeps frame rate > 30 FPS for large directories.
- **Target benchmarks:** REQ-WEB-002 (>= 30 FPS for 10K items)

### OPT-011: WASM Hydration Deferral

- **Subsystem:** Web Frontend
- **Technique:** Defer WASM bundle loading and hydration using `<script type="module" async>` and `IntersectionObserver` for below-the-fold components. Load the WASM bundle after the initial SSR HTML is interactive.
- **Impact:** Reduces Time to Interactive (TTI) by deferring the WASM download (~500KB gzipped) and hydration cost. Users can see and scroll the directory listing before WASM loads.
- **Target benchmarks:** REQ-WEB-001 (SSR + hydration)

---

## Phase 4 Optimizations

These optimizations are applied during the desktop client phase.

### OPT-012: Tauri IPC Optimization (Binary Protocol)

- **Subsystem:** Desktop
- **Technique:** Use Tauri's `invoke` with typed Rust structs (via `serde`) instead of string-based IPC. Batch multiple IPC calls into single commands where possible (e.g., batch file status queries).
- **Impact:** Reduces IPC serialization overhead. Binary serde is ~10x faster than JSON string round-trips. Batching reduces IPC call count for bulk operations.
- **Target benchmarks:** REQ-DESK-004 (system tray status updates), REQ-DESK-005 (notifications)

### OPT-013: rclone Sidecar Pipe Optimization

- **Subsystem:** Desktop
- **Technique:** Use buffered pipes (`tokio::io::duplex` with configurable buffer size) for rclone sidecar communication. Set appropriate stdout/stderr buffer sizes (64KB) to reduce syscalls. Parse rclone output incrementally (line-by-line) rather than buffering entire output.
- **Impact:** Reduces latency for rclone status updates (sync progress, errors). Prevents pipe buffer blocking when rclone produces verbose output.
- **Target benchmarks:** REQ-DESK-002 (sidecar lifecycle), REQ-DESK-004 (status display)

---

## Phase 5 Optimizations

These optimizations are applied during the intelligence (WASM/Search) phase.

### OPT-014: Tantivy Index Warm-Up

- **Subsystem:** Search
- **Technique:** Pre-load Tantivy index segments into the OS page cache on startup using `madvise(MADV_WILLNEED)` or equivalent. Schedule background segment merging during low-traffic periods.
- **Impact:** First search query after startup avoids cold-cache latency (~100ms for a large index). Warm index queries achieve < 10ms p99.
- **Target benchmarks:** REQ-SEARCH-001, REQ-SEARCH-003

### OPT-015: WASM Fuel Metering

- **Subsystem:** WASM Plugin
- **Technique:** Configure Wasmtime with `Config::consume_fuel(true)` and set per-plugin fuel limits. Use `Store::set_fuel()` before each plugin invocation. Monitor fuel consumption in metrics.
- **Impact:** Prevents runaway plugins from consuming unlimited CPU. Enables fair scheduling when multiple plugins are active concurrently.
- **Target benchmarks:** REQ-WASM-004 (resource limits)

### OPT-016: WASM Precompilation

- **Subsystem:** WASM Plugin
- **Technique:** Use Wasmtime's `Engine::precompile_module()` to compile WASM modules to native code at install time (not first invocation). Store precompiled artifacts alongside the `.wasm` files.
- **Impact:** Eliminates compilation latency (~50-200ms per module) on first invocation. Plugins start executing immediately.
- **Target benchmarks:** REQ-WASM-001 (runtime integration), REQ-WASM-002 (event-driven execution)

---

## Priority Summary

| Priority | Optimization | Phase | Impact |
|----------|-------------|-------|--------|
| Critical | OPT-001 Streaming XML | 1 | Prevents OOM on large PROPFIND |
| Critical | OPT-003 Async SHA-256 | 1 | Prevents runtime blocking |
| Critical | OPT-004 Keep-Alive | 1 | Fundamental for throughput |
| High | OPT-002 Connection Pooling | 1 | Per-request latency |
| High | OPT-006 JWKS Caching | 2 | 50x auth latency reduction |
| High | OPT-008 Path Index | 2 | O(log n) metadata lookup |
| Medium | OPT-005 Cedar Policy Cache | 2 | Reduces repeated evals |
| Medium | OPT-007 Statement Cache | 2 | CPU savings at scale |
| Medium | OPT-009 SSR Streaming | 3 | TTFB improvement |
| Medium | OPT-010 Virtual Scrolling | 3 | UI responsiveness |
| Low | OPT-011 Hydration Deferral | 3 | TTI improvement |
| Low | OPT-012 Tauri IPC | 4 | Desktop UX |
| Low | OPT-013 Sidecar Pipes | 4 | Desktop UX |
| Low | OPT-014 Index Warm-Up | 5 | Search cold-start |
| Low | OPT-015 Fuel Metering | 5 | Plugin safety |
| Low | OPT-016 WASM Precompilation | 5 | Plugin startup |
