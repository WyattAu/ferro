# Module Extraction Decisions — Remaining Server Modules

Three modules remain in `crates/server/src/` after three phases of extraction. Each was evaluated against extraction criteria and decided to stay. This document records the reasoning and criteria for future decisions.

---

## Extraction Criteria

A module is a **good extraction candidate** when:
1. It depends on ≤2 server-internal types (ideally zero)
2. Its domain types are self-contained (no `AppState` fields in structs)
3. It has a clear trait boundary that could become a crate API
4. Lines of code justify the crate overhead (~200+ lines)
5. No duplication of types would be required

A module **must stay** when:
1. It directly reads/writes `AppState` fields (especially atomic counters or inner stores)
2. It contains server-specific axum handlers that need `State<AppState>`
3. Its types are defined in the server crate and referenced by `AppState` (creating a circular dependency if extracted)
4. Extraction would require duplicating 3+ types just to bridge the boundary
5. The line count doesn't justify a new crate (~<150 lines of business logic)

---

## Module Analysis

### 1. `notification_prefs_api.rs` — 242 lines

**Decision:** Stay. Not worth extracting.

**Dependencies:**
| Type | Source |
|------|--------|
| `AppState` | `crate::AppState` — accessed via `state.notification_prefs_store` |
| `ApiError` | `crate::api_error` — for error responses |
| `DbHandle` | `crate::db` — `Arc<Mutex<Connection>>` for SQLite access |

**Why it stays:**
- `NotificationPrefsStore` is already a named field on `AppState` (`state.rs:116`). Extracting it would require defining the store in an external crate, then re-exporting the type back into the server crate to keep it on `AppState` — a circular dependency.
- The two handler functions (`get_notification_prefs`, `update_notification_prefs`) take `State<AppState>` and access the store directly. These are thin axum handlers — extracting them means the external crate would need an axum dependency and the `AppState` trait bound.
- The domain types (`NotificationPrefs`, `UpdateNotificationPrefsRequest`) are simple data structs that could live anywhere, but they're only 23 lines. Not enough to justify a crate boundary.

**What it would take to extract:**
- Define `NotificationPrefsStore` in a new `ferro-server-notification-prefs` crate
- Create a trait in that crate for the store interface
- Define an `AppState`-like trait in the external crate that the server implements
- Re-export the store type and implement the trait on the server's `AppState`
- Total effort: ~200 lines of glue code for a 242-line module. Not justified.

**Effort/line count ratio:** High effort, low payoff.

---

### 2. `link_analytics_api.rs` — 305 lines

**Decision:** Stay. Deep coupling to multiple server stores.

**Dependencies:**
| Type | Source |
|------|--------|
| `AppState` | `crate::AppState` — accessed via `state.share_store`, `state.db`, `state.used_bytes` |
| `ApiError` | `crate::api_error` — for error responses |
| `ShareStoreTrait` | `crate::shares` — `state.share_store.list()` |
| `DbHandle` | `crate::db` — direct SQLite queries against `link_analytics` and `shares` tables |

**Why it stays:**
- The handlers (`list_link_analytics`, `analytics_link_stats`, `analytics_overview`) access **three** different `AppState` fields: `share_store`, `db`, and `used_bytes`. This is the deepest coupling of the three modules.
- `track_link_access` is a middleware function called from share-serving code. It takes a raw `&rusqlite::Connection` — already decoupled from `AppState`, but the handlers are not.
- The `analytics_overview` handler joins the `link_analytics` table with the `shares` table (line 272). This cross-table query ties it to the share schema, which lives in the server crate.
- `link_analytics_api.rs` defines 5 domain types (`LinkAnalyticsEntry`, `LinkStats`, `ReferrerCount`, `DailyCount`, `AnalyticsOverview`, `TopLink`) — all public structs. These would need to move to an external crate, but they're only used by these handlers. No other crate references them.

