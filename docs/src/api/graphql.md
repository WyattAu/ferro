# GraphQL API

Ferro provides a GraphQL endpoint built with `async-graphql`. A Playground UI is available in the browser.

## Endpoint

```
POST /api/graphql
GET  /api/graphql   (Playground UI)
```

## Schema Overview

### Queries

| Query | Arguments | Description |
|-------|-----------|-------------|
| `files` | `path: String`, `limit: Int` | List files at a path (default limit: 100, max: 1000) |
| `file` | `path: String!` | Get metadata for a single file |
| `shares` | -- | List all share links |
| `me` | -- | Get current user info |
| `health` | -- | Server health check |
| `audit_log` | `limit: Int`, `offset: Int` | Query audit log entries |

### Mutations

| Mutation | Arguments | Description |
|----------|-----------|-------------|
| `create_folder` | `path: String!` | Create a directory |
| `delete_file` | `path: String!` | Delete a file or directory |

### Types

**FileItem**

| Field | Type | Description |
|-------|------|-------------|
| `path` | `String` | Full path |
| `name` | `String` | File name |
| `size` | `Int` | Size in bytes |
| `is_collection` | `Boolean` | Whether it is a directory |
| `mime_type` | `String` | MIME type |
| `modified` | `String` | Last modified timestamp |
| `owner` | `String` | Owner username |

**ShareItem**

| Field | Type | Description |
|-------|------|-------------|
| `token` | `String` | Share token |
| `path` | `String` | Shared file path |
| `expires_at` | `String` | Expiration timestamp |
| `password_protected` | `Boolean` | Has password protection |
| `max_downloads` | `Int` | Maximum download count |
| `download_count` | `Int` | Current download count |
| `created_by` | `String` | Creator username |

**UserItem**

| Field | Type | Description |
|-------|------|-------------|
| `username` | `String` | Username |
| `role` | `String` | User role |

**HealthItem**

| Field | Type | Description |
|-------|------|-------------|
| `status` | `String` | Health status |
| `version` | `String` | Server version |

**AuditItem**

| Field | Type | Description |
|-------|------|-------------|
| `method` | `String` | HTTP method |
| `path` | `String` | Request path |
| `user` | `String` | User who performed the action |
| `status` | `Int` | HTTP status code |
| `timestamp` | `String` | When the action occurred |

## Examples

### List root files

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ files(path: \"/\") { name size is_collection modified } }"}'
```

### Get a specific file

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ file(path: \"/documents/report.pdf\") { path name size mime_type modified } }"}'
```

### Create a folder

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { create_folder(path: \"/new-folder\") { path name } }"}'
```

### Delete a file

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "mutation { delete_file(path: \"/old-file.txt\") }"}'
```

### Query audit log

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ audit_log(limit: 10) { method path user status timestamp } }"}'
```

### Check health

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ health { status version } }"}'
```

### List shares

```bash
curl -X POST http://localhost:8080/api/graphql \
  -H "Authorization: Bearer TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "{ shares { token path password_protected download_count } }"}'
```

## Playground

Open `http://localhost:8080/api/graphql` in a browser to access the interactive GraphQL Playground with schema documentation and auto-completion.
