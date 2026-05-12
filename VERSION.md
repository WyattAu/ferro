# Ferro Version Tracking

## Current Status
- **Phase:** Production Hardening (Sprint AU per ROADMAP.md)
- **Version:** 2.5.1
- **Tests:** 790 passed, 0 failed, 1 ignored (rclone E2E), 0 clippy warnings
- **Status:** Active Development. All 25 sprints (A-AT) completed. Pre-commit hooks configured, cargo-deny active, fmt/clippy clean.
- **Last Updated:** 2026-05-12

## Phase Progress
| Phase | Status | Completion |
|-------|--------|------------|
| Phase -1: Context Discovery | Completed | 100% |
| Phase 0: Requirements | Completed | 100% |
| Phase 1: Research (Yellow Papers) | Completed | 100% |
| Phase 2: Architecture (Blue Papers) | Completed | 100% |
| Phase 2.5: Concurrency Analysis | Completed | 100% |
| Phase 3: Security Engineering | Completed | 100% |
| Phase 4: Performance Engineering | Completed | 100% |
| Phase 5: Prototype (Phase 1 MVP) | Completed | 100% |
| Phase 6: CI/CD | Completed | 100% |
| Phase 8: Execution Graph | Completed | 100% |
| Sprint J: End-to-End | Completed | 100% |
| Sprint K: Enterprise Hardening | Completed | 100% |
| Sprint L: Persistence Layer | Completed | 100% |
| Sprint M: Cloud + Polish | Completed | 100% |
| Sprint N: Enterprise | Completed | 100% |
| Sprint O: Make It Real | Completed | 100% |
| Sprint P: Ship It | Completed | 100% |