**What it would take to extract:**
- Move 6 domain structs to a `ferro-server-link-analytics` crate
- Create a trait for `ShareStore` access (already exists as `ShareStoreTrait`)
- Create a trait for DB access — but this module uses raw `rusqlite::Connection` directly, not through `DbHandle`. Would need to abstract over raw SQL queries, which is heavy.
- The `shares` table join means the external crate would depend on the server's share schema. This creates a dependency inversion problem.
- Total effort: ~300 lines of trait definitions + adapter code. Not justified.

**Effort/line count ratio:** High effort, moderate payoff (305 lines, but 5 types + deep coupling).

---

### 3. `prometheus_metrics.rs` — 248 lines

**Decision:** Stay. Impossible to extract without a massive trait abstraction.

**Dependencies:**
| Type | Source |
|------|--------|
| `AppState` | `crate::AppState` — reads **12+ fields** directly |
| `started_at` | `std::time::Instant` |
| `storage` | `Arc<dyn StorageEngine>` |
| `request_count` | `Arc<AtomicU64>` |
| `request_duration_buckets` | `Arc<[AtomicU64; 11]>` |
| `request_duration_sum_ms` | `Arc<AtomicU64>` |
| `request_status_counts` | `Arc<[AtomicU64; 4]>` |
| `storage_op_counts` | `Arc<[AtomicU64; 6]>` |
| `wasm_runtime` | `Option<Arc<WasmWorkerRuntime>>` |
| `read_cache` | `Arc<ReadCache>` |
| `wasm_dispatch_count` | `Arc<AtomicU64>` |
| `wasm_error_count` | `Arc<AtomicU64>` |
| `wasm_fuel_total` | `Arc<AtomicU64>` |
| `used_bytes` | `Arc<AtomicU64>` |

**Why it stays:**
- This handler reads from **12+ `AppState` fields** — more than any other module. It is essentially a snapshot of the entire server state rendered as Prometheus text.
- The handler formats the output as a single string literal with all metrics. There's no intermediate data model — it's pure formatting logic over raw atomic reads.
- Extracting this would require a `MetricsProvider` trait with 12+ associated methods (one per metric family). The server would need to implement this trait, and the external crate would just call those methods and format the output.
- The test suite (lines 156-248) calls `build_router(AppState::in_memory())` and makes HTTP requests. This integration test lives in the server crate. Moving the handler to an external crate would either duplicate the test setup or require the test to depend on the server crate — defeating the purpose.
- The actual "business logic" is just reading atomic counters and formatting strings. There's no domain model, no database queries, no type definitions beyond the handler function itself.

**What it would take to extract:**
- Define a `MetricsProvider` trait with ~15 methods in an external crate
- Implement the trait for `AppState` in the server crate (15 method implementations)
- Move the `prometheus_metrics_handler` function to the external crate, parameterized over the trait
- Move or duplicate 4 integration tests
- Total effort: ~200 lines of trait + impl code + test migration. For a module that's essentially a single format string.

**Effort/line count ratio:** Extremely high effort, near-zero payoff.

---

## Remaining Local Module Inventory

All 85 modules in `crates/server/src/` categorized:

### Core (must stay)

These modules define the server's fundamental infrastructure and cannot be extracted:

| Module | Reason |
|--------|--------|
| `lib.rs` | Crate root, module declarations |
| `main.rs` | Entry point |
| `config.rs` | Server configuration types |
| `state.rs` | `AppState` definition — the central hub |
| `routes.rs` | Axum route tree wiring |
| `handlers.rs` | Core HTTP handlers (methods, WebDAV) |
| `api.rs` | REST API endpoint definitions |
| `openapi.rs` | OpenAPI spec generation |
| `db.rs` | `DbHandle` type alias, migrations |
| `error.rs` | Error types |
| `api_error.rs` | `ApiError` type for HTTP responses |
| `request_id.rs` | Request ID middleware |
| `request_logging.rs` | Request logging middleware |
| `json_logging.rs` | Structured JSON logging |
| `security_headers.rs` | Security header middleware |
| `metrics.rs` | Metrics middleware (request counting) |
| `xml.rs` | WebDAV XML response helpers |

