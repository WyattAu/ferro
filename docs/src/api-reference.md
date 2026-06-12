# API Reference

## REST API Overview

### Base URL

All REST API endpoints are available under `/api/v1/` (canonical). The unversioned `/api/` prefix is deprecated and will be removed in a future release.

```
http://localhost:8080/api/v1/
```

### Authentication

Authentication is required when `--admin-user` or `--oidc-issuer` is configured.

| Method | Header | Example |
|--------|--------|---------|
| HTTP Basic Auth | `Authorization: Basic <base64>` | `Authorization: Basic YWRtaW46cGFzc3dvcmQ=` |
| Bearer Token (OIDC) | `Authorization: Bearer <token>` | `Authorization: Bearer eyJhbGciOi...` |

### Content Types

| Endpoint Type | Content-Type |
|---------------|-------------|
| JSON endpoints | `application/json` |
| WebDAV XML | `application/xml` |
| File upload | Matches file MIME type |
| GraphQL | `application/json` |

### Response Format

All JSON endpoints return responses in the format:

```json
{
  "data": { ... },
  "error": null
}
```

Error responses:

```json
{
  "data": null,
  "error": {
    "code": 404,
    "message": "File not found"
  }
}
```

## File Operations

### WebDAV Methods

| Method | Path | Description | RFC |
|--------|------|-------------|-----|
| `OPTIONS` | `/*` | Discover supported methods and DAV capabilities | RFC 4918 |
| `PROPFIND` | `/*` | Retrieve properties for one or more resources | RFC 4918 |
| `GET` | `/*` | Retrieve file content | RFC 7231 |
| `PUT` | `/*` | Create or update a file | RFC 7231 |
| `DELETE` | `/*` | Remove a resource | RFC 7231 |
| `MKCOL` | `/*` | Create a collection (directory) | RFC 4918 |
| `COPY` | `/*` | Copy a resource to a new location | RFC 4918 |
| `MOVE` | `/*` | Move a resource to a new location | RFC 4918 |
| `LOCK` | `/*` | Lock a resource (exclusive, write) | RFC 4918 |
| `UNLOCK` | `/*` | Release a lock | RFC 4918 |
| `PROPPATCH` | `/*` | Modify resource properties | RFC 4918 |

### REST File Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `PUT` | `/*` | Upload a file (WebDAV) |
| `GET` | `/*` | Download a file (WebDAV) |
| `DELETE` | `/*` | Delete a file or directory (WebDAV) |
| `POST` | `/api/v1/files/move` | Move a file |
| `POST` | `/api/v1/files/copy` | Copy a file |
| `POST` | `/api/v1/files/mkdir` | Create a directory |
| `GET` | `/api/v1/files` | List files (JSON) |
| `POST` | `/api/v1/files/encrypt` | Encrypt a file (E2E) |
| `POST` | `/api/v1/files/decrypt` | Decrypt a file (E2E) |

#### List files (JSON)

```bash
curl "http://localhost:8080/api/v1/files?path=/documents&depth=1" \
  -H "Authorization: Bearer TOKEN"
```

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `path` | string | `/` | Directory to list |
| `depth` | integer | `1` | Nesting depth (`0` or `1`) |

#### Move a file

```bash
curl -X POST http://localhost:8080/api/v1/files/move \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"from": "/documents/hello.txt", "to": "/archive/hello.txt"}'
```

#### Copy a file

```bash
curl -X POST http://localhost:8080/api/v1/files/copy \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"from": "/documents/hello.txt", "to": "/backup/hello.txt"}'
```

### Batch Operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/batch/copy` | Copy multiple files |
| `POST` | `/api/v1/batch/move` | Move multiple files |
| `POST` | `/api/v1/bulk/delete` | Delete multiple files |

#### Batch copy

```bash
curl -X POST http://localhost:8080/api/v1/batch/copy \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"items": [
    {"from": "/a.txt", "to": "/backup/a.txt"},
    {"from": "/b.txt", "to": "/backup/b.txt"}
  ]}'
```

#### Bulk delete

```bash
curl -X POST http://localhost:8080/api/v1/bulk/delete \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"paths": ["/a.txt", "/b.txt", "/c.txt"]}'
```

### Chunked Upload

