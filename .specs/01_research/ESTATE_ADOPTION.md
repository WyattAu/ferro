# Estate Adoption: WyattAu repos assessed for Ferro (R5)

> Assessment of 2026-10-22: 30 repos scanned, 12 relevant, 6 adopted,
> 3 deferred with rationale, rest not applicable. Validkit was already
> integrated (`common`, `server-security`).

## Adopted (this round)

| Repo | Version | Where | What it replaces/adds |
|---|---|---|---|
| `healthkit` | 1.3 (crates.io) | `server-health/src/healthkit.rs`, `/healthz/detailed` route | Subsystem registry with per-check timeouts, parallel execution, error detail, detailed JSON route. Ferro's k8s probe contracts (`/healthz`, `/readyz`, `/startupz`) unchanged. |
| `webhookkit` | 2.2 | `server-activitypub` inbox | `ReplayGuard` (1 h TTL) on activity ids; duplicate federation deliveries are acknowledged `200 duplicate` without re-applying (applied-set contract of YP-EVENT applied inbound). |
| `sheet-xlsx` | 0.1 | `server/src/indexer.rs` | XLSX cell-text extraction → Tantivy content index (spreadsheets were metadata-only before). |
| `aperture-vault` (patterns) | - | `core/src/persistence.rs` | CAS orphan sweep (`cas_gc`: age cutoff + metadata/snapshot/extra-root protection, dry-run default) and `verify_cas` bit-rot detection; **Ferro's CAS previously only grew**. |
| `aperture-vault` (patterns) | - | (same) | Per-aperture: swept rows need a `created_at` age threshold; sync manifest block hashes are NOT tracked table-side and must be passed as `extra_roots`. |
| `SIEVE`/estate patterns (R4 carryover) | - |; | See R4 synthesis. |

## Deferred (with rationale)