### Entangled (must stay due to deep coupling)

These modules access 3+ `AppState` fields or define types that are referenced by `AppState`:

| Module | Lines | Coupling |
|--------|-------|----------|
| `shares.rs` | ~500+ | `ShareStoreTrait` is on `AppState`; `ShareLink` type used everywhere |
| `users.rs` | ~400+ | `UserStoreTrait` is on `AppState`; `UserRole`, `UserInfo` used throughout |
| `audit.rs` | ~200+ | `AuditLog` on `AppState`; used by all adapters |
| `comments.rs` | ~300+ | `CommentStore` on `AppState`; `CollaborationState` trait |
| `favorites.rs` | ~200+ | `FavoriteStore` on `AppState` |
| `collab_ws.rs` | ~400+ | `CollabRoomManager` on `AppState`; WebSocket handling |
| `sync/` | ~500+ | `SyncStore` on `AppState`; offline-first sync logic |
| `shares_ext.rs` | ~200+ | Extension methods on share types |
| `preferences.rs` | ~150+ | `PreferenceStore` on `AppState` |
| `indexer.rs` | ~300+ | Indexes files using `AppState` storage |
| `streaming.rs` | ~200+ | Upload streaming; uses `AppState` thresholds |
| `upload.rs` | ~200+ | `UploadStore` on `AppState` |
| `trash/` | ~200+ | `TrashedEntry` on `AppState` |
| `presigned.rs` | ~150+ | Uses `presigned_generator` from `AppState` |
| `idempotency.rs` | ~150+ | `IdempotencyStore` on `AppState` |
| `ws.rs` | ~200+ | WebSocket connections; uses `ws_manager` |
| `worker_runner.rs` | ~200+ | WASM worker execution; uses `wasm_runtime` |
| `workers.rs` | ~200+ | Worker management; uses `AppState` |
| `object_store_backend.rs` | ~200+ | S3 backend; uses `AppState` storage config |
| `email.rs` | ~150+ | Uses `email_config` from `AppState` |
| `security.rs` | ~200+ | Security middleware; uses `AppState` auth fields |
| `simple_auth.rs` | ~200+ | Auth; uses `AppState` admin credentials |
| `ldap_auth.rs` | ~200+ | LDAP auth; uses `AppState` config |
| `ransomware.rs` | ~150+ | `RansomwareDetector` on `AppState` |
| `quota.rs` | ~150+ | Uses `quota_bytes`, `used_bytes` from `AppState` |
| `storage_health.rs` | ~150+ | Uses `AppState` storage |
| `metadata_replication.rs` | ~150+ | Uses `AppState` metadata store |
| `federation.rs` | ~200+ | Federation; uses `AppState` federation fields |
| `federation_sync.rs` | ~200+ | Federation sync; uses `AppState` |
| `events.rs` | ~200+ | Event bus; uses `AppState` event_bus |
| `triggers.rs` | ~150+ | DB triggers; uses `AppState` |
| `snapshots.rs` | ~200+ | Snapshot management; uses `AppState` snapshot_store |
| `conflict.rs` | ~150+ | Conflict resolution; uses `AppState` sync fields |
| `selective_sync_api.rs` | ~200+ | Uses `AppState` selective_sync_store |
| `whiteboard_api.rs` | ~200+ | Uses `AppState` storage |
| `totp_api.rs` | ~150+ | 2FA; uses `AppState` db |
| `webauthn_api.rs` | ~150+ | WebAuthn; uses `AppState` webauthn_store |
| `api_keys_routes.rs` | ~150+ | API keys; uses `AppState` api_key_store |
| `plugin_marketplace_api.rs` | ~200+ | Uses `AppState` plugin fields |
| `plugin_permissions.rs` | ~150+ | Uses `AppState` plugin registry |
| `calendar_api.rs` | ~200+ | Uses `AppState` calendar_store |
| `contacts_api.rs` | ~200+ | Uses `AppState` address_book_store |
| `notes_api.rs` | ~200+ | Uses `AppState` db |
| `tasks_api.rs` | ~200+ | Uses `AppState` task_store |
| `ai_search.rs` | ~200+ | Uses `AppState` ai_search |
| `ocr.rs` | ~150+ | Uses `AppState` storage |
| `thumbnail_cache.rs` | ~150+ | `ThumbnailCache` on `AppState` |
| `thumbnails.rs` | ~200+ | Uses `AppState` thumbnail fields |
| `batch.rs` | ~200+ | Uses `AppState` storage + other stores |
| `bulk.rs` | ~200+ | Uses `AppState` storage |
| `policies.rs` | ~150+ | Authorization policies; uses `AppState` cedar |
| `redis_lock.rs` | ~150+ | Redis distributed lock; uses `AppState` config |
| `redis_rate_limiter.rs` | ~150+ | Redis rate limiting; uses `AppState` config |
| `tenant_rate_limit_api.rs` | ~200+ | Uses `AppState` tenant fields |
| `wasm_hot_reload.rs` | ~150+ | Uses `AppState` wasm fields |
| `offline_wiring.rs` | ~150+ | Uses `AppState` offline fields |
| `pg_state.rs` | ~150+ | Postgres state; uses `AppState` |
| `fs_util.rs` | ~150+ | Filesystem utilities; uses `AppState` storage |