## Sprint Progress
| Sprint | Description | Status |
|--------|-------------|--------|
| Sprint A | StorageEngine trait, PROPFIND depth guard, conditional GET, PROPPATCH, Content-Type sniffing, rclone E2E | Completed |
| Sprint B | AppState builder, auth middleware, API endpoints, Cedar/OIDC config, SQLx migration | Completed |
| Sprint C | Background content indexer, search API, WASM worker API, search auto-creation | Completed |
| Sprint D | Leptos web frontend: file browser, upload/download, drag-drop, search bar, breadcrumbs | Completed |
| Sprint E | Tauri desktop scaffold, DesktopState commands, auto-mount, graceful shutdown | Completed |
| Sprint F | Metadata + CAS dedup + Presigned URLs + Share links + Audit logging + Snapshots + WOPI + User isolation + WASM event loop + Admin dashboard + Tauri build script + Virtual scrolling | Completed |
| Sprint G | Bug fixes: doubled-path URLs, audit IP/UA capture, share password enforcement, CORS middleware, content-type persistence, 7 new integration tests | Completed |
| Sprint H | Hardening: snapshot restore fix, rclone progress parsing, Tauri commands, SQLite tests, OIDC tests, virtual scrolling, 22 new tests | Completed |
| Sprint I | Nix flake: 6 devShells, PostgreSQL 16, trunk 0.21, wasm-bindgen-cli, process-compose, helper scripts | Completed |
| Sprint J | End-to-end: trunk WASM build, static file serving, PROPFIND root fix, COPY/MOVE Destination URI fix, lock token fix, E2E test un-ignored, release build | Completed |
| Sprint K | Enterprise: Cedar middleware wiring, WOPI Discovery + token validation, auto-indexing on PUT/DELETE, WASM spawn_blocking + timeout + I/O sandboxing, CLI PROPFIND rewrite with quick-xml, 23 new tests | Completed |
| Sprint L | Persistence: unified SQLite (metadata + CAS + snapshots + audit), --data-dir flag, body size limits, Dockerfile + docker-compose, 9 new tests | Completed |
| Sprint M | Cloud: S3/GCS/Azure feature flags, ServerPresignedUrlGenerator, LOCK refresh, concurrent access tests, rate limiting middleware, 13 new tests | Completed |
| Sprint N | Enterprise: OIDC PKCE login/callback, WOPI token issuance, WASM --wasm-enabled flag, Cedar context, 5 new tests | Completed |
| Sprint O | Real features: web UI auth flow (login page + callback + token storage), WASM worker upload + event-driven dispatch, real cloud presigned URLs (S3/GCS/Azure), 8 new tests | Completed |
| Sprint P | Ship it: GitHub Actions CI/CD, release workflow, Dependabot, comprehensive user documentation (README, deployment, configuration, API, WebDAV guides) | Completed |
| Sprint Q | Quality: fixed all 51 clippy warnings (Default impls, collapsed ifs, redundant closures, dead code, large Err variants), 0 warnings remaining | Completed |
| Sprint R | Robustness: 12 new integration tests (auth endpoints, WASM upload edge cases, rate limiter boundaries, CORS middleware, config endpoint), 190 total tests | Completed |
| Sprint T | Critical fixes: WASM runner spawn order, content-type buffer overread, CI --all-features, WOPI secret configurable, OIDC redirect URI, CLI policy commands, duplicate worker dedup, WOPI urlsrc configurable | Completed |
| Sprint U | Dependency security: wasmtime 26→44 (5 advisories fixed), rustls-webpki 0.103.13 (CRL panic fixed), WOPI token validation with HMAC+expiry, SECURITY.md, 7 new tests | Completed |
| Sprint V | Release polish: .gitignore/.editorconfig, docs updated (new CLI flags), CI matrix strategy + audit job, Dockerfile hardening (non-root user, layer caching) | Completed |
| Sprint W | Production hardening: graceful shutdown (SIGINT), structured request logging middleware, unwrap→proper errors, version info in /api/config, 1 new test | Completed |
| Sprint X | Observability: JSON health check with subsystem status, request ID middleware (X-Request-ID), gzip compression, /metrics endpoint, --version flag | Completed |
| Sprint Y | Ship-ready: simple auth (--admin-user/--admin-password), Docker with bundled web UI, Caddy HTTPS reverse proxy, data-dir startup warning, 6 new auth tests | Completed |
| Sprint Z | Config file (ferro.toml, --config flag, auto-discovery), audit pagination, web UI share dialog | Completed |
| Sprint AA | Performance benchmarks: criterion suite (throughput, latency, WebDAV ops, WASM dispatch, storage ops) | Completed |
| Sprint AB | E2E testing: Playwright suite (24 tests across file browser, auth, upload/download, navigation) | Completed |
| Sprint AC | Security audit prep: OWASP checklist, STRIDE threat model, pen-test plan, security headers middleware, path traversal protection, 5 new tests | Completed |
| Sprint AD | Release notes: comprehensive RELEASE_NOTES.md for v1.0.0-beta.1 | Completed |
| Sprint AE | API error standardization: consistent JSON error format with error codes, 23 error code constants | Completed |
| Sprint AF | Web UI accessibility: WCAG 2.1 AA (ARIA labels, keyboard nav, color contrast, focus management, skip nav) | Completed |
| Sprint AG | Edge case integration tests: 25 new tests (special chars, unicode, path traversal, auth edge cases) | Completed |
| Sprint AH | Admin API: /api/admin/stats, /api/admin/storage, /api/admin/audit endpoints | Completed |
| Sprint AI | Dark mode: system preference detection, localStorage persistence, full dark theme across all components | Completed |
| Sprint AJ | File preview: inline preview for images, text, PDF, video, audio with 100KB text limit | Completed |
| Sprint AK | Favorites + Recent: star/unstar files, favorites view, recent files view from audit log | Completed |
| Sprint AL | Trash/recycle bin: soft delete, restore, purge, empty trash, trash page | Completed |
| Sprint AM | Bulk operations: multi-select with checkboxes, shift+click range, batch delete | Completed |
| Sprint AN | Toast notifications: success/error/info/warning toasts, auto-dismiss, accessible | Completed |
| Sprint AO | File move/copy: server endpoints, context menu, recursive folder support | Completed |
| Sprint AP | Storage quota: --storage-quota flag, usage tracking, 413 on exceed, quota indicator in header | Completed |
| Sprint AQ | Activity feed: recent operations sidebar, auto-refresh, action icons | Completed |
| Sprint AR | Command palette: Ctrl+K trigger, search filtering, keyboard navigation, 13 commands | Completed |
| Sprint AS | Clipboard operations: Ctrl+C/X/V copy/cut/paste files, visual clipboard indicator | Completed |
| Sprint AT | Mobile responsive: card layout, touch targets, responsive toolbar/dialogs/header | Completed |

