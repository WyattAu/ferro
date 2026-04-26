# Ferro Code Quality Audit

**Version**: v1.0.0-beta.1  
**Date**: 2026-04-26  
**Scope**: All crates (server, core, common, cli, desktop, web)

---

## 1. Dead Code Audit

### `#[allow(dead_code)]` Annotations

| # | Location | Item | Justification | Status |
|---|----------|------|---------------|--------|
| 1 | `server/benches/helpers/mod.rs:10` | `create_test_app_state` | Benchmark helper | Acceptable |
| 2 | `server/benches/helpers/mod.rs:15` | `create_test_router` | Benchmark helper | Acceptable |
| 3 | `server/benches/helpers/mod.rs:20` | `make_request` | Benchmark helper | Acceptable |
| 4 | `server/benches/helpers/mod.rs:41` | `generate_test_body` | Benchmark helper | Acceptable |
| 5 | `server/benches/helpers/mod.rs:53` | `create_test_file` | Benchmark helper | Acceptable |
| 6 | `server/src/admin_api.rs:165` | `no_auth_test_app` | Test helper | Acceptable |
| 7 | `web/src/components/onboarding.rs:3` | `ONBOARDING_KEY` | WASM feature stub | Acceptable |
| 8 | `web/src/api.rs:81` | `urlencoding` | Utility for future use | Low - remove if not planned |
| 9 | `web/src/api.rs:91` | `LockScope` impl block | WASM stub | Acceptable |
| 10 | `web/src/api.rs:189` | `extract_xml_tag` | Likely unused utility | Low - remove or use |
| 11 | `web/src/api.rs:226` | `fetch_text` | Utility for future use | Low - remove if not planned |
| 12 | `server/src/main.rs:356` | `parse_bucket_from_url` | Future storage backend | Acceptable |
| 13 | `web/src/components/theme_toggle.rs:11` | `Theme::as_str` | WASM serialization | Acceptable |
| 14 | `web/src/components/theme_toggle.rs:19` | `Theme::from_str` | WASM deserialization | Acceptable |
| 15 | `web/src/auth.rs:11` | `STORAGE_KEY` | WASM feature stub | Acceptable |
| 16 | `web/src/auth.rs:15` | `AuthState` struct | WASM feature stub | Acceptable |
| 17 | `web/src/auth.rs:98` | `get_local_storage` | WASM feature stub | Acceptable |
| 18 | `core/src/search.rs:14` | `SearchEngine.schema` | Kept for future queries | Acceptable |
| 19 | `cli/src/client.rs:163` | `get_server_config` | Future CLI command | Acceptable |

**Unused Imports**: None found. No `#[allow(unused_imports)]` annotations.

---

## 2. Error Handling Rigor

### `.unwrap()` Calls in Production Code (non-test)

| # | Location | Expression | Severity | Recommendation |
|---|----------|------------|----------|----------------|
| 1 | `server/src/shares.rs:199` | `HeaderValue::from_str(&format!(...)).unwrap()` | Medium | Filename may contain non-ASCII chars; use `unwrap_or_else` with fallback header |
| 2 | `server/src/request_id.rs:20` | `request_id.parse().unwrap()` | Low | Safe: UUID strings always parse to `HeaderValue`; add comment |
| 3 | `server/src/wopi.rs:361` | `Hmac::new_from_slice(...).unwrap()` | Low | Safe: key length from config always valid for SHA-256; add comment |
| 4 | `server/src/auth/cedar.rs:83` | `"User::\"anonymous\"".parse().unwrap()` | Low | Safe: static literal; add comment |
| 5 | `server/src/auth/cedar.rs:87` | `"Action::\"unknown\"".parse().unwrap()` | Low | Safe: static literal; add comment |
| 6 | `server/src/auth/cedar.rs:91` | `"File::\"unknown\"".parse().unwrap()` | Low | Safe: static literal; add comment |
| 7 | `server/src/auth/cedar.rs:246` | `Self::new().unwrap()` in `Default` impl | Info | Only called from tests; could use `#[cfg(test)]` |
| 8 | `web/src/components/header.rs:190,194,199` | `window().unwrap()` | Low | WASM: window always exists in browser; acceptable |
| 9 | `web/src/api.rs:218` | `Headers::new().unwrap()` | Low | Safe: infallible in WASM |
| 10 | `server/src/wopi.rs:360` | `serde_json::to_string(...).unwrap_or_default()` | Info | Already uses `unwrap_or_default`; safe |