### Candidate for extraction (moderate effort)

These modules could be moved with moderate refactoring:

| Module | Lines | Why extractable |
|--------|-------|-----------------|
| `notification_prefs_api.rs` | 242 | Self-contained domain; store is a simple wrapper. See analysis above — moderate effort but low payoff. |
| `link_analytics_api.rs` | 305 | Self-contained domain types. Could be extracted if share_store access is abstracted via trait. See analysis above. |
| `prometheus_metrics.rs` | 248 | Pure formatting over atomic reads. Could be extracted with a metrics provider trait, but the trait would be trivial and not reused. |

### Already extracted (reference)

These were moved to external crates in earlier phases:
- `ferro-server-admin-api` (branding, GDPR, admin operations)
- `ferro-server-user-mgmt` (user CRUD, guests, devices)
- `ferro-server-collaboration` (tags, comments, collab)
- `ferro-server-compliance` (retention, DLP, WORM)
- `ferro-server-content` (watermarks, file operations)
- `ferro-server-productivity` (tasks, notes)
- `ferro-server-security-middleware` (auth tracking, rate limiting)
- `ferro-server-storage-ops` (streaming, dedup, chunking)
- `ferro-server-storage-utils` (snapshots, thumbnails, health)
- `ferro-server-webdav-core` (locks, trash, PROPFIND)
- `ferro-server-integrations` (mail, push, remote mounts, read cache)
- `ferro-server-activitypub` (ActivityPub store)
- `ferro-server-webrtc` (WebRTC signaling)
- `ferro-server-api-core` (webhooks, email, WS manager)

---

## Summary

| Module | Lines | Decision | Reason |
|--------|-------|----------|--------|
| `notification_prefs_api.rs` | 242 | Stay | `NotificationPrefsStore` is on `AppState`; handlers need `State<AppState>`; 3 type dependencies |
| `link_analytics_api.rs` | 305 | Stay | Accesses `share_store`, `db`, `used_bytes`; cross-table joins with `shares`; 5 domain types used only here |
| `prometheus_metrics.rs` | 248 | Stay | Reads 12+ `AppState` fields; pure formatting logic; no domain types to extract; tests use `build_router` |

**Total remaining:** ~795 lines across 3 modules. None justifies extraction given the coupling depth and effort required.
