# Ferro Domain Analysis — Phase -1: Context Discovery

**Document ID:** FERRO-DA-001
**Date:** 2026-04-18
**Status:** Draft

---

## 1. Primary Domain

**File Storage, Synchronization, and Collaboration Platform**

Ferro operates in the enterprise content management (ECM) and cloud storage gateway domain. It is a self-hosted, multi-protocol storage orchestrator that abstracts backend storage (local FS, S3, GCS, Azure Blob) behind standard protocols (WebDAV, WOPI) with policy-driven access control and extensible file processing.

---

## 2. Sub-Domains

### 2.1 Core Server (Storage Orchestration)
- Multi-backend storage abstraction via `object_store` crate
- Content-Addressable Storage (CAS) with SHA-256 deduplication
- Pre-signed URL generation for direct-to-cloud transfer (bypass proxy bottleneck)
- Tokio + Axum async HTTP server
- Metadata engine: PostgreSQL (HA) / LibSQL/SQLite (edge)

### 2.2 Interface Layer (Protocols)
- Native WebDAV Class 1 (read/write), Class 2 (locking), Class 3 (ACL)
- WOPI protocol for collaborative editing (Collabora, OnlyOffice, MS Office Online)
- REST API for programmatic access

### 2.3 Identity & Authorization
- OIDC SSO integration (Keycloak, Authelia, Okta)
- Cedar policy language for fine-grained, verifiable authorization
- Policy-as-code: attribute-based access control (ABAC)

### 2.4 Active FS (WASM Plugin System)
- Wasmtime runtime for sandboxed file-processing plugins
- Event-driven execution: pattern-matched triggers on file operations
- Use cases: OCR, virus scan, image resize, webhook notifications

### 2.5 Search
- Tantivy embedded full-text search engine
- Background content indexing workers
- Metadata + content search without external cluster dependency

### 2.6 Web Frontend
- Leptos SSR + WASM hydration (zero-JS/TS)
- Shared types via `ferro-common` crate (compile-time API contract)
- Virtualized scrolling for 10,000+ file views
- Admin dashboard: Cedar policy management, WASM worker monitoring

### 2.7 Desktop Client
- Tauri shell (Windows, macOS, Linux)
- Embedded rclone sidecar for zero-config VFS mount
- System tray integration, sync status monitoring
- Credential injection and lifecycle management

### 2.8 Enterprise Governance
- Immutable audit log (append-only, gRPC streaming to SOC)
- Ransomware protection via instant metadata snapshots (CAS)
- Global cross-account deduplication
- Sovereign proxying: unified gateway for S3, NAS, SFTP

---

## 3. Comparable Systems

| System | GitHub Stars | License | Language | Architecture | Key Differentiator |
|--------|-------------|---------|----------|-------------|-------------------|
| **Nextcloud** | ~30,000 | AGPL-3.0 | PHP + JS | Monolith + plugins | Largest ecosystem, most mature |
| **ownCloud** | ~15,000 | AGPL-3.0 | PHP + JS | Monolith | Enterprise-focused fork of Nextcloud |
| **Seafile** | ~13,000 | AGPL-3.0 (server) / Apache-2.0 (client) | C (core) + Python | Client-server with sync daemon | Best-in-class sync algorithm |
| **MinIO** | ~50,000 | AGPL-3.0 (enterprise) / Apache-2.0 | Go | Distributed object storage | S3-compatible, highest throughput |
| **Syncthing** | ~67,000 | MPL-2.0 | Go | Peer-to-peer mesh | Decentralized, no server required |
| **rclone** | ~50,000 | MIT | Go | CLI + FUSE mount | Universal cloud storage swiss-army knife |

**Ferro's Position:** Rust-native, memory-safe, CAS-backed storage with enterprise policy (Cedar), WebDAV-first, and WASM extensibility. Fills the gap between simple file sync (Syncthing) and heavy ECM platforms (Nextcloud) by being protocol-native and WASM-extensible.

---

## 4. Domain Complexity Assessment

Scoring scale: 1 (trivial) to 10 (extreme)

| Subsystem | State Space | Concurrency | Safety-Critical | Crypto | Real-Time | Persistence | Interop | **Weighted Score** |
|-----------|------------|-------------|----------------|--------|-----------|-------------|---------|-------------------|
| Core Server (Storage) | 8 | 8 | 7 | 6 | 5 | 9 | 7 | **8.0** |
| Interface Layer (WebDAV/WOPI) | 6 | 7 | 5 | 3 | 6 | 6 | 9 | **6.5** |
| Identity & Auth (OIDC/Cedar) | 7 | 6 | 9 | 8 | 5 | 7 | 8 | **7.5** |
| Active FS (WASM) | 6 | 7 | 8 | 4 | 6 | 5 | 6 | **6.2** |
| Search (Tantivy) | 7 | 6 | 3 | 2 | 7 | 8 | 4 | **5.6** |
| Web Frontend (Leptos) | 7 | 5 | 4 | 3 | 7 | 4 | 5 | **5.2** |
| Desktop (Tauri) | 5 | 5 | 4 | 4 | 5 | 3 | 7 | **4.9** |
| Enterprise (Audit/Snapshots) | 6 | 7 | 8 | 6 | 6 | 9 | 6 | **7.1** |

### Complexity Rationale

**Core Server (8.0) — Highest**
- CAS deduplication requires consistent hashing and atomic reference counting across concurrent uploads
- Multi-backend storage abstraction must handle partial failures, retries, and eventual consistency differences between S3/GCS/Azure
- Pre-signed URL generation has security implications (timing attacks, scope leakage)
- Metadata engine dual-backend (PostgreSQL + LibSQL) requires migration coherence

