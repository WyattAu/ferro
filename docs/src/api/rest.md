# REST API

The REST API provides JSON endpoints for file operations, user management, sharing, and server administration.

All endpoints are available under `/api/v1/` (canonical). The unversioned `/api/` prefix is deprecated and will be removed in a future release. Authentication is required when `--admin-user` or `--oidcissuer` is configured.

The examples below use `/api/` for brevity; replace with `/api/v1/` for forward-compatible code.

## Health Endpoints

### Health check

```bash
curl http://localhost:8080/.well-known/ferro
```

```json
{
  "status": "ok",
  "version": "2.5.1",
  "uptime_seconds": 3600,
  "subsystems": {
    "storage": "ok",
    "metadata": "in-memory",
    "wasm": "disabled",
    "search": "disabled",
    "auth": "disabled",
    "cas": "disabled"
  }
}
```

### Liveness probe

```bash
curl http://localhost:8080/healthz
```

### Readiness probe

```bash
curl http://localhost:8080/readyz
```

```json
{
  "status": "ok",
  "subsystems": {
    "storage": "ok",
    "metadata": "persistent"
  }
}
```

## Server Configuration

### Get capabilities

```bash
curl http://localhost:8080/api/config
```

```json
{
  "version": "2.5.1",
  "auth_enabled": true,
  "search_enabled": true,
  "wasm_workers_enabled": false,
  "cedar_enabled": false,
  "metadata_persistent": true,
  "cas_enabled": true,
  "storage": "configured",
  "external_url": "http://localhost:8080",
  "wopi_configured": false
}
```

## File Operations

### Upload a file (WebDAV PUT)

```bash
curl -X PUT http://localhost:8080/documents/hello.txt \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: text/plain" \
  -d "Hello, Ferro!"
```

### Download a file

```bash
curl http://localhost:8080/documents/hello.txt \
  -H "Authorization: Bearer TOKEN" -o hello.txt
```

### Move a file

```bash
curl -X POST http://localhost:8080/api/files/move \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"from": "/documents/hello.txt", "to": "/archive/hello.txt"}'
```

### Copy a file

```bash
curl -X POST http://localhost:8080/api/files/copy \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"from": "/documents/hello.txt", "to": "/backup/hello.txt"}'
```

### Encrypt a file

```bash
curl -X POST http://localhost:8080/api/files/encrypt \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/secret.txt", "passphrase": "my-password"}'
```

### Decrypt a file

```bash
curl -X POST http://localhost:8080/api/files/decrypt \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/secret.txt", "passphrase": "my-password"}'
```

## User Management

### Create a user

```bash
curl -X POST http://localhost:8080/api/admin/users \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username": "newuser", "password": "SecurePass123!", "role": "user"}'
```

### List users

```bash
curl http://localhost:8080/api/admin/users \
  -H "Authorization: Bearer TOKEN"
```

### Get current user

```bash
curl http://localhost:8080/api/users/me \
  -H "Authorization: Bearer TOKEN"
```

### Reset a user's password

```bash
curl -X POST http://localhost:8080/api/admin/users/1/reset-password \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"password": "NewPass456!"}'
```

## Sharing

### Create a share link

```bash
curl -X POST http://localhost:8080/api/shares \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/report.pdf", "password": "secret123", "expires_hours": 48}'
```

### List shares

```bash
curl http://localhost:8080/api/shares \
  -H "Authorization: Bearer TOKEN"
```

### Delete a share

```bash
curl -X DELETE http://localhost:8080/api/shares/TOKEN \
  -H "Authorization: Bearer TOKEN"
```

### Access a shared file

```bash
curl http://localhost:8080/s/TOKEN -o report.pdf
```

## Tags

### Add tags to a file

```bash
curl -X POST http://localhost:8080/api/tags/documents/report.pdf \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tags": ["important", "finance"]}'
```

### Get tags for a file

```bash
curl http://localhost:8080/api/tags/documents/report.pdf \
  -H "Authorization: Bearer TOKEN"
```

### Search by tag

```bash
curl "http://localhost:8080/api/tags/search?tag=important" \
  -H "Authorization: Bearer TOKEN"
```

### Remove a tag

```bash
curl -X DELETE http://localhost:8080/api/tags/documents/report.pdf/important \
  -H "Authorization: Bearer TOKEN"
```

## Batch Operations

### Batch copy