### `.expect()` Calls in Production Code (non-test)

| # | Location | Expression | Severity | Recommendation |
|---|----------|------------|----------|----------------|
| 1 | `desktop/src/rclone.rs:136` | `child.stderr.take().expect("stderr should be piped")` | Low | Safe: configured with `Stdio::piped()` |
| 2 | `desktop/src/rclone.rs:137` | `child.stdout.take().expect("stdout should be piped")` | Low | Safe: configured with `Stdio::piped()` |
| 3 | `server/src/main.rs:342` | `ctrlc::set_handler(...).expect("Failed to install CTRL+C handler")` | Low | Acceptable: fatal startup error |

### `todo!()` / `unimplemented!()` / `panic!()` in Production

None found outside of tests. All `panic!()` calls are within `#[cfg(test)]` blocks.

---

## 3. Concurrency Safety

### `std::sync::Mutex`

None found. The codebase correctly uses `tokio::sync::RwLock` for async-safe locking.

### `unsafe` Blocks

No `unsafe` Rust blocks found. The only occurrence of the word "unsafe" is:
- `security_headers.rs:31`: CSP string `'unsafe-inline'` (not Rust `unsafe`)
- `oidc.rs:242`: Function name `decode_claims_unsafe` (naming convention, not Rust `unsafe`)

### `.clone()` on Large Types

No `.clone()` calls on `AppState`, `DashMap`, or `DashSet` found. State sharing uses `Arc<T>` correctly throughout.

### Deadlock Risk

The `LockManager` uses `DashMap` internally, which provides fine-grained per-key locking. No multiple-lock-acquisition patterns detected. The `LockManager` methods (`acquire_lock`, `release_lock`, `check_lock`) each only access the single `DashMap`. No deadlock risk identified.

---

## 4. API Contract Consistency

### Endpoints Bypassing `ApiError` (inconsistent error format)

Several endpoints return raw `(StatusCode, Json)` tuples instead of using `ApiError`, resulting in inconsistent error response shapes:

| # | Location | Endpoint | Issue | Severity |
|---|----------|----------|-------|----------|
| 1 | `wasm_upload.rs:56,67,83,95,106,128,139,152` | `upload_wasm_module` | Errors use `{"error": "..."}` but missing `error_code` field | Medium |
| 2 | `wasm_upload.rs:206,218,231,249` | `delete_wasm_module` | Same inconsistency | Medium |
| 3 | `workers.rs:91` | `register_worker` | `SERVICE_UNAVAILABLE` missing `error_code` | Low |
| 4 | `snapshots.rs:167` | `create_snapshot` | `INTERNAL_SERVER_ERROR` missing `error_code` | Medium |
| 5 | `snapshots.rs:219,234` | `delete/restore_snapshot` | `NOT_FOUND` missing `error_code` | Medium |
| 6 | `auth/oidc.rs:298` | OIDC middleware | Returns plain text `"Unauthorized"` instead of JSON | High |
| 7 | `auth/oidc.rs:302` | OIDC middleware | Returns plain text `"Missing Bearer token"` instead of JSON | High |
| 8 | `wopi.rs:287` | `put_file` | Success returns `(StatusCode::OK, "")` — empty body | Low (WOPI spec) |
| 9 | `wopi.rs:330` | `unlock_file` | Success returns `(StatusCode::OK, "")` — empty body | Low (WOPI spec) |
| 10 | `webdav.rs:83-87,168-174` | WebDAV catch-all | Uses `{"error": "..."}` instead of ApiError | Low (WebDAV uses XML/JSON hybrid) |

### Success Response Consistency

Most success responses correctly return `(StatusCode::OK, Json(body))`. The WOPI endpoints returning empty bodies are per the WOPI protocol specification and are acceptable.

---

## 5. Security Review

### Hardcoded Secrets

| # | Location | Finding | Severity | Recommendation |
|---|----------|---------|----------|----------------|
| 1 | `server/src/lib.rs:113` | Default WOPI token secret: `"ferro-wopi-token-secret-change-me"` | High | Fail startup if secret not overridden in production; add config validation |
| 2 | `server/src/lib.rs:112` | Default external_url: `"http://localhost:8080"` | Medium | Add warning if not overridden in production |
| 3 | `server/tests/integration.rs:35` | WOPI test secret | Info | Acceptable: clearly test-only |

### Timing Attack Vectors

