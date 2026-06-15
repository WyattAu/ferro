# Ferro v3.1.0 Release Notes

**Release Date:** 2026-06-15
**Tag:** v3.1.0
**Binary Size:** ~9MB (release, LTO)

---

## What's New

### Desktop Client
- Tauri v2 desktop app with native Wayland support on KDE Plasma
- CLI auto-connect: `ferro-desktop --server-url http://host:8080 --auth-token <token>`
- System tray with sync status, pause/resume, settings
- File lock indicator, drag-and-drop, keyboard shortcuts
- Auth detection: automatically uses Basic or Bearer based on token format

### Server Decomposition
- Server split into 5 sub-crates for faster compilation and clearer boundaries:
  - `ferro-server-webdav` (14 tests) - WebDAV Class 1/2/3 + sync-collection
  - `ferro-server-security` (58 tests) - Auth, rate limiting, account lockout
  - `ferro-server-sharing` (50 tests) - Share links, permissions, guest access
  - `ferro-server-admin` (8 tests) - Admin dashboard API, user management
  - `ferro-server-automation` (13 tests) - WASM workers, event triggers, retention

### CalDAV Server
- RFC 4791 calendar-multiget REPORT
- Calendar and event CRUD via WebDAV
- iCal parsing with VEVENT/VTODO support

### SCIM Provisioning
- SCIM 2.0 user and group management endpoints
- Service provider config, schema discovery

### Migration Tools
- oCIS migration via WebDAV streaming
- Nextcloud migration with SQLite DB reader for users/shares/tags

### Production Infrastructure
- Docker Compose production stack (Ferro + PostgreSQL + Redis + Caddy + monitoring)
- Grafana dashboards with auto-provisioned Prometheus + Alertmanager
- Production deployment guide (663 lines)
- 24h soak test harness

### Search Improvements
- Configurable relevance tuning (name 3x, path 2x, recency 1.2x)
- Admin API for boost factor adjustment and index reindexing

### Collaboration
- CRDT document persistence across sessions
- WebSocket reconnection with exponential backoff
- Room-based document state management

---

## Quality Metrics

| Metric | Value |
|--------|-------|
| Crates | 43 |
| Tests | 2500+ passed |
| Clippy warnings | 0 |
| Fuzzing | 4 harnesses, 2.6M+ iterations, 0 crashes |
| WebDAV compliance | Class 1/2/3 + sync-collection (22 tests) |
| API endpoints | 90+ (REST, GraphQL, WebDAV, CalDAV, WebSocket) |

---

## Platform Support

| Platform | Status |
|----------|--------|
| Linux (X11) | Verified |
| Linux (Wayland/KDE) | Verified |
| Docker (amd64, arm64) | Multi-arch builds |
| Kubernetes | Helm chart available |
| macOS | CLI + WASM frontend |
| Windows | CLI + WASM frontend |
| iOS | Tauri v2 scaffold (requires Xcode) |
| Android | Tauri v2 scaffold (requires Android SDK) |

---

## Upgrade from v3.0.0

No breaking changes. Drop-in replacement.

```bash
# Docker
docker pull ghcr.io/wyattau/ferro:3.1.0

# Binary
curl -L https://github.com/WyattAu/ferro/releases/download/v3.1.0/ferro-server-linux-amd64 -o ferro-server
chmod +x ferro-server
```

---

## Known Issues

1. **Direct JS fetch calls lack auth headers:** The desktop frontend's lock polling (`/api/locks`) and service worker requests bypass Tauri commands and don't carry auth headers. This causes 403 responses but does not affect functionality.
2. **Rate limiter on failed auth:** Multiple failed auth attempts trigger account lockout. Server restart clears lockout.
3. **ClamAV integration:** Skeleton only. No real ClamAV socket connection yet.
4. **Raft consensus:** Module exists but not wired to server startup. Single-node mode recommended.