**Identity & Auth (7.5) — Second Highest**
- Cedar policy evaluation must be correct; authorization bugs are security vulnerabilities
- OIDC token lifecycle management across sessions
- Policy conflict detection and resolution

**Enterprise (7.1) — Third Highest**
- Immutable audit log requires append-only guarantees even under adversarial conditions
- Instant snapshot/rollback on CAS requires careful metadata versioning

---

## 5. Multi-Lingual Requirements Assessment

| Subsystem | EN-Only? | Additional Languages Needed | Rationale |
|-----------|----------|---------------------------|-----------|
| Core Server | Yes | — | `object_store` crate and Rust ecosystem are EN-documented |
| WebDAV | No | **Required:** ZH, RU | RFC 4918/3253 are EN-only, but ZH/RU blog posts document MS Office WebDAV quirks (locking behavior, encoding issues) not covered in RFCs |
| WOPI | No | **Required:** ZH | OnlyOffice/Collabora have extensive Chinese documentation for WOPI integration edge cases |
| OIDC/Cedar | No | **Recommended:** ZH | Cedar policy language has growing ZH community; AWS Cedar docs have ZH translations |
| WASM/WASI | Yes | — | Wasmtime/WASI specs are well-documented in EN |
| Tantivy | Yes | — | Rust-native, EN-documented |
| Leptos | Yes | — | Rust-native, EN-documented |
| Tauri | No | **Recommended:** ZH, JP | Tauri has large ZH/JP communities with tutorials for rclone sidecar patterns |
| rclone Integration | No | **Recommended:** ZH | rclone has significant ZH user base; advanced mount configurations better documented in ZH |

**Summary:** English is sufficient for most subsystems. Chinese (ZH) sources are valuable for WOPI/OnlyOffice integration and Tauri+rclone patterns. Russian (RU) sources help with WebDAV Office compatibility edge cases.

---

## 6. Risk Assessment — Critical Path Risks (CPRs)

### Phase 1: Core Server + Basic WebDAV

| ID | Risk | Probability | Impact | Mitigation |
|----|------|------------|--------|-----------|
| CPR-1.1 | WebDAV XML parsing overhead negates performance claims | Medium | High | Benchmark early with `quick-xml`; consider SIMD-optimized parser |
| CPR-1.2 | `object_store` crate lacks atomic multi-part upload coordination for CAS dedup | Medium | Critical | Implement write-before-commit pattern with staging area |
| CPR-1.3 | rclone WebDAV mount requires specific locking semantics not documented | High | High | Write rclone integration tests first; validate against rclone's test suite |
| CPR-1.4 | Pre-signed URL security: time-window attacks on direct-to-cloud uploads | Low | Critical | Use short-lived tokens (60s), IP-binding where provider supports it |

### Phase 2: PostgreSQL + Cedar Policy Engine

| ID | Risk | Probability | Impact | Mitigation |
|----|------|------------|--------|-----------|
| CPR-2.1 | Cedar policy evaluation latency under high concurrency | Medium | High | Cache evaluation results; batch policy checks |
| CPR-2.2 | Dual-backend metadata (PostgreSQL/LibSQL) schema drift | Medium | High | Use shared migration tool; compile-time type checks via SQLx |
| CPR-2.3 | Cedar <-> OIDC claim mapping complexity | High | Medium | Define canonical attribute schema early; write integration tests |

### Phase 3: Leptos Web Frontend

| ID | Risk | Probability | Impact | Mitigation |
|----|------|------------|--------|-----------|
| CPR-3.1 | Leptos 0.6 SSR hydration mismatches with dynamic content | Medium | Medium | Strict SSR data serialization; test hydration with `cargo-leptos` |
| CPR-3.2 | Virtualized scrolling at 10K+ files with WebDAV backend latency | Medium | Medium | Implement cursor-based pagination; prefetch on scroll direction |

### Phase 4: Tauri Desktop

| ID | Risk | Probability | Impact | Mitigation |
|----|------|------------|--------|-----------|
| CPR-4.1 | rclone sidecar lifecycle management (crash recovery, orphan processes) | High | High | Implement watchdog with health checks; PID file tracking |
| CPR-4.2 | Cross-platform credential injection (Windows Credential Manager vs macOS Keychain vs Linux secret-service) | Medium | Medium | Use `keyring` crate for abstraction |
| CPR-4.3 | Tauri WebView compatibility (Linux webkitgtk version fragmentation) | Medium | Medium | Pin minimum webkitgtk version; document supported distros |

### Phase 5: Tantivy + WASM Workers

| ID | Risk | Probability | Impact | Mitigation |
|----|------|------------|--------|-----------|
| CPR-5.1 | Wasmtime sandbox escape or resource exhaustion (DoS via malicious WASM plugin) | Low | Critical | Enforce WASI capability restrictions; CPU/memory limits per instance |
| CPR-5.2 | Tantivy index corruption during concurrent write from WASM workers | Medium | High | Single-writer pattern for index updates; queue mutations |
| CPR-5.3 | WASM plugin ABI stability across Wasmtime version upgrades | Medium | Medium | Define stable WASI interface; version the plugin ABI |

---

## 7. Top 5 Critical Path Risks (Cross-Phase)

| Rank | Risk ID | Description | Phase |
|------|---------|-------------|-------|
| 1 | CPR-1.2 | CAS dedup with `object_store` multi-backend atomicity | 1 |
| 2 | CPR-1.3 | rclone WebDAV locking compatibility | 1 |
| 3 | CPR-4.1 | rclone sidecar lifecycle management | 4 |
| 4 | CPR-2.1 | Cedar policy evaluation latency at scale | 2 |
| 5 | CPR-5.1 | Wasmtime sandbox security | 5 |