```bash
curl -X POST http://localhost:8080/api/batch/copy \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"items": [
    {"from": "/a.txt", "to": "/backup/a.txt"},
    {"from": "/b.txt", "to": "/backup/b.txt"}
  ]}'
```

### Batch move

```bash
curl -X POST http://localhost:8080/api/batch/move \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"items": [
    {"from": "/a.txt", "to": "/archive/a.txt"},
    {"from": "/b.txt", "to": "/archive/b.txt"}
  ]}'
```

### Bulk delete

```bash
curl -X POST http://localhost:8080/api/bulk/delete \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"paths": ["/a.txt", "/b.txt", "/c.txt"]}'
```

## Search

### Full-text search

```bash
curl "http://localhost:8080/api/search?q=report&limit=10" \
  -H "Authorization: Bearer TOKEN"
```

## Trash

### List trashed items

```bash
curl http://localhost:8080/api/trash \
  -H "Authorization: Bearer TOKEN"
```

### Restore from trash

```bash
curl -X POST http://localhost:8080/api/trash/restore \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/old-file.txt"}'
```

### Empty trash

```bash
curl -X DELETE http://localhost:8080/api/trash/empty \
  -H "Authorization: Bearer TOKEN"
```

## Snapshots

### Create a snapshot

```bash
curl -X POST http://localhost:8080/api/snapshots \
  -H "Authorization: Bearer TOKEN"
```

### List snapshots

```bash
curl http://localhost:8080/api/snapshots \
  -H "Authorization: Bearer TOKEN"
```

### Restore a snapshot

```bash
curl -X POST http://localhost:8080/api/snapshots/1/restore \
  -H "Authorization: Bearer TOKEN"
```

## Storage Stats

```bash
curl http://localhost:8080/api/storage/stats \
  -H "Authorization: Bearer TOKEN"
```

## File Listing

### List files (JSON)

```bash
curl "http://localhost:8080/api/files?path=/documents&depth=1" \
  -H "Authorization: Bearer TOKEN"
```

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `path` | string | `/` | Directory to list |
| `depth` | integer | `1` | Nesting depth (`0` or `1`) |

```json
{
  "entries": [
    {
      "name": "report.pdf",
      "path": "/documents/report.pdf",
      "size": 1024,
      "is_collection": false,
      "mime_type": "application/pdf",
      "etag": "abc123",
      "content_hash": "sha256:...",
      "modified_at": "2026-01-01T00:00:00+00:00",
      "created_at": "2026-01-01T00:00:00+00:00"
    }
  ]
}
```

### Create a directory

```bash
curl -X POST http://localhost:8080/api/files/mkdir \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/new-folder"}'
```

```json
{
  "path": "/documents/new-folder",
  "created_at": "2026-01-01T00:00:00+00:00"
}
```

## Favorites

### List favorites

```bash
curl http://localhost:8080/api/favorites \
  -H "Authorization: Bearer TOKEN"
```

### Add a favorite

```bash
curl -X PUT http://localhost:8080/api/favorites \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/report.pdf"}'
```

### Remove a favorite

```bash
curl -X DELETE http://localhost:8080/api/favorites \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/report.pdf"}'
```

## Recent Files

### List recent files

```bash
curl http://localhost:8080/api/recent \
  -H "Authorization: Bearer TOKEN"
```

Returns recently modified files from the audit log.

## Activity Feed

### Get activity

```bash
curl "http://localhost:8080/api/activity?limit=50&offset=0" \
  -H "Authorization: Bearer TOKEN"
```

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `limit` | integer | `50` | Maximum entries per page |
| `offset` | integer | `0` | Pagination offset |

```json
{
  "entries": [
    {
      "action": "upload",
      "path": "/docs/report.pdf",
      "size": 1024,
      "timestamp": "2026-01-01T00:00:00+00:00",
      "user": "admin"
    }
  ],
  "total": 42
}
```

## Tags

### List all tags

```bash
curl http://localhost:8080/api/tags \
  -H "Authorization: Bearer TOKEN"
```

### Add tags to a file

```bash
curl -X POST http://localhost:8080/api/tags/documents/report.pdf \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tags": ["important", "finance"]}'
```

### Get tags for a file

```bash
curl http://localhost:8080/api/tags/documents/report.pdf \
  -H "Authorization: Bearer TOKEN"
```

### Remove a tag

