# Ferro v1.0.0

## Platform

High-performance, self-hosted file storage platform in Rust.
WebDAV, OIDC, block-dedup sync, WASM plugins, full-text search.

## What's in v1.0.0

### Files
- Web UI (SolidJS + Astro): browse, upload, download, rename, trash,
  restore, share, search, preview (images/PDF/text/video/audio),
  drag-drop, multi-select, breadcrumbs, type filters, context menus
- Shared Spaces: browse and manage shared team folders with per-space
  Cedar-enforced access control
- Admin panel: stats, audit log, users, groups, space members
  (live policy reload), backups, GDPR, branding
- Settings: profile, password, TOTP, passkeys, preferences, quota
- Block-dedup sync: only changed 64 KB blocks uploaded
- LAN mesh: peers on the same network exchange blocks directly
- Offline token: `offline_access` scope keeps sync alive across reboots

### Server
- OIDC authentication (Keycloak) + group-based admin
- Per-user data isolation with Cedar authorization
- TOTP + passkey support
- WebDAV (RFC 4918), CalDAV, CardDAV with RFC 6764 discovery
- WOPI office editing (Collabora Online)
- VictoriaMetrics + Grafana monitoring
- Restic backups with staleness alerting
- Rate limiting, audit log, DLP, watermarking, ClamAV

### Performance
- App response: 1-2 ms (50× inside 100 ms target)
- Parallel chunk hashing (rayon)
- mimalloc allocator, zstd compression
- QUIC tunnel with 7 MB UDP buffers

## Known limitations
- Android APK build failing (non-blocking — deferred platform)
- macOS DMG requires Xcode for universal binary
- PDF/office preview requires Cloudflare CSP rule adjustment

## Install

See `docs/desktop/INSTALL_CHECKLIST.md` for per-platform instructions.
Artifacts: GitHub Actions → latest "Desktop (All Platforms)" run.
