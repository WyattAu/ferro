# Admin API

Admin endpoints require authentication. All routes are under `/api/admin/` (or `/api/v1/admin/`).

## Server Statistics

### GET /api/admin/stats

Return server statistics including version, uptime, file counts, and enabled features.

```bash
curl http://localhost:8080/api/admin/stats \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "version": "2.5.1",
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

## Storage Details

### GET /api/admin/storage

Return detailed storage statistics including the largest file and recent files.

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `limit` | integer | 10000 | Maximum entries to scan |

```bash
curl "http://localhost:8080/api/admin/storage?limit=1000" \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "backend": "memory",
  "total_bytes": 1048576,
  "file_count": 42,
  "directory_count": 7,
  "largest_file": { "path": "/videos/demo.mp4", "size": 524288 },
  "recent_files": [
    { "path": "/docs/report.pdf", "size": 1024, "modified_at": "2026-01-01T00:00:00+00:00" }
  ]
}
```

## Audit Log

### GET /api/admin/audit

Return paginated audit log entries from the server-side audit log.

| Query param | Type | Default | Description |
|-------------|------|---------|-------------|
| `limit` | integer | 100 | Maximum entries per page |
| `offset` | integer | 0 | Pagination offset |

```bash
curl "http://localhost:8080/api/admin/audit?limit=50&offset=0" \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "entries": [
    {
      "method": "PUT",
      "path": "/docs/report.pdf",
      "user": "admin",
      "status": 201,
      "timestamp": "2026-01-01T00:00:00+00:00"
    }
  ],
  "total": 150,
  "limit": 50,
  "offset": 0
}
```

## Maintenance Mode

### GET /api/admin/maintenance

Check whether maintenance mode is currently active.

```bash
curl http://localhost:8080/api/admin/maintenance \
  -H "Authorization: Bearer TOKEN"
```

```json
{ "maintenance_mode": false }
```

### POST /api/admin/maintenance

Enable or disable maintenance mode. When enabled, all write operations return 503 except the maintenance toggle itself.

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `enabled` | boolean | yes | `true` to enable, `false` to disable |

```bash
curl -X POST http://localhost:8080/api/admin/maintenance \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

```json
{ "maintenance_mode": true }
```

## Backups

### POST /api/admin/backup

Create a new backup of all files and the SQLite database. Requires `--data-dir` to be configured.

```bash
curl -X POST http://localhost:8080/api/admin/backup \
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

### GET /api/admin/backups

List all available backups, sorted by creation time descending.

```bash
curl http://localhost:8080/api/admin/backups \
  -H "Authorization: Bearer TOKEN"
```

```json
[
  {
    "id": "backup-20260127-120000",
    "created_at": "2026-01-27T12:00:00+00:00",
    "files": 42,
    "bytes": 1048576
  }
]
```

### DELETE /api/admin/backup/:id

Delete a backup by its ID.

```bash
curl -X DELETE http://localhost:8080/api/admin/backup/backup-20260127-120000 \
  -H "Authorization: Bearer TOKEN"
```

Returns 204 No Content on success.

### POST /api/admin/restore

Restore files from a previous backup. Existing files are skipped (idempotent).

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `backup_id` | string | yes | ID of the backup to restore |

```bash
curl -X POST http://localhost:8080/api/admin/restore \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"backup_id": "backup-20260127-120000"}'
```

```json
{
  "restored_files": 42,
  "total_files": 42,
  "backup_id": "backup-20260127-120000"
}
```

## Integrity

### GET /api/admin/integrity

Audit all stored files for hash integrity. Reads each file, recomputes its SHA-256 digest, and compares against the stored `content_hash`.

```bash
curl http://localhost:8080/api/admin/integrity \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "scanned_at": "2026-01-27T12:00:00+00:00",
  "total_files": 42,
  "ok": 41,
  "mismatches": 0,
  "unreadable": 0,
  "invalid_hashes": 1,
  "findings": [
    {
      "path": "/docs/corrupt.bin",
      "status": "invalid_hash",
      "stored_hash": "abc123",
      "computed_hash": ""
    }
  ]
}
```

Integrity status values: `ok`, `mismatch`, `unreadable`, `invalid_hash`.

### GET /api/admin/audit-chain

Verify the audit log chain hash integrity. Requires SQLite persistence to be configured.

```bash
curl http://localhost:8080/api/admin/audit-chain \
  -H "Authorization: Bearer TOKEN"
