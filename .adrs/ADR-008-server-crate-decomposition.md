# ADR-008: Server Crate Decomposition via Trait-Based AppState Refactoring

## Status: Proposed

## Date: 2026-06-29

## Context

`crates/server/src/lib.rs` contains 2,877 lines with 140+ `pub mod` declarations and a monolithic `AppState` struct (135 fields). Every module depends on `AppState` directly, making it impossible to extract modules into separate crates without creating a circular dependency on `ferro-server`.

The existing `ferro-server-admin` crate was an incomplete attempt at decomposition: it depends on `ferro-server` for `AppState` and `ApiError`, and duplicated `backup.rs` (1,526 lines, now removed). The fundamental blocker is `AppState` -- no module can become a separate crate unless it depends only on traits, not on `AppState`.

## Decision

Implement a phased decomposition using the **trait-based inversion** pattern:

### Phase 0: Define Core Traits in `ferro-common`

Define focused traits in `ferro-common/src/server_context.rs` that abstract the most-depended-upon `AppState` fields:

```rust
pub trait HasStorage {
    fn storage(&self) -> &Arc<dyn StorageEngine>;
}

pub trait HasAudit {
    fn audit_log(&self) -> &Arc<dyn AuditLog>;
}

pub trait HasDb {
    fn db(&self) -> &Option<DbHandle>;
}

pub trait HasSearch {
    fn search(&self) -> &Option<SearchEngine>;
}
```

### Phase 1: Refactor Handler Signatures (incremental, module-by-module)

Change handler functions from:
```rust
async fn handler(State(state): State<AppState>) -> impl IntoResponse
```
to:
```rust
async fn handler<S: HasStorage + HasAudit>(State(state): State<S>) -> impl IntoResponse
```

**Critical constraint:** Do NOT change all 140 modules at once. Refactor one module at a time, running tests after each change. Start with zero-dependency leaf modules.

### Phase 2: Extract Modules into Separate Crates

Once a module uses only traits, it can depend on `ferro-common` instead of `ferro-server`:

| Order | Crate | Modules | Est. Lines |
|-------|-------|---------|------------|
| 1 | `ferro-server-storage-ops` | storage, streaming_upload, read_cache, quota, dedup | ~2,500 |
| 2 | `ferro-server-security-middleware` | security, security_headers, simple_auth, request_id | ~1,500 |
| 3 | `ferro-server-content` | xml, range_get, ocr_engine, watermark_api, e2ee, encryption | ~3,000 |
| 4 | `ferro-server-plugins` | plugin_permissions, plugin_marketplace_api, wasm_upload, workers | ~2,500 |
| 5 | `ferro-server-collaboration` | chat_api, collab_ws, whiteboard_api, comments, tags | ~2,800 |
| 6 | `ferro-server-sharing` | shares, shares_ext, favorites | ~2,000 |
| 7 | `ferro-server-compliance` | retention, worm, dlp_api, antivirus_api, clamav | ~3,000 |
| 8 | `ferro-server-user-mgmt` | users, user_api, guests, account_api, totp_api | ~3,000 |
| 9 | `ferro-server-admin-api` | admin_api, backup, branding, gdpr, dashboard, activity | ~5,000 |
| 10 | `ferro-server-integrations` | mail_api, push_notifications, offline_api, remote_mount | ~3,500 |
| 11 | `ferro-server-webdav-core` | webdav, dav, lock, move_copy, sync | ~6,000 |
| 12 | `ferro-server-api-core` | api, search, events, event_triggers, webhooks | ~5,000 |

### Phase 3: Slim Down `lib.rs`

After extraction, `lib.rs` shrinks to ~500 lines:
- `AppState` definition (~135 lines)
- `build_router()` composing extracted crate routers (~200 lines)
- Health/readiness endpoints (~100 lines)

## Alternatives Considered

1. **Monolithic extraction without traits** -- rejected because every extracted crate would need `ferro-server` as a dependency, defeating the purpose.
2. **Full rewrite** -- rejected as too risky for a production system with 2500+ tests.
3. **Extract only admin modules** -- partially done (server-admin exists), but the root cause (AppState coupling) remains.

## Consequences

- **Positive:** Faster incremental builds, clearer module boundaries, reusability, testability
- **Negative:** Significant refactoring effort (estimated 2-3 weeks), temporary increase in complexity during transition
- **Risks:** Handler signature changes could introduce subtle behavioral differences; must maintain full test coverage throughout

## Related Standards

- IEEE 1016-2009 (Software Design Descriptions)
- ISO/IEC 12207 (Software Life Cycle Processes)

## Related ADRs

None (this is the first ADR).

## Author

Nexus (Principal Systems Architect)
