# Configuration Reference

## CLI Flags

All flags can be set via environment variables (noted in the table). CLI flags take precedence over environment variables, which take precedence over configuration file values.

```
ferro-server [OPTIONS]
```

### Server

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--config` | `FERRO_CONFIG` | string | (none) | Path to configuration file (TOML format). Supports `include` for multi-file config with cycle detection. |
| `--host` | — | string | `0.0.0.0` | Bind address |
| `-p, --port` | — | u16 | `8080` | Server port |
| `--log-level` | — | string | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `--log-format` | `FERRO_LOG_FORMAT` | string | `text` | Log format: `text` or `json` (structured JSON logging) |
| `--external-url` | `FERRO_EXTERNAL_URL` | string | `http://localhost:8080` | External base URL for OIDC callbacks and WOPI redirects |
| `--api-version` | `FERRO_API_VERSION` | string | `v1` | API version prefix. Routes are mounted at `/api/{version}`. |
| `--maintenance-mode` | `FERRO_MAINTENANCE_MODE` | bool | `false` | Start in maintenance mode. All write operations (PUT/DELETE/POST/PATCH/MKCOL/COPY/MOVE) return 503. GET/HEAD/OPTIONS pass through. Toggable at runtime via `POST /api/admin/maintenance`. |
| `--graceful-shutdown-timeout` | `FERRO_GRACEFUL_SHUTDOWN_TIMEOUT` | u64 | `30` | Graceful shutdown timeout in seconds. After receiving SIGTERM/SIGINT, the server drains connections and waits for background tasks (indexer, worker runner, trash purge, lock cleanup) to finish within this window. |

### Storage

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--storage` | — | string | `memory` | Storage backend: `memory`, `local:/path`, `s3://bucket`, `gs://bucket`, `az://container` |
| `--data-dir` | `FERRO_DATA_DIR` | string | (none) | Persistent data directory for SQLite. When set, all in-memory stores (metadata, CAS, snapshots, audit, shares) are replaced with SQLite-backed persistence. Creates `<data-dir>/ferro.db` with WAL mode. |
| `--metadata-db` | `FERRO_METADATA_DB` | string | (none) | PostgreSQL metadata database URL (alternative to `--data-dir` for metadata only) |
| `--cas-enabled` | — | bool | `false` | Enable content-addressable storage deduplication |
| `--max-body-size` | `FERRO_MAX_BODY_SIZE` | u64 | `1073741824` (1 GB) | Maximum request body size in bytes |
| `--static-dir` | `FERRO_STATIC_DIR` | string | (none) | Path to static web assets (serves index.html, JS, WASM) |
| `--storage-quota` | `FERRO_STORAGE_QUOTA` | string | (none) | Storage quota per user (e.g., `10GB`, `500MB`, `1TB`). Empty means unlimited. |

### Search

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--search-index-path` | — | string | `{data-dir}/search-index` or `/tmp/ferro-search` | Tantivy search index directory |

### Authentication

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--admin-user` | `FERRO_ADMIN_USER` | string | (none) | Admin username for simple HTTP Basic Auth. Enables Basic Auth when set with `--admin-password`. |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | string | (none) | Admin password for simple HTTP Basic Auth (plain text, use env var in production). Weak passwords (`changeme`, `admin`, `password`, `ferro`, empty) trigger a warning on startup and force a password change before any other API access. |
| `--oidc-issuer` | `FERRO_OIDC_ISSUER` | string | (none) | OIDC issuer URL (enables OIDC authentication) |
| `--oidc-client-id` | `FERRO_OIDC_CLIENT_ID` | string | (none) | OIDC client ID |
| `--oidc-audience` | `FERRO_OIDC_AUDIENCE` | string | `ferro` | OIDC audience claim |
| `--oidc-jwks-uri` | `FERRO_OIDC_JWKS_URI` | string | (none) | JWKS URI (overrides auto-discovery from issuer) |
| `--multi-user` | `FERRO_MULTI_USER` | bool | `false` | Enable multi-user mode with per-user home directories |

### Authorization

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--cedar-policy-file` | `FERRO_CEDAR_POLICY_FILE` | string | (none) | Path to Cedar policy file for fine-grained access control |

### WASM Workers

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--wasm-enabled` | `FERRO_WASM_ENABLED` | bool | `false` | Enable WASM worker runtime |

### WOPI (Office Online Integration)

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--wopi-token-secret` | `FERRO_WOPI_TOKEN_SECRET` | string | (none) | HMAC-SHA256 secret for signing WOPI access tokens. Required when WOPI is enabled. |
| `--wopi-office-url` | `FERRO_WOPI_OFFICE_URL` | string | `""` (empty) | Collabora/OnlyOffice server URL (e.g., `https://collabora.example.com`). When empty, WOPI discovery returns 503 NOT_CONFIGURED. |

