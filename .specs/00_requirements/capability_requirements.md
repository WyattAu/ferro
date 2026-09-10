# Ferro Capability Requirements — Tools & Dependencies

**Document ID:** FERRO-CR-001
**Date:** 2026-04-18
**Status:** Draft

---

## 1. Development Toolchain

### 1.1 Rust Toolchain

| Tool | Minimum Version | Source | Status | Notes |
|------|----------------|--------|--------|-------|
| `rustc` | >= 1.85.0 (edition 2024) | Nix flake (`rust-bin.stable.latest`) | Available | Edition 2024 stabilized in Rust 1.85 |
| `cargo` | >= 1.85.0 | Nix flake (bundled with rustc) | Available | |
| `rust-analyzer` | latest stable | Nix flake (`rust-src` extension) | Available | LSP server for editors |
| `rust-src` | matches rustc | Nix flake (`rust-src` extension) | Available | Required for `cargo check` on stdlib code |
| `wasm32-unknown-unknown` target | N/A | Nix flake (`targets` config) | Available | Required for Leptos WASM output |

### 1.2 Build & Bundling Tools

| Tool | Minimum Version | Source | Status | Notes |
|------|----------------|--------|--------|-------|
| `trunk` | >= 0.21.0 | Nix flake | Available | Leptos WASM/SSR bundler |
| `binaryen` (wasm-opt) | >= 120 | Nix flake | Available | WASM size/speed optimization |
| `cargo-watch` | >= 8.5 | Nix flake | Available | Auto-rebuild on file change |
| `cargo-tauri` | >= 2.0 | Nix flake | Available | Tauri CLI for desktop build |

### 1.3 Database Tools

| Tool | Minimum Version | Source | Status | Notes |
|------|----------------|--------|--------|-------|
| `sqlx-cli` | >= 0.8 | Nix flake | Available | Database migrations; offline query checking |
| `postgresql` | >= 16 | External | **Missing** | Not in flake; must be external or added |
| `libsql` / `turso-cli` | >= 0.24 | External | **Missing** | Edge DB alternative; not in flake |

### 1.4 Desktop Client Dependencies

| Tool | Minimum Version | Source | Status | Notes |
|------|----------------|--------|--------|-------|
| `rclone` | >= 1.68 | Nix flake | Available | Sidecar for Tauri desktop client |
| `webkitgtk_4_1` | system | Nix flake | Available | Linux WebView for Tauri |
| `gtk3` | system | Nix flake | Available | Linux GUI toolkit for Tauri |
| `cairo`, `gdk-pixbuf`, `glib`, `dbus`, `librsvg`, `libsoup_3` | system | Nix flake | Available | Linux GUI support libraries |

### 1.5 Verification & Quality Tools

| Tool | Minimum Version | Source | Status | Notes |
|------|----------------|--------|--------|-------|
| `cargo-clippy` | bundled with rustc | Nix flake | Available | Linting |
| `cargo-audit` | >= 0.21 | External | **Missing** | Security vulnerability scanning; add to flake |
| `cargo-deny` | >= 0.15 | External | **Missing** | License/dependency audit; add to flake |
| `cargo-nextest` | >= 0.9 | External | **Missing** | Faster test runner; recommended for CI |
| `lean` (Lean 4) | >= 4.0 | External | **Missing** | Optional: formal verification of CAS invariants |

### 1.6 Infrastructure Tools

| Tool | Minimum Version | Source | Status | Notes |
|------|----------------|--------|--------|-------|
| `just` | >= 1.36 | External | **Missing** | Task runner; recommended over Makefile |
| `protobuf` / `protoc` | >= 29 | External | **Missing** | gRPC audit log streaming |
| `grpcurl` | latest | External | **Missing** | gRPC endpoint testing |
| `nix` | >= 2.24 | System | External | Nix package manager (assumed installed) |
| `direnv` | >= 2.35 | System | Available | Already configured (.direnv/ exists) |

---

## 2. Rust Crate Dependencies

### 2.1 Core Dependencies (Planned)