| # | Location | Finding | Severity | Recommendation |
|---|----------|---------|----------|----------------|
| 1 | `server/src/shares.rs:165` | Share password comparison uses `==` (non-constant-time) | Medium | Use `subtle::ConstantTimeEq` for password comparison |

### Plaintext Password Storage

| # | Location | Finding | Severity | Recommendation |
|---|----------|---------|----------|----------------|
| 1 | `server/src/shares.rs:18` | Share passwords stored in plaintext in `ShareLink.password` | Medium | Hash passwords with bcrypt/argon2 before storage |

### Sensitive Data in URL Parameters

| # | Location | Finding | Severity | Recommendation |
|---|----------|---------|----------|----------------|
| 1 | `server/src/shares.rs:163` | Share password in query param `?password=...` | Medium | Acceptable for share links (one-time use, low-value target), but document the risk |

### Unsafe Function Exposed

| # | Location | Finding | Severity | Recommendation |
|---|----------|---------|----------|----------------|
| 1 | `server/src/auth/oidc.rs:242` | `decode_claims_unsafe` is `pub` (skips JWT signature verification) | High | Restrict visibility: `pub(crate)` or move to test module |

### Debug Output in Production

- `println!`: Only used in `cli/src/main.rs` (CLI tool output — acceptable). None in server/core/common.
- `dbg!`: None found anywhere.
- `eprintln!`: None found.
- All server logging uses `tracing` macros — correct.

### SQL Injection

All SQL queries in `core/src/persistence.rs` and `core/src/sqlx_metadata.rs` use parameterized queries via `sqlx::query(...).bind(...)`. No raw string interpolation in SQL. **No SQL injection vectors found.**

### Input Validation

- `wasm_upload.rs`: Validates filename (no path separators, `.wasm` extension), validates WASM magic bytes — good.
- `webdav.rs`: Path sanitization via `sanitize_path()` — good.
- `shares.rs`: Path construction uses `format!("/{}", path)` — potential path traversal if `path` contains `..`; however, the storage layer should handle this.
- Body size limit enforced in `webdav.rs:93` — good.

---

## 6. Code Organization

### File Sizes (God Modules)

| File | Lines | Assessment |
|------|-------|------------|
| `server/src/webdav.rs` | 1994 | **High** — handles OPTIONS, GET, PUT, DELETE, MKCOL, COPY, MOVE, PROPFIND, PROPPATCH, LOCK, UNLOCK, plus all tests. Should be split into handler submodules. |
| `web/src/components/file_browser.rs` | 1850 | **Medium** — single UI component; common for Leptos. Acceptable. |
| `server/tests/integration.rs` | 1790 | **Info** — test file; acceptable. |
| `web/src/api.rs` | 1081 | **Medium** — API client with many endpoints; could split by domain. |
| `server/src/wasm_upload.rs` | 594 | **Low** — upload handler + tests; acceptable. |

### Module Structure

- `server/src/lib.rs` exports **37 modules** all as `pub`. This exposes internal implementation details. Many of these (e.g., `conflict`, `xml`, `error`, `user_paths`, `preferences`) are internal implementation details that should be `pub(crate)`.
- No circular dependencies detected.
- Logical separation: `auth/`, API handlers, middleware, storage.

### Public API Surface

`server/src/lib.rs` re-exports everything as `pub`. Modules like `conflict`, `xml`, `error`, `user_paths`, `preferences`, `request_logging`, `security_headers` are internal implementation details that should not be part of the public API.

---

## 7. Documentation

### Missing Doc Comments on `pub` Items

**Severity**: Medium — the public API surface lacks documentation.

#### `ferro_common` (14 undocumented items)

| Location | Item |
|----------|------|
| `common/src/path.rs:3` | `pub fn normalize_path` |
| `common/src/path.rs:26` | `pub fn parent_path` |
| `common/src/path.rs:35` | `pub fn base_name` |
| `common/src/path.rs:43` | `pub fn is_collection_path` |
| `common/src/path.rs:47` | `pub fn validate_path` |
| `common/src/path.rs:51` | `pub fn join_path` |
| `common/src/metadata.rs:6` | `pub struct ContentHash` |
| `common/src/metadata.rs:9-40` | All `ContentHash` methods |
| `common/src/metadata.rs:52` | `pub struct FileMetadata` |
| `common/src/metadata.rs:65,80` | `FileMetadata::new`, `new_collection` |
| `common/src/webdav.rs:49-109` | `LockToken`, `LockInfo`, `WebDavProperty`, `MultiStatusResponse`, `MultiStatusItem` |
| `common/src/auth.rs:4` | `pub struct Claims` |
| `common/src/error.rs` | `pub enum FerroError` |