### Federation

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--federation-secret` | `FERRO_FEDERATION_SECRET` | string | `""` (empty) | HMAC-SHA256 secret for verifying HTTP Signatures on the ActivityPub federation inbox. When empty, federation is disabled and the inbox returns 503. |

### Trash

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--trash-ttl` | `FERRO_TRASH_TTL` | string | `30d` | Trash auto-purge TTL (e.g., `30d`, `7d`, `24h`, `0` to disable). Purge runs hourly. |

### CORS

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--cors-allowed-origins` | `FERRO_CORS_ALLOWED_ORIGINS` | string | `*` | Comma-separated list of allowed CORS origins. `*` allows all origins. |
| `--cors-origins` | `FERRO_CORS_ORIGINS` | string | `*` | Alias for `--cors-allowed-origins` |

### Versioning

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--max-file-versions` | `FERRO_MAX_FILE_VERSIONS` | u64 | `10` | Maximum number of file versions to retain per file. `0` disables versioning. |

### Thumbnails

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--thumbnail-size` | `FERRO_THUMBNAIL_SIZE` | u32 | `256` | Maximum thumbnail dimension in pixels (range: 64-1024) |

### Feature-Gated: PostgreSQL Distributed State

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--database-url` | `FERRO_DATABASE_URL` | string | (none) | PostgreSQL URL for distributed state (shares, favorites, preferences). Requires `pg` feature flag at compile time. |

### Feature-Gated: Redis

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--redis-url` | `FERRO_REDIS_URL` | string | (none) | Redis URL for distributed locking and rate limiting. Requires `redis` feature flag at compile time. |

### Feature-Gated: LDAP

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--ldap-url` | `FERRO_LDAP_URL` | string | (none) | LDAP server URL. Enables LDAP authentication. Requires `ldap` feature flag. |
| `--ldap-bind-dn` | `FERRO_LDAP_BIND_DN` | string | (none) | LDAP bind DN for service account |
| `--ldap-bind-password` | `FERRO_LDAP_BIND_PASSWORD` | string | (none) | LDAP service account password |
| `--ldap-user-search-base` | `FERRO_LDAP_USER_SEARCH_BASE` | string | `""` (empty) | LDAP user search base DN |

## Storage Backend URLs

### In-Memory

Files are stored in RAM and lost on restart. Useful for testing.

```
--storage memory
```

### Local Filesystem

Files are stored on the local disk.

```
--storage local:/path/to/directory
```

The directory must exist and be writable by the Ferro process.

### Amazon S3

Requires the `s3` feature flag and environment credentials.

```
--storage s3://bucket-name
--storage s3://bucket-name/prefix
```

Environment variables:
- `AWS_ACCESS_KEY_ID` -- AWS access key
- `AWS_SECRET_ACCESS_KEY` -- AWS secret key
- `AWS_REGION` -- AWS region (e.g., `us-east-1`)

### Google Cloud Storage

Requires the `gcs` feature flag.

```
--storage gs://bucket-name
--storage gs://bucket-name/prefix
```

Environment variables:
- `GOOGLE_APPLICATION_CREDENTIALS` -- Path to service account JSON key file

### Azure Blob Storage

Requires the `azure` feature flag.

```
--storage az://container-name
--storage azure://container-name
```

Environment variables:
- `AZURE_STORAGE_ACCOUNT_NAME` -- Storage account name
- `AZURE_STORAGE_ACCOUNT_KEY` -- Storage account key

## Persistence Modes

Ferro supports three persistence modes, configured by different flag combinations:

### Mode 1: Fully Ephemeral (Default)

No flags. All data (metadata, shares, audit log, snapshots) is in-memory.

```bash
ferro-server
```

### Mode 2: Unified SQLite Persistence

Use `--data-dir` for a single SQLite database covering metadata, CAS, snapshots, and audit:

```bash
ferro-server --data-dir /var/lib/ferro
```

Creates `<data-dir>/ferro.db` with WAL mode. CAS deduplication is automatically enabled.

### Mode 3: PostgreSQL Metadata

Use `--metadata-db` for PostgreSQL-backed metadata only:

```bash
ferro-server --metadata-db postgres://user:pass@host:5432/ferro
```

Snapshots and audit remain in-memory. CAS must be enabled separately with `--cas-enabled`.

## Feature Flags

Feature flags are set at compile time via Cargo:

| Flag | Crate | Description |
|------|-------|-------------|
| `s3` | `ferro-core`, `ferro-server` | Amazon S3 storage backend |
| `gcs` | `ferro-core`, `ferro-server` | Google Cloud Storage backend |
| `azure` | `ferro-core`, `ferro-server` | Azure Blob Storage backend |
| `pg` | `ferro-server` | PostgreSQL distributed state |
| `redis` | `ferro-server` | Redis distributed locking and rate limiting |
| `ldap` | `ferro-server` | LDAP authentication |

```bash
# Build with all cloud backends
cargo build --features s3,gcs,azure

# Build with PostgreSQL and Redis
cargo build --features pg,redis

# Build release binary
cargo build --release --features s3,gcs,azure --bin ferro-server
```

## Cedar Policy File

When OIDC is enabled, Cedar authorization is automatically activated. By default, a permissive policy allows all operations. Provide a custom policy file for fine-grained access control.