| Crate | Version | Subsystem | Status | Notes |
|-------|---------|-----------|--------|-------|
| `tokio` | 1.x | Core Server | Declared (workspace) | `features = ["full"]` |
| `axum` | 0.7.x | Core Server | Declared (workspace) | HTTP framework |
| `serde` | 1.x | All | Declared (workspace) | `features = ["derive"]` |
| `leptos` | 0.6.x | Web Frontend | Declared (workspace) | `features = ["csr"]` |
| `object_store` | 0.11.x | Core Server | **Missing** | Multi-backend storage; not yet in Cargo.toml |
| `sqlx` | 0.8.x | Metadata Engine | **Missing** | PostgreSQL + LibSQL; not yet added |
| `cedar-policy` | 4.x | Identity & Auth | **Missing** | Authorization engine |
| `wasmtime` | 28.x | Active FS | **Missing** | WASM runtime |
| `tantivy` | 0.22.x | Search | **Missing** | Full-text search |
| `tauri` | 2.x | Desktop | **Missing** | Desktop framework |
| `quick-xml` | 0.37.x | Interface Layer | **Missing** | WebDAV XML parsing |
| `sha2` | 0.10.x | Core Server | **Missing** | SHA-256 for CAS |
| `jsonwebtoken` | 9.x | Identity & Auth | **Missing** | OIDC token handling |
| `tonic` | 0.12.x | Enterprise | **Missing** | gRPC audit log streaming |
| `tower` | 0.5.x | Core Server | **Missing** | Middleware layer (auth, logging, rate-limit) |
| `tower-http` | 0.6.x | Core Server | **Missing** | CORS, compression, tracing |
| `tracing` | 0.1.x | All | **Missing** | Structured logging |
| `uuid` | 1.x | Core Server | **Missing** | File/version identifiers |
| `chrono` | 0.4.x | All | **Missing** | Timestamp handling |

---

## 3. Environment Matrix

### 3.1 Development Environment

| Requirement | Specification | Source | Status |
|-------------|--------------|--------|--------|
| Rust toolchain | Stable 1.85+, edition 2024 | Nix flake | OK |
| WASM target | `wasm32-unknown-unknown` | Nix flake | OK |
| PostgreSQL | 16+ for local dev | External | **Add to flake** |
| Node.js | Not required (Leptos uses trunk) | N/A | OK |
| System libraries | webkitgtk, gtk3, cairo, etc. | Nix flake | OK |
| rclone | 1.68+ for testing | Nix flake | OK |

### 3.2 CI Environment

| Requirement | Specification | Source | Status |
|-------------|--------------|--------|--------|
| Rust toolchain | Same as dev | GHA/nix | Future |
| PostgreSQL | 16+ service | GHA service container | Future |
| cargo-audit | Weekly scan | GHA action | Future |
| cargo-nextest | Test runner | GHA action | Future |
| cross-compilation | aarch64, windows, macOS | GHA matrix | Future |

### 3.3 Production Environment

| Requirement | Specification | Notes |
|-------------|--------------|-------|
| PostgreSQL | 16+ (enterprise) / LibSQL (edge) | Configurable via feature flags |
| Reverse proxy | Caddy / Nginx / Traefik | TLS termination; not bundled |
| Object storage | S3 / GCS / Azure Blob / Local FS | Configured at runtime |
| OIDC provider | Keycloak / Authelia / Okta | External dependency |
| WOPI client | Collabora / OnlyOffice / MS Office Online | External dependency |

---

## 4. Nix Flake Coverage Summary

### Provided by Nix Flake (12 tools)
- rustc, cargo, rust-analyzer, rust-src
- wasm32-unknown-unknown target
- trunk, cargo-tauri, cargo-watch, binaryen (wasm-opt)
- sqlx-cli, rclone
- pkg-config
- GUI libraries: webkitgtk_4_1, gtk3, cairo, gdk-pixbuf, glib, dbus, librsvg, libsoup_3

### Not in Nix Flake — Should Be Added (7 tools)
| Tool | Priority | Reason |
|------|----------|--------|
| `cargo-audit` | P0 | Security scanning required by ISO 27001 / NIST |
| `cargo-deny` | P1 | License compliance (Apache 2.0 enforcement) |
| `cargo-nextest` | P1 | Test performance; essential for large test suites |
| `postgresql` | P0 | Required for Phase 2 development |
| `protobuf` + `grpcurl` | P1 | Required for gRPC audit log development |
| `just` | P2 | Task runner convenience |

### External / Optional (2 tools)
| Tool | Priority | Reason |
|------|----------|--------|
| `lean` (Lean 4) | P2 | Formal verification of CAS invariants; optional |
| `libsql` / `turso-cli` | P1 | Edge DB alternative; may be Cargo dependency only |

---

## 5. Readiness Assessment

| Phase | Required Tools Available? | Blockers |
|-------|--------------------------|----------|
| Phase 1 (Core + WebDAV) | Yes | None — `object_store` crate needs to be added to Cargo.toml |
| Phase 2 (PostgreSQL + Cedar) | Partial | PostgreSQL not in flake; `sqlx` + `cedar-policy` crates not yet added |
| Phase 3 (Leptos Frontend) | Yes | `trunk` available; `leptos` declared in workspace |
| Phase 4 (Tauri Desktop) | Yes | `cargo-tauri` + `rclone` + GUI libs in flake |
| Phase 5 (Tantivy + WASM) | Yes | `wasm32-unknown-unknown` target available; crates not yet added |
