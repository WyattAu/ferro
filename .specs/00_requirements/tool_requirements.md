# Ferro Tool Requirements — Consolidated

**Document ID:** FERRO-TR-001
**Date:** 2026-04-18
**Status:** Draft

---

## 1. Core Build Toolchain

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `rustc` | >= 1.85.0 (edition 2024) | Rust compiler | Nix flake (`rust-bin.stable.latest`) | `rustc --version` |
| `cargo` | >= 1.85.0 | Rust package manager | Nix flake (bundled with rustc) | `cargo --version` |
| `rust-analyzer` | latest stable | LSP server for editors | Nix flake (`rust-src` extension) | `rust-analyzer --version` |
| `rust-src` | matches rustc | Standard library source (required for `cargo check` on stdlib) | Nix flake (`rust-src` extension) | `rustup component list \| grep rust-src` |
| `wasm32-unknown-unknown` target | N/A | WASM compilation target for Leptos | Nix flake (`targets` config) | `rustup target list \| grep wasm32-unknown-unknown` |

## 2. Build & Bundling

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `trunk` | >= 0.21.0 | Leptos WASM/SSR bundler | Nix flake | `trunk --version` |
| `binaryen` (wasm-opt) | >= 120 | WASM binary size and speed optimization | Nix flake | `wasm-opt --version` |
| `cargo-watch` | >= 8.5 | Auto-rebuild on file change during development | Nix flake | `cargo watch --version` |
| `cargo-tauri` | >= 2.0 | Tauri desktop client build and dev tooling | Nix flake | `cargo tauri --version` |

## 3. Database

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `sqlx-cli` | >= 0.8 | Database migrations and offline query checking | Nix flake | `sqlx --version` |
| `postgresql` | >= 16 | Relational database for enterprise metadata engine | External (add to flake) | `psql --version` |
| `libsql-server` / `turso-cli` | >= 0.24 | Embedded database for edge/single-user deployments | External / Cargo dependency | `turso --version` |

## 4. Desktop Client Dependencies

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `rclone` | >= 1.68 | VFS sidecar for Tauri desktop mount | Nix flake | `rclone version` |
| `webkitgtk_4_1` | system | Linux WebView component for Tauri | Nix flake | `pkg-config --modversion webkitgtk-4.1` |
| `gtk3` | system | Linux GUI toolkit for Tauri | Nix flake | `pkg-config --modversion gtk-3` |
| `cairo` | system | 2D graphics library (Tauri rendering) | Nix flake | `pkg-config --modversion cairo` |
| `gdk-pixbuf` | system | Image loading library (Tauri icons/images) | Nix flake | `pkg-config --modversion gdk-pixbuf-2.0` |
| `glib` | system | Core GLib library (Tauri event loop) | Nix flake | `pkg-config --modversion glib-2.0` |
| `dbus` | system | D-Bus IPC (Linux system tray, notifications) | Nix flake | `pkg-config --modversion dbus-1` |
| `librsvg` | system | SVG rendering (Tauri icon scaling) | Nix flake | `pkg-config --modversion librsvg-2.0` |
| `libsoup_3` | system | HTTP client library (Tauri network) | Nix flake | `pkg-config --modversion libsoup-3.0` |

## 5. Verification & Quality

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `cargo-clippy` | bundled with rustc | Rust linting | Nix flake | `cargo clippy --version` |
| `cargo-audit` | >= 0.21 | Security vulnerability scanning (RustSec advisory database) | External (add to flake) | `cargo audit --version` |
| `cargo-deny` | >= 0.15 | License compliance and dependency audit | External (add to flake) | `cargo deny --version` |
| `cargo-nextest` | >= 0.9 | Faster test runner with better output | External (add to flake) | `cargo nextest --version` |
| `lean` (Lean 4) | >= 4.0 | Optional: formal verification of CAS invariants | External | `lean --version` |

## 6. Infrastructure

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `just` | >= 1.36 | Task runner (replaces Makefile) | External (add to flake) | `just --version` |
| `protobuf` / `protoc` | >= 29 | Protocol Buffers compiler for gRPC audit log streaming | External (add to flake) | `protoc --version` |
| `grpcurl` | latest | gRPC endpoint testing and debugging | External (add to flake) | `grpcurl --version` |
| `nix` | >= 2.24 | Nix package manager (reproducible builds) | System (prerequisite) | `nix --version` |
| `direnv` | >= 2.35 | Shell environment management (auto-loads flake) | System / Nix flake | `direnv --version` |
| `pkg-config` | system | Library dependency resolution for native builds | Nix flake | `pkg-config --version` |

## 7. CI-Only Tools

| Tool | Minimum Version | Purpose | Provider | Install Verification |
|------|----------------|---------|----------|---------------------|
| `cargo-llvm-cov` | >= 0.6 | Code coverage measurement | CI-only (GitHub Actions) | `cargo llvm-cov --version` |
| `cross` | >= 0.2 | Cross-compilation for aarch64, Windows, macOS | CI-only (GitHub Actions) | `cross --version` |
| `grcov` | latest | Coverage report generation (alternative) | CI-only | `grcov --version` |

---

## 8. Phase Readiness Matrix

| Phase | Required Tools | Available? | Action Needed |
|-------|---------------|------------|---------------|
| **Phase 1** | rustc, cargo, object_store crate | Nix: rustc, cargo, sqlx-cli, rclone, trunk | Add `object_store`, `sha2`, `quick-xml`, `tower`, `tower-http`, `tracing` to Cargo.toml |
| **Phase 2** | Phase 1 + PostgreSQL, sqlx, cedar-policy, tonal | PostgreSQL not in flake; crates not in Cargo.toml | Add `postgresql` to flake; add `sqlx`, `cedar-policy`, `tonic`, `jsonwebtoken` to Cargo.toml |
| **Phase 3** | Phase 2 + trunk, leptos, wasm32 target | All in flake; `leptos` in workspace | Add `leptos` SSR feature; create `ferro-common`, `ferro-web` crates |
| **Phase 4** | Phase 3 + cargo-tauri, rclone, GUI libs | All in flake | Create `ferro-desktop` crate; bundle rclone sidecar |
| **Phase 5** | Phase 4 + wasm32 target, wasmtime, tantivy | wasm32 target in flake; crates not in Cargo.toml | Add `wasmtime`, `tantivy` to Cargo.toml; create `ferro-wasm`, `ferro-search` crates |

---

## 9. Nix Flake Additions Required

The following tools should be added to the Nix flake before their respective phases:

| Tool | Phase Needed | Priority | Flake Package |
|------|-------------|----------|---------------|
| `cargo-audit` | Phase 1 (CI) | P0 | `cargo-audit` |
| `postgresql` | Phase 2 | P0 | `postgresql_16` |
| `cargo-deny` | Phase 2 | P1 | `cargo-deny` |
| `cargo-nextest` | Phase 2 | P1 | `cargo-nextest` |
| `protobuf` + `grpcurl` | Phase 2 | P1 | `protobuf`, `grpcurl` |
| `just` | Phase 2 | P2 | `just` |
