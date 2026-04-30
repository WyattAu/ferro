# REST API

The REST API provides JSON endpoints for file operations, user management, sharing, and server administration.

All endpoints are prefixed with `/api/`. Authentication is required when `--admin-user` or `--oidc-issuer` is configured.

## Health Endpoints

### Health check

```bash
curl http://localhost:8080/.well-known/ferro
```

```json
{
  "version": "2.5.0",
  "storage": "ok"
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
  "version": "2.5.0",
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

## Audit Log

```bash
curl "http://localhost:8080/api/audit?limit=50&offset=0" \
  -H "Authorization: Bearer TOKEN"
```