| Repo | Why deferred |
|---|---|
| `axum-stack` | **RESOLVED R6; no adoption, fully assessed.** Piece-by-piece: request-ID → *redundant* (Ferro's `server/src/request_id.rs` honors inbound `X-Request-Id` and inserts into extensions; a superset of axum-stack's generate+propagate); compression → *redundant* (tower-http `CompressionLayer` already in the stack); CORS → *do not swap* (Ferro's hand-rolled `cors_layer` is order-coupled to `csrf_middleware` preflight exemptions at `routes.rs:1330-1343`; a tower-http CorsLayer silently changes preflight/credential semantics); timeout → *not applicable as-is* (a global 30 s `TimeoutLayer` breaks WebDAV large transfers; the only adoptable idea is a **per-route timeout for API/auth routes**, R7 candidate); health mount → *redundant* (R5 healthkit integration covers it). Ferro's middleware order is a documented security invariant: cors → auth_guard → state_gate → guest_expiry → csrf → simple_auth → auth → cedar → rate_limit (request order, layers apply bottom-up). |
| `fetch-kit` | Retry/circuit-breaker would **duplicate Ferro's formally verified breaker** (`CircuitBreakerLiveness.lean`) and the outbox lease-retry. Use only for net-new outbound integrations; never as a replacement. |
| `telemetry-init` | Wires `metrics-kit` + OTLP; Ferro's `observability` crate has its own registry and loki/victoria exporters. Only relevant under a deliberate observability migration. |
| `chisel-doc` | Pre-alpha PDF engine; watch for a future preview/extraction worker. |
| `EvergreenShims` | Docker healthcheck shims; deploy nicety, not code. |

## Upstream state (checked 2026-10-22; updated same session)

1. **healthkit: the inverted `check_readiness` comparator is already fixed upstream**; and **crates.io serves the fixed 1.3.1**, so Ferro now depends on the published crate directly (`healthkit = "1.2"` resolves 1.3.1); the temporary git pin was removed. No publish action needed; 1.3.1 was already live.
2. Ferro keeps a local `overall_status` aggregate (same `max_by_key` semantics) as a belt-and-braces contract on the exact JSON the detailed route serves; the dead-database test asserts upstream `check_readiness` agrees.
3. **`webhookkit` local clone is stale**: `~/dev/src/github.com/WyattAu/webhookkit` main is **behind origin by 11 commits and has 1 unpushed local commit** (`01404fb test: adversarial verification suite + redis replay guard`). Ferro uses crates.io 2.2.x and is unaffected; the clone needs a `git pull --rebase` decision from you.
4. `sheet-xlsx` local (0.1.1) is in sync with origin and matches what Ferro uses. API worked exactly as documented.

## Not applicable

`signalkit`, `trendkit`, `contact-miner`, `simd-tokenizer`, `model-router`, `skillkit`, `kestrel`, `vane` (Go), `continuwuity` (interop case study at most), `leptos-leaflet`/`leptos-capital-markers` (not used by ferro-web), `estate-site`/`uofg_sml_soc_docs`/`starlight-sites`, `engineering-standards`, `estate-integration`, `EvergreenImageRegistry`, `crawkit`.

## RESOLVED (R6): sync-protocol test hangs: mutex re-entry deadlock

Root cause: `SyncStateManager::get_or_create` held the `conn` std-Mutex guard across the `QueryReturnedNoRows` arm, which calls `self.save`; re-locking the same mutex deadlocked the calling thread. `test_save_and_get` passed only because the row pre-existed. Production impact: `SyncEngine::sync_with_peer` → `record_success` → `get_or_create` on a **first-contact peer deadlocked the sync path**. Fix: scope the guard to the query block (`state.rs::get_or_create`); regression test `test_get_or_create_concurrent_first_contact` (8 threads, fresh manager). Full suite now 30 passed in 0.01s; the `--skip state --skip engine` quarantine is removed.

## R6: simple-auth deployment mode was broken end-to-end (fixed via litmus work)

Setting up the litmus run exposed two production bugs in the `FERRO_ADMIN_USER` (Basic-auth, no-OIDC) deployment mode:

1. **Identity clobbering** (`crates/auth/src/oidc.rs::auth_middleware`): with no OIDC validator, the middleware inserted `Claims::anonymous()` for every non-public path; after `simple_auth_middleware` had already authenticated the Basic credentials. Downstream Cedar then denied every authenticated request (principal `anonymous` ≠ `User::"admin"`). Fix: map the authenticated `UserInfo` into `Claims` (sub = username) on the no-OIDC path; regression test `test_simple_auth_user_info_becomes_claims`.
2. **Password rotation locked out** (`crates/server-api/src/lib.rs::auth_change_password_impl`): the OIDC-caller guard fired on any non-anonymous `Claims`, so (with fix 1) the simple-auth admin could never rotate the mandatory default password. Fix: the guard now requires `state.oidc().is_some()`; credentials are IdP-managed only when OIDC is actually configured.
3. **Dead config flag**: `--allow-open-authz` / `FERRO_ALLOW_OPEN_AUTHZ` is defined in `server-config` but **consumed nowhere**; its own doc claims it is "required for public/simple_auth deployments". Until it is wired (or removed), simple-auth deployments need an explicit Cedar policy file permitting the admin principal.

Ferro's middleware order is a security invariant worth restating (request order; axum layers apply bottom-up): cors → auth_guard → state_gate → guest_expiry → csrf → simple_auth → oidc_auth → cedar → rate_limit.

## 2026-10-29 estate re-scan: ~60 NEW repos; dogfood priorities for R8

All published on crates.io (UA required for API). Top dogfood candidates for Ferro:

1. **mp4-core 0.3.2 + id3-core 0.3.1**; audio metadata extraction (ilst/ID3v2: title/artist/album) → indexer search text. SAME pattern as the shipped sheet-xlsx integration (`extract_audio_metadata_text` pure fn + branch in `index_file_with_content`). mp4-core has a writer for test fixtures; id3-core has Tag::new/frame builders. **Highest value/effort; next session's first item.**
2. **tantivy-ext 1.0.0**; Tantivy wrapper (autocomplete, highlighting). Ferro's `core::search` uses Tantivy directly; adopt additively (highlighting for search API) rather than swapping the engine.
3. **crdts-kit 1.0.0**; RGA replicated string + participant presence: direct fit for `collab_ws` (Ferro's DeltaCRDT Lean proofs pair with it).
4. **metrics-kit 1.0.0**; lock-free Prometheus counters/gauges/histograms: candidate to replace Ferro's hand-rolled metrics registry (route latency/counters live in `startup.rs`).
5. **graceful 0.1.1**; signal handling/drain: Ferro's CancellationToken shutdown could adopt it.
6. **webauthn-kit 1.0.0**; replace/augment `webauthn_api`.
7. **eventbus 0.5.1**; typed async pub/sub with persistence: candidate under `WebdavFileEvent` dispatch (compare with the formally-verified outbox first; do NOT duplicate YP-EVENT).
8. **percentile-kit, envstack, flag-kit, clock-kit, app-error**; assess alongside their Ferro counterparts (metrics histograms, server-config, no flag system yet, time sources, error taxonomy).

Not applicable: dsp-*, font-*, can-*/xcp-core/dbc-parse/a2l-parse (automotive), accounting suite (ledger-kit/double-entry/vat-*/pay-kit/ap-kit/invoice-kit/report-kit/group-kit/fx-kit/recon-kit/statement-*/sales-tax; for the WyattAu accounting product), audiobook-* (uses mp4-core/id3-core), argus/aileron/clawdius/waves (platforms), ui-text-edit/input-kit/tui-actions (GUI/terminal), docs-core/theme-kit/a11y-kit (for ferro-web; secondary), mcp-doctor/skillkit/agentic-awesome-skills (agent tooling).

## 2026-10-29 dogfood round 2

- **DONE: mp4-core 0.3 + id3-core 0.3**; audio metadata (M4A/M4B ilst + MP3 ID3v2) indexed into search (server 255 tests green).
- **tantivy-ext 1.0.0: NOT consumable**; crates.io API lists it, but the sparse index lacks it and the CDN download returns AccessDenied (failed/partial publish). Upstream: re-run `cargo publish`. Adopt autocomplete/highlighting once resolvable.
- **Verified resolvable**: crdts-kit 1.0, metrics-kit 1.0, eventbus 0.5.1, graceful 0.1.1, percentile-kit 0.1.1, webauthn-kit 1.0.
- Next integration order: metrics-kit (route metrics → Prometheus counters/histograms), then graceful (CancellationToken shutdown swap), then crdts-kit (collab_ws RGA).

## Hook fix + remaining workspace clippy debt (2026-10-29)

`.githooks/pre-commit` now lints `ferro-mobile` without `--all-features`
(upstream tauri-plugin-camera 0.1.4 breaks desktop all-features builds).
Remaining `--all-features -D warnings` debt that trips the hook's
workspace fallback on doc-only commits: `migrate` (~20), `client` (~6),
`desktop` (~2); mostly let-chain collapses and needless borrows. One
session pass should clear it; code-change commits (per-crate path) are
unaffected.

## 2026-10-30: stash resolved + tantivy mystery solved

- **Stash archived** (tag `stash-archive-2026-10-29`): stale draft of the crate-split
  refactor, superseded by the committed Phase-3 extraction (23d1565d/995df2b7).
  613 insertions are Cargo.lock churn; nothing unique. Recoverable via the tag.
- **"tantivy-ext 1.0.0" is not our crate**; it's an unrelated/stale crates.io
  entry (AccessDenied download, absent from sparse index). The real wrapper is
  **`tantivy-helper` 0.3.1** (published, 232 downloads): `SearchEngine` +
  `Highlighter` + query/schema modules. **R8 dogfood target: swap or wrap
  Ferro's 818-line `core/src/search.rs` engine onto it**; the win is
  `Highlighter` (snippets in search API) and a maintained engine surface.
  Sized: moderate (search.rs has 4 public search fns + index_content used
  across server handlers).
