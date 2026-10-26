# Estate Adoption — WyattAu repos assessed for Ferro (R5)

> Assessment of 2026-10-22: 30 repos scanned, 12 relevant, 6 adopted,
> 3 deferred with rationale, rest not applicable. Validkit was already
> integrated (`common`, `server-security`).

## Adopted (this round)

| Repo | Version | Where | What it replaces/adds |
|---|---|---|---|
| `healthkit` | 1.3 (crates.io) | `server-health/src/healthkit.rs`, `/healthz/detailed` route | Subsystem registry with per-check timeouts, parallel execution, error detail, detailed JSON route. Ferro's k8s probe contracts (`/healthz`, `/readyz`, `/startupz`) unchanged. |
| `webhookkit` | 2.2 | `server-activitypub` inbox | `ReplayGuard` (1 h TTL) on activity ids — duplicate federation deliveries are acknowledged `200 duplicate` without re-applying (applied-set contract of YP-EVENT applied inbound). |
| `sheet-xlsx` | 0.1 | `server/src/indexer.rs` | XLSX cell-text extraction → Tantivy content index (spreadsheets were metadata-only before). |
| `aperture-vault` (patterns) | — | `core/src/persistence.rs` | CAS orphan sweep (`cas_gc`: age cutoff + metadata/snapshot/extra-root protection, dry-run default) and `verify_cas` bit-rot detection — **Ferro's CAS previously only grew**. |
| `aperture-vault` (patterns) | — | (same) | Per-aperture: swept rows need a `created_at` age threshold; sync manifest block hashes are NOT tracked table-side and must be passed as `extra_roots`. |
| `SIEVE`/estate patterns (R4 carryover) | — | — | See R4 synthesis. |

## Deferred (with rationale)

| Repo | Why deferred |
|---|---|
| `axum-stack` | **RESOLVED R6 — no adoption, fully assessed.** Piece-by-piece: request-ID → *redundant* (Ferro's `server/src/request_id.rs` honors inbound `X-Request-Id` and inserts into extensions — a superset of axum-stack's generate+propagate); compression → *redundant* (tower-http `CompressionLayer` already in the stack); CORS → *do not swap* (Ferro's hand-rolled `cors_layer` is order-coupled to `csrf_middleware` preflight exemptions at `routes.rs:1330-1343`; a tower-http CorsLayer silently changes preflight/credential semantics); timeout → *not applicable as-is* (a global 30 s `TimeoutLayer` breaks WebDAV large transfers — the only adoptable idea is a **per-route timeout for API/auth routes**, R7 candidate); health mount → *redundant* (R5 healthkit integration covers it). Ferro's middleware order is a documented security invariant: cors → auth_guard → state_gate → guest_expiry → csrf → simple_auth → auth → cedar → rate_limit (request order, layers apply bottom-up). |
| `fetch-kit` | Retry/circuit-breaker would **duplicate Ferro's formally verified breaker** (`CircuitBreakerLiveness.lean`) and the outbox lease-retry. Use only for net-new outbound integrations; never as a replacement. |
| `telemetry-init` | Wires `metrics-kit` + OTLP; Ferro's `observability` crate has its own registry and loki/victoria exporters. Only relevant under a deliberate observability migration. |
| `chisel-doc` | Pre-alpha PDF engine — watch for a future preview/extraction worker. |
| `EvergreenShims` | Docker healthcheck shims — deploy nicety, not code. |

## Upstream state (checked 2026-10-22; updated same session)

1. **healthkit: the inverted `check_readiness` comparator is already fixed upstream** — and **crates.io serves the fixed 1.3.1**, so Ferro now depends on the published crate directly (`healthkit = "1.2"` resolves 1.3.1); the temporary git pin was removed. No publish action needed — 1.3.1 was already live.
2. Ferro keeps a local `overall_status` aggregate (same `max_by_key` semantics) as a belt-and-braces contract on the exact JSON the detailed route serves; the dead-database test asserts upstream `check_readiness` agrees.
3. **`webhookkit` local clone is stale**: `~/dev/src/github.com/WyattAu/webhookkit` main is **behind origin by 11 commits and has 1 unpushed local commit** (`01404fb test: adversarial verification suite + redis replay guard`). Ferro uses crates.io 2.2.x and is unaffected; the clone needs a `git pull --rebase` decision from you.
4. `sheet-xlsx` local (0.1.1) is in sync with origin and matches what Ferro uses. API worked exactly as documented.

## Not applicable

`signalkit`, `trendkit`, `contact-miner`, `simd-tokenizer`, `model-router`, `skillkit`, `kestrel`, `vane` (Go), `continuwuity` (interop case study at most), `leptos-leaflet`/`leptos-capital-markers` (not used by ferro-web), `estate-site`/`uofg_sml_soc_docs`/`starlight-sites`, `engineering-standards`, `estate-integration`, `EvergreenImageRegistry`, `crawkit`.

## RESOLVED (R6): sync-protocol test hangs — mutex re-entry deadlock

Root cause: `SyncStateManager::get_or_create` held the `conn` std-Mutex guard across the `QueryReturnedNoRows` arm, which calls `self.save` — re-locking the same mutex deadlocked the calling thread. `test_save_and_get` passed only because the row pre-existed. Production impact: `SyncEngine::sync_with_peer` → `record_success` → `get_or_create` on a **first-contact peer deadlocked the sync path**. Fix: scope the guard to the query block (`state.rs::get_or_create`); regression test `test_get_or_create_concurrent_first_contact` (8 threads, fresh manager). Full suite now 30 passed in 0.01s — the `--skip state --skip engine` quarantine is removed.

## R6: simple-auth deployment mode was broken end-to-end (fixed via litmus work)

Setting up the litmus run exposed two production bugs in the `FERRO_ADMIN_USER` (Basic-auth, no-OIDC) deployment mode:

1. **Identity clobbering** (`crates/auth/src/oidc.rs::auth_middleware`): with no OIDC validator, the middleware inserted `Claims::anonymous()` for every non-public path — after `simple_auth_middleware` had already authenticated the Basic credentials. Downstream Cedar then denied every authenticated request (principal `anonymous` ≠ `User::"admin"`). Fix: map the authenticated `UserInfo` into `Claims` (sub = username) on the no-OIDC path; regression test `test_simple_auth_user_info_becomes_claims`.
2. **Password rotation locked out** (`crates/server-api/src/lib.rs::auth_change_password_impl`): the OIDC-caller guard fired on any non-anonymous `Claims`, so (with fix 1) the simple-auth admin could never rotate the mandatory default password. Fix: the guard now requires `state.oidc().is_some()` — credentials are IdP-managed only when OIDC is actually configured.
3. **Dead config flag**: `--allow-open-authz` / `FERRO_ALLOW_OPEN_AUTHZ` is defined in `server-config` but **consumed nowhere** — its own doc claims it is "required for public/simple_auth deployments". Until it is wired (or removed), simple-auth deployments need an explicit Cedar policy file permitting the admin principal.

Ferro's middleware order is a security invariant worth restating (request order; axum layers apply bottom-up): cors → auth_guard → state_gate → guest_expiry → csrf → simple_auth → oidc_auth → cedar → rate_limit.