For large files, use the chunked upload API:

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/upload/init` | Initialize a chunked upload session |
| `PUT` | `/api/v1/upload/{session_id}/{chunk_number}` | Upload a chunk |
| `POST` | `/api/v1/upload/{session_id}/complete` | Complete the upload |
| `DELETE` | `/api/v1/upload/{session_id}` | Cancel an upload |

See [Chunked Upload](./api/chunked-upload.md) for detailed documentation.

## Admin API

Admin endpoints require admin role authentication. All routes are under `/api/v1/admin/`.

### Server Statistics

```bash
curl http://localhost:8080/api/v1/admin/stats \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "version": "3.0.0",
  "uptime_seconds": 3600,
  "total_files": 42,
  "total_directories": 7,
  "total_bytes": 1048576,
  "storage_backend": "memory",
  "auth_type": "basic",
  "wasm_workers_loaded": 0,
  "search_enabled": true,
  "features": {
    "s3": false,
    "gcs": false,
    "azure": false,
    "oidc": false,
    "cedar": false
  }
}
```

### User Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/admin/users` | List all users |
| `POST` | `/api/v1/admin/users` | Create a user |
| `GET` | `/api/v1/admin/users/{id}` | Get a user |
| `PUT` | `/api/v1/admin/users/{id}` | Update a user |
| `DELETE` | `/api/v1/admin/users/{id}` | Delete a user |
| `POST` | `/api/v1/admin/users/{id}/reset-password` | Reset password |

#### Create a user

```bash
curl -X POST http://localhost:8080/api/v1/admin/users \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "SecurePass123!", "role": "user"}'
```

### System Configuration

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/config` | Get server capabilities |
| `GET` | `/api/v1/admin/maintenance` | Check maintenance mode |
| `POST` | `/api/v1/admin/maintenance` | Toggle maintenance mode |
| `GET` | `/api/v1/admin/storage` | Detailed storage statistics |
| `GET` | `/api/v1/admin/integrity` | File hash integrity audit |
| `GET` | `/api/v1/admin/audit-chain` | Audit log chain verification |

### Backups

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/admin/backup` | Create a backup |
| `GET` | `/api/v1/admin/backups` | List all backups |
| `DELETE` | `/api/v1/admin/backup/{id}` | Delete a backup |
| `POST` | `/api/v1/admin/restore` | Restore from backup |

```bash
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "id": "backup-20260127-120000",
  "created_at": "2026-01-27T12:00:00+00:00",
  "files": 42,
  "bytes": 1048576
}
```

### Webhooks

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/admin/webhooks` | List webhooks |
| `POST` | `/api/v1/admin/webhooks` | Create a webhook |
| `DELETE` | `/api/v1/admin/webhooks/{id}` | Delete a webhook |

Webhook payload format:

```json
{
  "event": "file.upload",
  "timestamp": "2026-01-27T12:00:00+00:00",
  "path": "/docs/report.pdf",
  "size": 1024,
  "user": "admin",
  "etag": "abc123"
}
```

Delivered with headers: `X-Ferro-Signature: sha256=<hmac>`, `X-Ferro-Event: <event-type>`. Retried up to 3 times with exponential backoff.

## Search API

### Full-text Search

```bash
curl "http://localhost:8080/api/v1/search?q=report&limit=10" \
  -H "Authorization: Bearer TOKEN"
```

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `q` | string | (required) | Search query |
| `limit` | integer | `20` | Maximum results |

### Tag-based Search

```bash
curl "http://localhost:8080/api/v1/tags/search?tag=important" \
  -H "Authorization: Bearer TOKEN"
```

### Tag Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/tags` | List all tags |
| `GET` | `/api/v1/tags/{path}` | Get tags for a file |
| `POST` | `/api/v1/tags/{path}` | Add tags to a file |
| `DELETE` | `/api/v1/tags/{path}/{tag}` | Remove a tag |

```bash
curl -X POST http://localhost:8080/api/v1/tags/documents/report.pdf \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tags": ["important", "finance"]}'
```

### Pagination

Most list endpoints support pagination:

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `limit` | integer | `50` | Maximum entries per page |
| `offset` | integer | `0` | Pagination offset |

## WebSocket API

### Connection