## Feature Status
| Feature | Status |
|--------|--------|
| WebDAV (full Class 1/2/3) | Working |
| Storage backends (memory, local) | Working |
| **Storage backends (S3, GCS, Azure)** | **Working (feature flags: s3, gcs, azure)** |
| Content-addressable dedup (CAS) | Wired (in-memory or SQLite) |
| PostgreSQL metadata | Wired (optional) |
| SQLite metadata | Wired (--data-dir or --metadata-db) |
| Unified SQLite persistence | Wired (--data-dir for metadata + CAS + snapshots + audit) |
| Request body size limits | Working (--max-body-size, default 1GB) |
| **Docker support** | **Dockerfile + docker-compose.yml + Caddy HTTPS + bundled web UI** |
| Tantivy full-text search | Working + auto-indexed |
| WASM worker runtime | Working + sandboxed (--wasm-enabled) |
| **OIDC authentication** | **Working (PKCE login flow + callback + token validation)** |
| **Simple HTTP Basic Auth** | **Working (--admin-user/--admin-password for personal use)** |
| **OIDC callback** | **Working (/api/auth/callback)** |
| Cedar authorization | Enforced at middleware level |
| Share links | Working |
| Pre-signed URLs | Working (ServerPresignedUrlGenerator) |
| Audit logging | Working |
| Metadata snapshots | Working |
| WOPI protocol | Working + Discovery + **token issuance** |
| **LOCK refresh** | **Working (RFC 4918 §9.10.2 via If header)** |
| **Rate limiting** | **Working (10k req/min per IP, 429 response)** |
| Per-user isolation | Working |
| Leptos web UI | Built + served at /ui/ + **auth-connected (login, callback, token storage, user display)** |
| Tauri desktop | Config ready |
| Admin CLI | Working + share/snapshot commands |
| Static file serving | Working (--static-dir) |
| **WASM worker upload** | **Working (multipart upload, module listing, deletion)** |
| **Event-driven dispatch** | **Working (workers fire immediately on PUT)** |
| **Real cloud presigned URLs** | **Working (S3/GCS/Azure via object_store Signer trait)** |
| **Graceful shutdown** | **Working (SIGINT handler, connection draining)** |
| **Structured request logging** | **Working (method, path, status, duration, client IP, request ID)** |
| **Health check** | **Working (JSON with subsystem status, version, uptime)** |
| **Metrics endpoint** | **Working (JSON metrics: uptime, storage stats)** |
| **Request ID** | **Working (X-Request-ID header, preserved from client or UUID v4)** |
| **Response compression** | **Working (gzip via tower-http)** |
| **--version flag** | **Working (server + CLI binaries)** |
| **User documentation** | **README + deployment + configuration + API + WebDAV guides** |
| **Dark mode** | **Working (system preference + manual toggle)** |
| **File preview** | **Working (images, text, PDF, video, audio)** |
| **Favorites** | **Working (star/unstar, favorites view)** |
| **Recent files** | **Working (from audit log)** |
| **Trash/recycle bin** | **Working (soft delete, restore, purge)** |
| **Bulk operations** | **Working (multi-select, batch delete)** |
| **Toast notifications** | **Working (success/error/info/warning)** |
| **File move/copy** | **Working (recursive, API + UI)** |
| **Storage quota** | **Working (--storage-quota, 413 on exceed)** |
| **Activity feed** | **Working (auto-refresh sidebar)** |
| **Command palette** | **Working (Ctrl+K, 13 commands)** |
| **Clipboard operations** | **Working (Ctrl+C/X/V, visual indicator)** |
| **Mobile responsive** | **Working (card layout, touch targets)** |
| **WCAG 2.1 AA** | **Working (ARIA, keyboard nav, focus management)** |

## Crates Status
| Crate | Tests | Status |
|-------|-------|--------|
| ferro-common | 8 passing | Implemented |
| ferro-core | 57 passing | Implemented |
| ferro-server | 94 lib + 37 integration + 1 E2E | Implemented |
| ferro-web | 0 (WASM-only, trunk-built) | Implemented |
| ferro-cli | 5 passing | Implemented |
| ferro-desktop | 7 passing | Implemented |

## Total Tests: 692+ passing, 2 ignored (1 rclone E2E ignored - needs rclone, 1 doctest ignored)
## Clippy: 0 warnings
## Security: cargo-deny configured, cargo-audit CI-gated, deny.toml with documented ignores for desktop-only transitive deps
## Error Level: None
## Rollback Checkpoint: main@b25fc13 (production security hardening, 2026-05-12)