```

Returns a verification report. Returns 503 if persistence is not configured.

## Webhooks

### GET /api/admin/webhooks

List all webhook subscriptions.

```bash
curl http://localhost:8080/api/admin/webhooks \
  -H "Authorization: Bearer TOKEN"
```

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "url": "https://example.com/hook",
    "secret": "whsec_...",
    "events": ["file.upload", "file.delete"],
    "enabled": true
  }
]
```

### POST /api/admin/webhooks

Create a new webhook subscription. Maximum 100 webhooks per server.

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | yes | URL to receive webhook payloads |
| `events` | string[] | yes | Event types to subscribe to |
| `secret` | string | no | HMAC-SHA256 secret for payload verification (auto-generated if omitted) |
| `enabled` | boolean | no | Whether the webhook is active (default: `true`) |

```bash
curl -X POST http://localhost:8080/api/admin/webhooks \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/hook", "events": ["file.upload", "file.delete"]}'
```

Returns 201 Created with the full webhook config including the generated `id` and `secret`.

### DELETE /api/admin/webhooks/:id

Delete a webhook subscription by ID.

```bash
curl -X DELETE http://localhost:8080/api/admin/webhooks/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer TOKEN"
```

Returns 204 No Content on success, 404 if not found.

## User Management

All user management endpoints require admin role.

### GET /api/admin/users

List all registered users.

```bash
curl http://localhost:8080/api/admin/users \
  -H "Authorization: Bearer TOKEN"
```

```json
{
  "users": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "alice",
      "display_name": "Alice",
      "email": "alice@example.com",
      "role": "user",
      "created_at": "2026-01-01T00:00:00+00:00",
      "last_login": null,
      "status": "active",
      "storage_quota_bytes": null,
      "storage_used_bytes": 0,
      "is_ldap": false
    }
  ]
}
```

### POST /api/admin/users

Create a new user. Returns 409 if the username already exists.

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `username` | string | yes | Unique username |
| `password` | string | yes | User password |
| `display_name` | string | no | Display name (defaults to username) |
| `email` | string | no | Email address |
| `role` | string | no | User role (default: `"user"`) |
| `storage_quota_bytes` | integer | no | Per-user storage quota |

```bash
curl -X POST http://localhost:8080/api/admin/users \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "SecurePass123!", "role": "user"}'
```

Returns 201 Created with the user object (password hash omitted).

### GET /api/admin/users/{id}

Get a single user by ID.

```bash
curl http://localhost:8080/api/admin/users/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer TOKEN"
```

Returns 200 with the user object, or 404 if not found.

### PUT /api/admin/users/{id}

Update a user. Supports partial updates -- only provided fields are changed.

```bash
curl -X PUT http://localhost:8080/api/admin/users/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"display_name": "Alice Smith", "role": "admin"}'
```

Returns 200 with the updated user object, or 404/409 on error.

### DELETE /api/admin/users/{id}

Delete a user by ID.

```bash
curl -X DELETE http://localhost:8080/api/admin/users/550e8400-e29b-41d4-a716-446655440000 \
  -H "Authorization: Bearer TOKEN"
```

```json
{ "ok": true }
```

### POST /api/admin/users/{id}/reset-password

Reset a user's password.

Request body:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `new_password` | string | yes | New password (minimum 1 character) |

```bash
curl -X POST http://localhost:8080/api/admin/users/550e8400-e29b-41d4-a716-446655440000/reset-password \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"new_password": "NewPass456!"}'
```

```json
{ "ok": true }
```

## Webhook Delivery

Webhook payloads are sent as POST requests to the configured URL with these headers:

| Header | Description |
|--------|-------------|
| `Content-Type` | `application/json` |
| `X-Ferro-Signature` | `sha256=<hex>` HMAC of the payload body |
| `X-Ferro-Event` | The event type that triggered the webhook |

Payload format:

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

Failed deliveries are retried up to 3 times with exponential backoff (1s, 2s, 4s).