Example `policies.cedar`:

```cedar
permit(
  principal,
  action in [Action::"read", Action::"list"],
  resource
);

permit(
  principal == User::"admin@example.com",
  action in [Action::"write", Action::"delete", Action::"admin"],
  resource
);
```

## HTTP Method to Cedar Action Mapping

| HTTP Method | Cedar Action |
|-------------|-------------|
| GET, HEAD, PROPFIND | `read` |
| MKCOL, OPTIONS | `list` |
| PUT, COPY, MOVE, LOCK, PROPPATCH | `write` |
| DELETE, UNLOCK | `delete` |
| POST (non-WebDAV) | `admin` |

## Rate Limiting

Built-in per-IP rate limiter:

| Setting | Value |
|---------|-------|
| Max requests | 10,000 per window |
| Window duration | 60 seconds |
| Response on limit | `429 Too Many Requests` |
| Retry-After header | `60` seconds |

Rate limiting is always active and cannot be disabled.

## Default Password Protection

When simple auth is enabled (`--admin-user` + `--admin-password`), Ferro checks the password against known weak values: `changeme`, `admin`, `password`, `ferro`, and empty string.

- On startup: warning logged if password is weak
- On first request: all API endpoints return 403 `PASSWORD_CHANGE_REQUIRED` until password is changed via `POST /api/auth/change-password`
- Exempt paths: `/api/auth/change-password`, `/api/auth/login`, `/api/auth/info`, `/api/config`, `/healthz`, `/readyz`, `/metrics/prometheus`

## Maintenance Mode

When maintenance mode is active, all write HTTP methods (PUT, DELETE, POST, PATCH, MKCOL, COPY, MOVE) return 503 with error code `MAINTENANCE_MODE`. Read methods (GET, HEAD, OPTIONS) and the admin toggle endpoint (`POST /api/admin/maintenance`) remain accessible.

- CLI: `--maintenance-mode` flag
- Runtime: `POST /api/admin/maintenance` (admin only)
- Status: `GET /api/admin/maintenance`

## Graceful Shutdown

On SIGTERM/SIGINT, the server:
1. Stops accepting new connections
2. Drains in-flight requests
3. Cancels background tasks: indexer, worker runner, trash purge, lock cleanup
4. Commits the search index to disk
5. Logs SQLite WAL status
6. Exits after `--graceful-shutdown-timeout` seconds (default: 30)

## Example Configurations

### Development

```bash
ferro-server --log-level debug --storage local:./dev-data
```

### Production with Persistence

```bash
ferro-server \
  --host 0.0.0.0 \
  --port 8080 \
  --data-dir /var/lib/ferro \
  --static-dir /var/www/ferro/ui \
  --wasm-enabled
```

### Production with OIDC and Cloud Storage

```bash
ferro-server \
  --host 0.0.0.0 \
  --port 8080 \
  --storage s3://ferro-files \
  --data-dir /var/lib/ferro \
  --oidc-issuer https://keycloak.example.com/realms/ferro \
  --oidc-client-id ferro \
  --cedar-policy-file /etc/ferro/policies.cedar \
  --wasm-enabled
```

### Production with WOPI (Collabora/OnlyOffice)

```bash
ferro-server \
  --data-dir /var/lib/ferro \
  --wopi-office-url https://collabora.example.com \
  --wopi-token-secret "$(openssl rand -hex 32)"
```

### Minimal Docker Environment

```yaml
environment:
  FERRO_DATA_DIR: /data
  FERRO_STATIC_DIR: /app/ui
  FERRO_WASM_ENABLED: "true"
  FERRO_LOG_FORMAT: json
  RUST_LOG: info
```

## Configuration File

Ferro supports a TOML configuration file (`ferro.toml`). Use `--config` to specify a path:

```bash
ferro-server --config /path/to/ferro.toml
```

If `--config` is not set, Ferro searches for `ferro.toml` in the current directory, then `/etc/ferro/ferro.toml`. CLI flags and environment variables take precedence over file values.

### Supported Keys

```toml
host = "0.0.0.0"
port = 8080
log_level = "info"
log_format = "text"
storage = "memory"
data_dir = "/var/lib/ferro"
static_dir = "/var/www/ferro/ui"
max_body_size = "1073741824"
admin_user = "admin"
admin_password = "changeme"
external_url = "http://localhost:8080"
wopi_token_secret = ""
wopi_office_url = ""
federation_secret = ""
oidc_issuer = ""
oidc_client_id = ""
oidc_audience = "ferro"
oidc_jwks_uri = ""
cedar_policy_file = ""
search_index_path = "/tmp/ferro-search"
metadata_db = ""
cas_enabled = false
wasm_enabled = false
storage_quota = ""
trash_ttl = "30d"
graceful_shutdown_timeout = 30
cors_allowed_origins = "*"

# Multi-file config with include (supports recursive, cycle-detected)
include = ["overrides.toml", "secrets.toml"]
```

All keys are optional. Omit a key to use its default value.
