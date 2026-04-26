# API Reference

All API endpoints return JSON unless otherwise noted. The server listens on port 8080 by default.

## Health Check

### GET `/.well-known/ferro`

Returns a plain text health check response.

**Response** `200 OK`

```
Ferro OK
```

## Server Configuration

### GET `/api/config`

Returns server configuration and enabled capabilities.

**Response** `200 OK`

```json
{
  "version": "0.1.0",
  "auth_enabled": false,
  "search_enabled": true,
  "wasm_workers_enabled": false,
  "cedar_enabled": false,
  "metadata_persistent": false,
  "cas_enabled": false,
  "storage": "configured"
}
```

## Authentication

### GET `/api/auth/info`

Returns current user info from OIDC claims, or anonymous info when auth is disabled.

**Response** `200 OK` (authenticated)

```json
{
  "sub": "user-123",
  "iss": "https://keycloak.example.com/realms/ferro",
  "aud": "ferro",
  "email": "user@example.com",
  "name": "Jane Doe",
  "groups": ["users", "admin"]
}
```

**Response** `200 OK` (anonymous)

```json
{
  "sub": "anonymous",
  "iss": "ferro",
  "aud": "ferro"
}
```

### GET `/api/auth/login`

Initiates OIDC login with PKCE. Returns the authorization URL for the frontend to redirect to.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `redirect` | string | `/ui/` | URL to redirect to after successful login |

**Response** `200 OK`

```json
{
  "authorization_url": "https://keycloak.example.com/realms/ferro/protocol/openid-connect/auth?response_type=code&client_id=ferro&redirect_uri=...&scope=openid+profile+email&state=...&code_challenge=...&code_challenge_method=S256",
  "state": "uuid-state-parameter"
}
```

**Response** `503 Service Unavailable` (OIDC not configured)

```json
{
  "error": "OIDC not configured",
  "message": "Set FERRO_OIDC_ISSUER environment variable to enable OIDC login."
}
```

### GET `/api/auth/callback`

Handles the OIDC callback. Exchanges the authorization code for tokens and returns user info.

**Query Parameters**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `code` | string | yes | Authorization code from OIDC provider |
| `state` | string | yes | State parameter for CSRF verification |

**Response** `200 OK`

```json
{
  "access_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "sub": "user-123",
    "email": "user@example.com",
    "name": "Jane Doe"
  },
  "redirect": "/ui/"
}
```

**Response** `400 Bad Request` (invalid state)

```json
{
  "error": "Invalid or expired state parameter"
}
```

## Search

### GET `/api/search`

Full-text search powered by Tantivy.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `q` | string | — | Search query (required) |
| `limit` | integer | 20 | Max results (1–100) |

**Response** `200 OK`

```json
{
  "query": "invoice",
  "results": [
    {
      "path": "/documents/invoice-2024.pdf",
      "score": 3.45,
      "snippet": "...invoice for the quarter ending..."
    }
  ],
  "total": 1,
  "limit": 20
}
```

**Response** `200 OK` (search not configured)

```json
{
  "query": "invoice",
  "results": [],
  "total": 0,
  "limit": 20,
  "configured": false
}
```

## WASM Workers

### GET `/api/workers`

Lists registered WASM workers.

**Response** `200 OK` (WASM enabled)

```json
{
  "workers": [
    {
      "pattern": "*.pdf",
      "module_path": "/data/workers/uuid-ocr.wasm",
      "function_name": "process",
      "max_fuel": 1000000000,
      "max_memory_bytes": 67108864
    }
  ]
}
```

**Response** `200 OK` (WASM not enabled)

```json
{
  "workers": [],
  "configured": false
}
```

### POST `/api/workers`

Register a new WASM worker.

**Request Body**