#### `ferro_core` (25+ undocumented items)

| Location | Item |
|----------|------|
| `core/src/wasm.rs:11,30,39,46` | `WorkerConfig`, `WorkerResult`, `WorkerEvent`, `WasmWorkerRuntime` |
| `core/src/storage.rs:13,19` | `InMemoryStorageEngine` + `new()` |
| `core/src/search.rs:10,25,32-163` | `SearchEngine`, `SearchResult`, all methods |
| `core/src/sqlx_metadata.rs:10,49,139,172` | `PgMetadataStore`, `SqliteMetadataStore` |
| `core/src/presigned.rs:19,51,69,111,153` | All presigned generators |
| `core/src/cas.rs:19,24` | `InMemoryCasStore` |
| `core/src/persistence.rs:45,54,62,76` | Persistence structs |

#### `ferro_server` (30+ undocumented items)

| Location | Item |
|----------|------|
| `server/src/lib.rs:65` | `pub struct AppState` + all builder methods |
| `server/src/lock.rs:9` | `pub struct LockManager` + all methods |
| `server/src/activity.rs:9,18,24` | `ActivityEntry`, `ActivityResponse`, `ActivityParams` |
| `server/src/quota.rs:9,40,50,68` | `QuotaInfo`, `check_quota`, `record_usage`, `parse_human_size` |
| `server/src/rate_limit.rs:18` | `RateLimiterConfig` |
| `server/src/shares.rs:15,26,33` | `ShareLink`, `CreateShareRequest`, `ShareStore` |
| `server/src/snapshots.rs:14,25,86,94,155` | Snapshot types |
| `server/src/workers.rs:43,52` | `RegisterWorkerRequest/Response` |
| `server/src/wopi.rs:14,19,33` | WOPI types |
| `server/src/policies.rs:29,61` | Policy request types |
| `server/src/config.rs:30,57` | `FileConfig`, `ServerConfig` |
| `server/src/api.rs:167,172` | `LoginParams`, `CallbackParams` |

### Well-Documented Areas

- `server/src/workers.rs`: Handler functions have `///` doc comments with endpoint paths
- `server/src/wopi.rs`: Token issuance function well-documented
- `server/src/api_error.rs`: Error codes are self-documenting via constants
- `core/src/persistence.rs:154`: Pool accessor has doc comment

---

## Summary

| Category | Critical | High | Medium | Low | Info |
|----------|----------|------|--------|-----|------|
| Dead Code | 0 | 0 | 0 | 3 | 16 |
| Error Handling | 0 | 0 | 1 | 8 | 2 |
| Concurrency Safety | 0 | 0 | 0 | 0 | 0 |
| API Contract | 0 | 2 | 4 | 3 | 0 |
| Security | 0 | 2 | 3 | 2 | 2 |
| Code Organization | 0 | 0 | 1 | 2 | 1 |
| Documentation | 0 | 0 | 3 | 0 | 0 |
| **Total** | **0** | **4** | **12** | **15** | **21** |

### Priority Actions

1. **[High] Restrict `decode_claims_unsafe` visibility** — `server/src/auth/oidc.rs:242`. Change to `pub(crate)` to prevent accidental use in production auth flows.
2. **[High] Fix OIDC middleware plain-text error responses** — `server/src/auth/oidc.rs:298,302`. Return JSON via `ApiError::unauthorized()` for consistent API errors.
3. **[High] Fail startup on default WOPI secret in production** — `server/src/lib.rs:113`. Add config validation that refuses to start with the default secret.
4. **[High] Use constant-time comparison for share passwords** — `server/src/shares.rs:165`. Use `subtle::ConstantTimeEq`.
5. **[Medium] Migrate error responses in `wasm_upload.rs`, `snapshots.rs`, `workers.rs` to use `ApiError`** for consistent `error_code` field.
6. **[Medium] Add doc comments to all `pub` items** in `ferro_common`, `ferro_core`, and `ferro_server`.
7. **[Medium] Hash share passwords** instead of storing plaintext — `server/src/shares.rs:18`.
8. **[Medium] Split `webdav.rs`** (1994 lines) into submodules by HTTP method or resource type.
9. **[Medium] Reduce module visibility** in `server/src/lib.rs` — change internal modules from `pub` to `pub(crate)`.
10. **[Low] Fix potential panic in `shares.rs:199`** — use `unwrap_or_else` for header value construction.