```bash
curl -X DELETE http://localhost:8080/api/tags/documents/report.pdf/important \
  -H "Authorization: Bearer TOKEN"
```

### Search by tag

```bash
curl "http://localhost:8080/api/tags/search?tag=important" \
  -H "Authorization: Bearer TOKEN"
```

## Trash

### List trashed items

```bash
curl http://localhost:8080/api/trash \
  -H "Authorization: Bearer TOKEN"
```

### Move a file to trash

```bash
curl -X DELETE http://localhost:8080/api/trash/documents/old-file.txt \
  -H "Authorization: Bearer TOKEN"
```

Request body:

```json
{ "original_path": "/documents/old-file.txt" }
```

### Restore from trash

```bash
curl -X POST http://localhost:8080/api/trash/restore \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/old-file.txt"}'
```

### Purge a specific trashed item

```bash
curl -X DELETE http://localhost:8080/api/trash/purge \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/old-file.txt"}'
```

### Empty trash

```bash
curl -X DELETE http://localhost:8080/api/trash/empty \
  -H "Authorization: Bearer TOKEN"
```

## Batch Operations

### Bulk delete

```bash
curl -X POST http://localhost:8080/api/bulk/delete \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"paths": ["/a.txt", "/b.txt", "/c.txt"]}'
```

### Batch copy

```bash
curl -X POST http://localhost:8080/api/batch/copy \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"items": [
    {"from": "/a.txt", "to": "/backup/a.txt"},
    {"from": "/b.txt", "to": "/backup/b.txt"}
  ]}'
```

### Batch move

```bash
curl -X POST http://localhost:8080/api/batch/move \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"items": [
    {"from": "/a.txt", "to": "/archive/a.txt"},
    {"from": "/b.txt", "to": "/archive/b.txt"}
  ]}'
```

## Authentication

### Get auth info

```bash
curl http://localhost:8080/api/auth/info \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "sub": "admin",
  "iss": "ferro",
  "aud": "ferro",
  "email": null,
  "name": null,
  "groups": [],
  "auth_type": "basic"
}
```

### OIDC login

Initiate OIDC authentication with PKCE. Requires OIDC to be configured.

```bash
curl "http://localhost:8080/api/auth/login?redirect=/ui/" \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "authorization_url": "https://idp.example.com/authorize?...",
  "state": "uuid-value"
}
```

### OIDC callback

Handle the OIDC redirect callback. Exchanges the authorization code for tokens.

### Refresh token

Exchange a refresh token for a new access token. Requires OIDC to be configured.

```bash
curl -X POST http://localhost:8080/api/auth/refresh \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"refresh_token": "..."}'
```

```json
{
  "access_token": "...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "..."
}
```

### Change password

Change the authenticated user's password. Rejects passwords shorter than 8 characters or matching known defaults.

```bash
curl -X POST http://localhost:8080/api/auth/change-password \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"password": "NewSecurePass123!"}'
```

```json
{
  "status": "ok",
  "message": "Password changed successfully. Default password restrictions lifted."
}
```

## Current User

### Get current user

```bash
curl http://localhost:8080/api/users/me \
  -H "Authorization: Bearer TOKEN"
```

### Update current user

```bash
curl -X PUT http://localhost:8080/api/users/me \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"display_name": "Admin User", "password": "NewPass456!"}'
```

Both `display_name` and `password` are optional. Providing a password changes it.

## Sync

### Sync events (SSE)

Stream file-change events as Server-Sent Events. The connection stays open and pushes events as they occur.

```bash
curl http://localhost:8080/api/sync/events \
  -H "Authorization: Bearer TOKEN"
```

Each event is typed `file-change` and contains a JSON-encoded sync operation.

### Sync delta

Fetch operations that occurred since a given clock value.

```bash
curl "http://localhost:8080/api/sync/delta?since=42" \
  -H "Authorization: Bearer TOKEN"
```

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `since` | integer | `0` | Clock value to fetch operations after |

```json
{
  "current_clock": 100,
  "ops": [...],
  "count": 58
}
```

### Sync status

```bash
curl http://localhost:8080/api/sync/status \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "current_clock": 100,
  "total_ops": 100,
  "max_ops": 100000
}
```

## Quota

### Get quota usage

```bash
curl http://localhost:8080/api/quota \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "used_bytes": 5242880,
  "quota_bytes": 10737418240,
  "used_percent": 0.05,
  "file_count": 42,
  "unlimited": false
}
```

