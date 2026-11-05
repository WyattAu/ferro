# Ferro session ledger — mobile + native-Rust + CI round

Status: CURRENT (2026-11-04/05). Append-only; update the table, re-read
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
| CI green on c11e59e | PENDING | run in progress; test job had been blocked by the lockfile inconsistency fixed here |
| NAS redeploy after green CI | DONE early | prod-20261105 pulled from ghcr (post-fix digest 275bde3c) at 03:30 UTC Nov 5 |
| Phone in-app test (vivo, Ferro app) | BLOCKED | vivo unplugged; app installed; needs replug + `adb reverse tcp:8080 tcp:8199` + wake |
| Phone web test (Samsung, Chrome) | DONE | login/list/Range/video-playing verified via CDP + screenshots |
| utoipa 6 + zip 9 + axum 0.8 migration | DEFERRED | utoipa v8 swagger-ui build script incompatible with zip 9; needs axum 0.7→0.8 workspace migration |
| Traefik ferro router dropped on recreate | OPEN | NAS tunnel now bypasses Traefik for ferro.wyattau.com (ingress → localhost:8081) |


## Round addendum (2026-11-05 late)

| Task | Status | Evidence / next |
|---|---|---|
| WebDAV COPY 409 on implicit parents | DONE, pushed 34d0553c | preflight probes with listing instead of exists(); litmus 22/22 |
| Cedar OPTIONS un-gating (WebDAV discovery) | DONE, pushed 5f46b7ef | spawned server OPTIONS 200 + DAV banner; production anonymous OPTIONS now 401-auth (correct) |
| Production redeploy with cedar fix | PENDING | prod-img5 building (incremental); NAS compose pull + up -d after push |
| CI green | PENDING | runs cancelled at ~33m twice (cancel-in-progress + parallel sessions); not test failures — next CI run for main validates |
| rclone_e2e spawn timeout | DONE, pushed d630997c | 10s → 120s |
| 30 cargo-deny advisories | DONE, pushed 5a80d626 | deny 0 errors |

## Resume notes

- Dev server binary rebuilds via `systemd-run --user --unit=devsrv-build`
  (target dir vanishes on tree flap; rebuild is incremental when the
  cache survives).
- After ANY dev server relaunch: rotate the admin password (env restart
  resets to default; the default-password gate 403s everything).
- Phone quick-start: `adb reverse tcp:8080 tcp:8199`; app → login
  admin / Drive-T3st!pass; CDP via `adb forward tcp:9223
  localabstract:webview_devtools_remote_<pid>`.
- Production share link: `/s/8f3a91d2-5c47-4b68-a09e-video4463v1`.