```
ws://localhost:8080/api/ws
```

### Event Types

| Event | Description |
|-------|-------------|
| `file_created` | New file uploaded or created |
| `file_updated` | File content modified |
| `file_deleted` | File or directory deleted |
| `file_moved` | File moved or renamed |
| `file_shared` | Share link created |
| `sync_op` | Sync operation occurred |
| `storage_health` | Storage backend health changed |

#### Example: file_created

```json
{
  "type": "file_created",
  "path": "/documents/report.pdf",
  "size": 12345,
  "owner": "admin"
}
```

### Real-time Sync

The WebSocket connection is unidirectional from server to client -- the server pushes events, and clients receive them. For catching up on missed events, use the REST sync API:

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/sync/events` | SSE stream of file-change events |
| `GET` | `/api/v1/sync/delta?since=<clock>` | Delta changes since a clock value |
| `GET` | `/api/v1/sync/status` | Current sync clock status |

### Reconnection

If the connection is lost, reconnect with exponential backoff. Use `/api/v1/sync/delta?since=<last_clock>` to catch up on missed events.

## Error Responses

### HTTP Status Codes

| Code | Description |
|------|-------------|
| `200` | Success |
| `201` | Created |
| `204` | No Content (success, no body) |
| `207` | Multi-Status (WebDAV PROPFIND) |
| `400` | Bad Request (invalid input) |
| `401` | Unauthorized (missing/invalid auth) |
| `403` | Forbidden (insufficient permissions) |
| `404` | Not Found |
| `405` | Method Not Allowed |
| `409` | Conflict (e.g., user already exists) |
| `413` | Request Entity Too Large |
| `429` | Too Many Requests (rate limit) |
| `500` | Internal Server Error |
| `503` | Service Unavailable (maintenance mode) |

### Error Format

```json
{
  "error": "File not found",
  "status": 404
}
```

### Rate Limiting

When rate limit is exceeded (default: 10,000 req/min per IP):

```
HTTP/1.1 429 Too Many Requests
Retry-After: 60
```

## Additional Endpoints

### Health Probes

| Endpoint | Description |
|----------|-------------|
| `GET /healthz` | Liveness probe (returns `ok`) |
| `GET /readyz` | Readiness probe (checks all subsystems) |
| `GET /startupz` | Startup probe (200 once startup complete) |
| `GET /.well-known/ferro` | Full health check with subsystem status |
| `GET /metrics` | Server metrics (JSON) |
| `GET /metrics/prometheus` | Prometheus exposition format |

### Other Features

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/trash` | List trashed items |
| `POST` | `/api/v1/trash/restore` | Restore from trash |
| `DELETE` | `/api/v1/trash/empty` | Empty trash |
| `POST` | `/api/v1/snapshots` | Create a snapshot |
| `GET` | `/api/v1/snapshots` | List snapshots |
| `POST` | `/api/v1/snapshots/{id}/restore` | Restore a snapshot |
| `GET` | `/api/v1/favorites` | List favorites |
| `PUT` | `/api/v1/favorites` | Add a favorite |
| `DELETE` | `/api/v1/favorites` | Remove a favorite |
| `GET` | `/api/v1/recent` | Recent files |
| `GET` | `/api/v1/activity` | Activity feed |
| `GET` | `/api/v1/quota` | Storage quota usage |
| `GET` | `/api/v1/preferences` | User preferences |
| `PUT` | `/api/v1/preferences` | Update preferences |
| `GET` | `/api/v1/locks` | List active locks |
| `GET` | `/api/v1/storage/stats` | Storage statistics |
| `GET` | `/api/v1/thumbnail/*` | Generate thumbnails |
| `GET` | `/api/v1/health/storage` | Storage backend health |
| `GET` | `/api/v1/audit` | Audit log |
| `GET` | `/api/v1/auth/info` | Current auth info |
| `POST` | `/api/v1/auth/change-password` | Change password |

### GraphQL

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/graphql` | GraphQL Playground (browser) |
| `POST` | `/api/graphql` | Execute queries and mutations |

See [GraphQL API](./api/graphql.md) for the full schema.

### Federation

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/fed/share` | Share a file via ActivityPub |

See [Federation API](./api/federation.md) for details.