```json
{
  "pattern": "*.pdf",
  "module_path": "/data/workers/uuid-ocr.wasm",
  "function_name": "process",
  "max_fuel": 1000000000,
  "max_memory_bytes": 67108864
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `pattern` | string | yes | — | File glob pattern to match |
| `module_path` | string | yes | — | Path to WASM module |
| `function_name` | string | yes | — | Exported function name to call |
| `max_fuel` | integer | no | 1,000,000,000 | Fuel limit for execution |
| `max_memory_bytes` | integer | no | 67,108,864 | Memory limit in bytes (64 MB) |

**Response** `201 Created`

```json
{
  "status": "registered",
  "pattern": "*.pdf",
  "module_path": "/data/workers/uuid-ocr.wasm",
  "function_name": "process"
}
```

### POST `/api/workers/upload`

Upload a WASM module file. Requires `--data-dir` to be set. Validates WASM magic bytes (`0x00 0x61 0x73 0x6D`).

**Request**

`Content-Type: multipart/form-data` with a `file` field containing the `.wasm` file.

**Response** `201 Created`

```json
{
  "module_path": "/data/workers/550e8400-e29b-41d4-a716-446655440000-worker.wasm",
  "size": 1024,
  "filename": "550e8400-e29b-41d4-a716-446655440000-worker.wasm"
}
```

### GET `/api/workers/modules`

List uploaded WASM modules in the workers directory.

**Response** `200 OK`

```json
{
  "modules": [
    {
      "filename": "550e8400-worker.wasm",
      "module_path": "/data/workers/550e8400-worker.wasm",
      "size": 1024,
      "uploaded_at": "2026-04-22T12:00:00Z"
    }
  ]
}
```

### DELETE `/api/workers/modules/{filename}`

Delete an uploaded WASM module by filename.

**Response** `200 OK`

```json
{
  "status": "deleted",
  "filename": "550e8400-worker.wasm"
}
```

## Cedar Policies

### GET `/api/policies`

List Cedar policy status.

**Response** `200 OK`

```json
{
  "policies": [],
  "configured": true
}
```

### POST `/api/policies`

Add a new Cedar policy.

**Request Body**

```json
{
  "policy": "permit(principal, action == Action::\"read\", resource);"
}
```

**Response** `201 Created`

```json
{
  "status": "added"
}
```

**Response** `400 Bad Request` (invalid policy)

```json
{
  "error": "Invalid policy: parse error: ..."
}
```

### DELETE `/api/policies`

Remove a policy by ID. Currently returns `501 Not Implemented` as Cedar's `PolicySet` does not support individual removal.

**Request Body**

```json
{
  "policy_id": "policy-id"
}
```

**Response** `501 Not Implemented`

```json
{
  "error": "Policy removal requires reloading the full policy set. Use PUT /api/policies to replace all policies."
}
```

## Pre-Signed URLs

### GET `/api/upload-url`

Generate a pre-signed URL for direct file upload to cloud storage.

**Query Parameters**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | string | — | Target file path (required) |
| `expires` | integer | 3600 | URL expiration time in seconds |

**Response** `200 OK`

```json
{
  "url": "https://bucket.s3.amazonaws.com/path/to/file?X-Amz-Algorithm=...",
  "method": "PUT",
  "expires_in": 3600,
  "path": "/path/to/file"
}
```

### GET `/api/download-url`

Generate a pre-signed URL for direct file download from cloud storage.

**Query Parameters** — Same as `/api/upload-url`.

**Response** — Same format, with `"method": "GET"`.

Both endpoints return `503 Service Unavailable` with `{"error": "Pre-signed URLs not configured"}` if no pre-signed URL generator is configured.

## Share Links

### GET `/api/shares`

List all active share links.

**Response** `200 OK`

```json
{
  "shares": [
    {
      "token": "550e8400-e29b-41d4-a716-446655440000",
      "url": "/s/550e8400-e29b-41d4-a716-446655440000",
      "path": "/documents/report.pdf",
      "expires_at": "2026-04-29T12:00:00+00:00",
      "max_downloads": 10,
      "download_count": 3,
      "created_by": "anonymous"
    }
  ]
}
```

### POST `/api/shares`

Create a new share link.

**Request Body**

```json
{
  "path": "/documents/report.pdf",
  "password": "secret123",
  "expires_in_hours": 48,
  "max_downloads": 10
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path to share |
| `password` | string | no | (none) | Password to protect the share |
| `expires_in_hours` | integer | no | 168 (7 days) | Expiration time in hours |
| `max_downloads` | integer | no | (none) | Maximum number of downloads |

**Response** `201 Created`

```json
{
  "token": "550e8400-e29b-41d4-a716-446655440000",
  "url": "/s/550e8400-e29b-41d4-a716-446655440000",
  "path": "/documents/report.pdf",
  "expires_at": "2026-04-24T12:00:00+00:00",
  "max_downloads": 10
}
```

### DELETE `/api/shares/:token`

Delete a share link.

**Response** `204 No Content` on success, `404 Not Found` if the token does not exist.

### GET `/s/:token`

Public share download endpoint. If the share requires a password, include `?password=...`.

**Query Parameters**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `password` | string | conditional | Required if share has a password |

**Response** `200 OK` — Returns the file content with appropriate `Content-Type` and `Content-Disposition` headers.

**Error Responses**

| Status | Condition |
|--------|-----------|
| `401` | Password required or invalid |
| `404` | Share token or file not found |
| `410` | Share expired or download limit reached |

## Audit Log

### GET `/api/audit`

Returns the most recent 100 audit log entries.

**Response** `200 OK`

```json
{
  "entries": [
    {
      "timestamp": "2026-04-22T12:00:00+00:00",
      "method": "PUT",
      "path": "/documents/report.pdf",
      "user": "user-123",
      "status": 201,
      "client_ip": "192.168.1.100",
      "user_agent": "rclone/1.65.0"
    }
  ]
}
```

## Storage Statistics

### GET `/api/storage/stats`

Returns file and collection counts, total size, and CAS statistics.

**Response** `200 OK`

```json
{
  "files": 42,
  "collections": 8,
  "total_bytes": 10485760,
  "cas": {
    "enabled": true,
    "content_blocks": 35
  },
  "metadata_store": true
}
```

## Snapshots

### GET `/api/snapshots`

List all snapshots.

**Response** `200 OK`

```json
{
  "snapshots": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "description": "Before migration",
      "created_at": "2026-04-22T12:00:00+00:00",
      "entry_count": 150
    }
  ]
}
```

### POST `/api/snapshots`

Create a point-in-time snapshot of all current metadata.

**Request Body**

```json
{
  "description": "Before migration"
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `description` | string | no | `Manual snapshot` | Snapshot description |

**Response** `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "description": "Before migration",
  "created_at": "2026-04-22T12:00:00+00:00",
  "entry_count": 150
}
```

### DELETE `/api/snapshots/:id`

Delete a snapshot by ID.

**Response** `204 No Content` on success, `404 Not Found` if not found.

### POST `/api/snapshots/:id/restore`

Restore a snapshot. Recreates collections that were deleted since the snapshot, and reports files that are still intact or have missing content.

**Response** `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "entries": 150,
  "files_intact": 140,
  "collections_created": 2,
  "missing_content": 8
}
```

## WOPI Protocol

### GET `/hosting/discovery`

Returns the WOPI discovery XML document listing supported operations for various file types.

**Response** `200 OK` — `Content-Type: application/xml; charset=utf-8`

```xml
<?xml version="1.0" encoding="utf-8"?>
<wopi-discovery>
  <net-zone name="external-https">
    <app name="Edit" favIconUrl="" checkLicense="true">
      <action name="edit" ext="odt" urlsrc=""/>
      <action name="edit" ext="docx" urlsrc=""/>
      <!-- ... more file types ... -->
    </app>
    <app name="View" favIconUrl="" checkLicense="true">
      <action name="view" ext="pdf" urlsrc=""/>
      <!-- ... more file types ... -->
    </app>
  </net-zone>
