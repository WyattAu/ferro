# Configuration Reference

## CLI Flags

All flags can be set via environment variables (noted in the table). Environment variables take precedence over CLI defaults.

```
ferro-server [OPTIONS]
```

### Server

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--host` | — | string | `0.0.0.0` | Bind address |
| `-p, --port` | — | u16 | `8080` | Server port |
| `--log-level` | — | string | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |

### Network

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--external-url` | `FERRO_EXTERNAL_URL` | string | `http://localhost:8080` | External base URL for OIDC callbacks |

### Storage

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--storage` | — | string | `memory` | Storage backend URL |
| `--data-dir` | `FERRO_DATA_DIR` | string | (none) | Persistent data directory |
| `--metadata-db` | `FERRO_METADATA_DB` | string | (none) | PostgreSQL metadata database URL |
| `--cas-enabled` | — | bool | `false` | Enable in-memory CAS deduplication |
| `--max-body-size` | `FERRO_MAX_BODY_SIZE` | u64 | `1073741824` | Max request body size in bytes |
| `--static-dir` | `FERRO_STATIC_DIR` | string | (none) | Web UI static files directory |

### Search

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--search-index-path` | — | string | `/tmp/ferro-search` | Tantivy search index directory |

### Authentication

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--admin-user` | `FERRO_ADMIN_USER` | string | (none) | Admin username for simple HTTP Basic Auth |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | string | (none) | Admin password for simple HTTP Basic Auth |

Both `--admin-user` and `--admin-password` must be set to enable simple auth. When enabled, the server requires HTTP Basic Auth for all requests and `/api/auth/info` returns `"auth_type": "basic"`.

### Configuration

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--config` | `FERRO_CONFIG` | string | (none) | Path to configuration file (TOML format) |

### OIDC Authentication

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--oidc-issuer` | `FERRO_OIDC_ISSUER` | string | (none) | OIDC issuer URL |
| `--oidc-client-id` | `FERRO_OIDC_CLIENT_ID` | string | (none) | OIDC client ID |
| `--oidc-audience` | `FERRO_OIDC_AUDIENCE` | string | `ferro` | OIDC audience claim |
| `--oidc-jwks-uri` | `FERRO_OIDC_JWKS_URI` | string | (none) | JWKS URI (overrides auto-discovery) |

### Authorization

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--cedar-policy-file` | `FERRO_CEDAR_POLICY_FILE` | string | (none) | Path to Cedar policy file |

### WASM Workers

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--wasm-enabled` | `FERRO_WASM_ENABLED` | bool | `false` | Enable WASM worker runtime |

### WOPI

| Flag | Env Var | Type | Default | Description |
|------|---------|------|---------|-------------|
| `--wopi-token-secret` | `FERRO_WOPI_TOKEN_SECRET` | string | `ferro-wopi-token-secret-change-me` | HMAC secret for WOPI tokens |
| `--wopi-office-url` | `FERRO_WOPI_OFFICE_URL` | string | (none) | Collabora/OnlyOffice server URL for WOPI |

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
- `AWS_ACCESS_KEY_ID` — AWS access key
- `AWS_SECRET_ACCESS_KEY` — AWS secret key
- `AWS_REGION` — AWS region (e.g., `us-east-1`)

### Google Cloud Storage

Requires the `gcs` feature flag.

```
--storage gs://bucket-name
--storage gs://bucket-name/prefix
```

Environment variables:
- `GOOGLE_APPLICATION_CREDENTIALS` — Path to service account JSON key file

### Azure Blob Storage

Requires the `azure` feature flag.

```
--storage az://container-name
--storage azure://container-name
```

Environment variables:
- `AZURE_STORAGE_ACCOUNT_NAME` — Storage account name
- `AZURE_STORAGE_ACCOUNT_KEY` — Storage account key

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

```bash
# Build with all cloud backends
cargo build --features s3,gcs,azure

# Build only S3
cargo build --features s3

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

### Minimal Docker Environment

```yaml
environment:
  FERRO_DATA_DIR: /data
  FERRO_STATIC_DIR: /app/ui
  FERRO_WASM_ENABLED: "true"
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
storage = "memory"
data_dir = "/var/lib/ferro"
static_dir = "/var/www/ferro/ui"
max_body_size = "1073741824"
admin_user = "admin"
admin_password = "changeme"
external_url = "http://localhost:8080"
wopi_token_secret = "ferro-wopi-token-secret-change-me"
wopi_office_url = ""
oidc_issuer = ""
oidc_client_id = ""
oidc_audience = "ferro"
oidc_jwks_uri = ""
cedar_policy_file = ""
search_index_path = "/tmp/ferro-search"
metadata_db = ""
cas_enabled = false
wasm_enabled = false
```

All keys are optional. Omit a key to use its default value.
