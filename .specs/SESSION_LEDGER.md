# Ferro session ledger — mobile + native-Rust + CI round

Status: CURRENT (2026-11-04/06). Append-only; update the table, re-read
first after any compaction or restart.

## Status table

| Task | Status | Evidence / next |
|---|---|---|
| Video share + Range on production | DONE | https://ferro.wyattau.com/s/8f3a91d2-5c47-4b68-a09e-video4463v1 → 206 video/mp4 |
| Audit split-store fix (in_memory two AuditLogs) | DONE, pushed f0effd42 | integration.rs test_audit_log green in CI run 41954495409 |
| New-user empty-drive listing (200 not 404) | DONE, pushed 3eece095 | integration_rest 34/34 green |
| Cedar OPTIONS policy-gate fix (WebDAV discovery) | DONE, pushed 5f46b7ef | spawned server OPTIONS 200 + DAV banner; rclone_e2e green |
| Production upgraded to prod-20261105 | DONE | NAS compose prod-20261101 → prod-20261105, PG stores init, share 206 verified |
| 30 cargo-deny advisories cleared | DONE, pushed 5a80d626 | deny 0 errors (10 documented ignores in deny.toml) |
| CI config-leak root cause (absolute target-dir) | DONE, pushed 3a1766cd | .cargo/config.toml [build] override removed; was killing every job with exit 101 |
| rclone_e2e spawn flake (10s → 120s) | DONE, pushed d630997c | only remaining CI red was this flake + lockfile inconsistency |
| Android app in-flow verification (vivo) | DONE | fresh APK (SPA fallback + no-store proxy + login fixes): login → listing → in-app video playback + seek via CDP |
| Android AssetServer fixes (SPA fallback, no-store, accept-encoding strip) | DONE, pushed af81d9d0 | deep links 404 → shell; binary-garbage JSON; stale cached configs |
| Configurable backend URL (server_url pref) | DONE, uncommitted→push with this round | AssetServer reads SharedPreferences; default adb-reverse 8080 |
| CI green on d630997c+ | PENDING | earlier runs cancelled by concurrency (newer pushes); latest run had 60m cap; re-run after current round settles |

## Environment quick-reference (dev server on box:8199)

- Relaunch: env FERRO_STORAGE=local:/tmp/opencode/ferro-video FERRO_ADMIN_USER=admin FERRO_ADMIN_PASSWORD=admin FERRO_STATIC_DIR=crates/web/dist ./target/debug/ferro-server --host 0.0.0.0 --port 8199 --data-dir /tmp/opencode/ferro-data --cedar-policy-file /tmp/opencode/ferro-litmus-policy.cedar --cors-origins "http://localhost:8199,http://127.0.0.1:8199,http://127.0.0.1:8888,http://192.168.0.105:8199,http://192.168.1.1:8199,http://100.93.183.32:8199"
- After relaunch: rotate admin password (env reset arms the default-password gate).
- CORS origins must cover the phone's WebView origin (127.0.0.1:8888).
- adb reverse rules are per-device and die with the USB session: re-add after replug.
- /tmp/opencode is wiped on env restarts (Cedar policy file must be recreated).

## Open items

- CI: one clean run of main to publish the docker image (60m timeouts in place).
- Production OIDC: phone app on prod domain needs Keycloak login (basic login is dev-only).
- CI feature-flag/PG job failures: investigate separately (pre-date this round; the rclone 120s + audit fixes address the main flake class).