</wopi-discovery>
```

### GET `/wopi/files/{path}`

WOPI CheckFileInfo. Returns metadata about a file for Office Online.

**Query Parameters**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `access_token` | string | yes | WOPI access token |

**Response** `200 OK`

```json
{
  "base_file_name": "report.docx",
  "size": 102400,
  "version": "abc123",
  "last_modified_time": "2026-04-22T12:00:00+00:00",
  "owner_id": "user-123",
  "user_can_write": true,
  "user_can_not_write_relative": false,
  "supports_update": true,
  "supports_locks": true,
  "supports_coauth": false
}
```

### GET `/wopi/files/{path}/contents`

Download file content via WOPI.

**Query Parameters**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `access_token` | string | yes | WOPI access token |

Returns the file content with appropriate `Content-Type`.

### POST `/wopi/files/{path}`

WOPI operations: PUT file, LOCK, UNLOCK.

**Headers**

| Header | Description |
|--------|-------------|
| `X-WOPI-Override` | Operation: `LOCK`, `UNLOCK`, or omitted for PUT |
| `X-WOPI-AccessToken` | WOPI access token |
| `X-WOPI-Lock` | Lock token (for PUT with lock) |
| `X-WOPI-LockId` | Lock ID (for UNLOCK) |

**Response** — `200 OK` on success, `409 Conflict` if file is locked by another user.

### POST `/wopi/files/{path}/token`

Issue a time-limited WOPI access token (valid for 8 hours).

**Response** `200 OK`

```json
{
  "access_token": "base64-encoded-token",
  "expires_in": 28800,
  "token_ttl": 28800
}
```

## Rate Limiting

All endpoints are subject to per-IP rate limiting (10,000 requests per 60 seconds).

**Response** `429 Too Many Requests`

```json
{
  "error": "Rate limit exceeded",
  "retry_after": 60
}
```

Header: `Retry-After: 60`

## CORS

Cross-origin requests are handled automatically. Preflight `OPTIONS` requests receive appropriate CORS headers. Same-origin requests (no `Origin` header) pass through untouched, preserving WebDAV `DAV` and `Allow` headers.

**Preflight Response Headers**

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, PATCH, OPTIONS, PROPFIND, MKCOL, COPY, MOVE, LOCK, UNLOCK, PROPPATCH
Access-Control-Allow-Headers: Content-Type, Authorization, Depth, Destination, If, If-Match, If-None-Match, Lock-Token, Overwrite
Access-Control-Max-Age: 86400
```