When no quota is configured, `unlimited` is `true` and `used_percent` is `0.0`.

## Preferences

### Get preferences

```bash
curl http://localhost:8080/api/preferences \
  -H "Authorization: Bearer TOKEN"
```

### Update preferences

```bash
curl -X PUT http://localhost:8080/api/preferences \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"theme": "dark", "language": "en"}'
```

## Locks

### List locks

```bash
curl http://localhost:8080/api/locks \
  -H "Authorization: Bearer TOKEN"
```

### Force-unlock

Force-release a file lock.

```bash
curl -X POST http://localhost:8080/api/locks/force-unlock \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/report.pdf"}'
```

### Delete lock by token

```bash
curl -X DELETE http://localhost:8080/api/locks/lock-token-value \
  -H "Authorization: Bearer TOKEN"
```

## Storage Health

### GET /api/health/storage

Check storage backend health including latency and check history.

```bash
curl http://localhost:8080/api/health/storage \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "backend_type": "memory",
  "healthy": true,
  "last_check": "2026-01-27T12:00:00+00:00",
  "latency_ms": 1,
  "error": null,
  "total_checks": 1000,
  "failed_checks": 0
}
```

## Thumbnails

### GET /api/thumbnail/*path

Generate or return a cached thumbnail for an image file. Supports JPEG, PNG, GIF, WebP, and PDF.

```bash
curl http://localhost:8080/api/thumbnail/documents/photo.jpg \
  -H "Authorization: Bearer TOKEN" -o thumb.jpg
```

Returns the thumbnail as JPEG. For unsupported file types, returns an SVG file icon.

## GraphQL

### GET /api/graphql

Access the GraphQL Playground (interactive schema explorer).

### POST /api/graphql

Execute GraphQL queries and mutations.

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ files(prefix: \"/\") { path size } }"}'
```

## Federation

### POST /api/fed/share

Share a file via ActivityPub federation.

```bash
curl -X POST http://localhost:8080/api/fed/share \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/documents/report.pdf", "actor": "alice@example.com"}'
```

## Search

### Full-text search

```bash
curl "http://localhost:8080/api/search?q=report&limit=10" \
  -H "Authorization: Bearer TOKEN"
```

## Snapshots

### Create a snapshot

```bash
curl -X POST http://localhost:8080/api/snapshots \
  -H "Authorization: Bearer TOKEN"
```

### List snapshots

```bash
curl http://localhost:8080/api/snapshots \
  -H "Authorization: Bearer TOKEN"
```

### Restore a snapshot

```bash
curl -X POST http://localhost:8080/api/snapshots/1/restore \
  -H "Authorization: Bearer TOKEN"
```

### Delete a snapshot

```bash
curl -X DELETE http://localhost:8080/api/snapshots/1 \
  -H "Authorization: Bearer TOKEN"
```

## Storage Stats

```bash
curl http://localhost:8080/api/storage/stats \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "files": 42,
  "collections": 7,
  "total_bytes": 1048576,
  "cas": { "enabled": true, "content_blocks": 100 },
  "metadata_store": true
}
```

## Audit Log

```bash
curl "http://localhost:8080/api/audit?limit=50&offset=0" \
  -H "Authorization: Bearer TOKEN"
```

## Infrastructure Endpoints

### Liveness probe

```bash
curl http://localhost:8080/healthz
```

Returns plain text `ok` with 200.

### Readiness probe

```bash
curl http://localhost:8080/readyz
```

Checks storage, metadata, database, and search subsystems. Returns 200 or 503.

### Startup probe

Returns 200 once the server has completed all startup checks (storage, CAS, DB). Returns 503 until startup is complete.

```bash
curl http://localhost:8080/startupz
```

```json
{ "status": "ok" }
```

### Metrics (JSON)

```bash
curl http://localhost:8080/metrics
```

```json
{
  "uptime_seconds": 3600,
  "storage": { "files": 42, "total_bytes": 1048576 },
  "requests": { "total": 1000 }
}
```

### Metrics (Prometheus)

Returns server metrics in Prometheus exposition format. Includes gauges for uptime, file count, and storage usage, plus counters for HTTP requests, response status codes, storage operations, cache hits/misses, and WASM worker dispatches.

```bash
curl http://localhost:8080/metrics/prometheus
```

Content-Type: `text/plain; version=0.0.4; charset=utf-8`.
