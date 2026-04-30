# Configuration

Ferro supports three layers of configuration, applied in order of priority:

1. **CLI flags** (highest priority)
2. **Environment variables**
3. **TOML config file** (lowest priority)

## CLI Flags

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--host` | -- | `0.0.0.0` | Bind address |
| `-p, --port` | -- | `8080` | Server port |
| `--log-level` | -- | `info` | Log level (trace, debug, info, warn, error) |
| `--log-format` | `FERRO_LOG_FORMAT` | `text` | Log format: `text` or `json` |
| `--storage` | -- | `memory` | Storage backend (`memory`, `local:/path`, `s3://bucket`, `gs://bucket`, `az://container`) |
| `--data-dir` | `FERRO_DATA_DIR` | (none) | Persistent data directory (enables SQLite metadata, CAS, snapshots, audit) |
| `--static-dir` | `FERRO_STATIC_DIR` | (none) | Web UI static files directory |
| `--max-body-size` | `FERRO_MAX_BODY_SIZE` | `1073741824` | Max request body size in bytes (1 GB) |
| `--wasm-enabled` | `FERRO_WASM_ENABLED` | `false` | Enable WASM worker runtime |
| `--cas-enabled` | -- | `false` | Enable in-memory content-addressable deduplication |
| `--search-index-path` | -- | (auto) | Search index directory |
| `--metadata-db` | `FERRO_METADATA_DB` | (none) | PostgreSQL metadata database URL |
| `--oidc-issuer` | `FERRO_OIDC_ISSUER` | (none) | OIDC issuer URL (enables authentication) |
| `--oidc-client-id` | `FERRO_OIDC_CLIENT_ID` | (none) | OIDC client ID |
| `--oidc-audience` | `FERRO_OIDC_AUDIENCE` | `ferro` | OIDC audience claim |
| `--oidc-jwks-uri` | `FERRO_OIDC_JWKS_URI` | (none) | JWKS URI (overrides auto-discovery) |
| `--cedar-policy-file` | `FERRO_CEDAR_POLICY_FILE` | (none) | Path to Cedar policy file |
| `--admin-user` | `FERRO_ADMIN_USER` | (none) | Admin username for HTTP Basic Auth |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | (none) | Admin password for HTTP Basic Auth |
| `--config` | `FERRO_CONFIG` | (none) | Path to configuration file (TOML format) |
| `--external-url` | `FERRO_EXTERNAL_URL` | `http://localhost:8080` | External base URL for OIDC callbacks |
| `--wopi-token-secret` | `FERRO_WOPI_TOKEN_SECRET` | (default) | HMAC secret for WOPI tokens |
| `--wopi-office-url` | `FERRO_WOPI_OFFICE_URL` | (none) | Collabora/OnlyOffice server URL |
| `--federation-secret` | `FERRO_FEDERATION_SECRET` | (none) | Secret for federation HTTP Signatures |
| `--storage-quota` | `FERRO_STORAGE_QUOTA` | (none) | Storage quota (e.g., `10GB`, `500MB`) |
| `--trash-ttl` | `FERRO_TRASH_TTL` | `30d` | Trash auto-purge TTL (`0` to disable) |
| `--graceful-shutdown-timeout` | `FERRO_GRACEFUL_SHUTDOWN_TIMEOUT` | `30` | Graceful shutdown timeout in seconds |
| `--cors-allowed-origins` | `FERRO_CORS_ALLOWED_ORIGINS` | `*` | Comma-separated CORS origins |
| `--max-file-versions` | `FERRO_MAX_FILE_VERSIONS` | `10` | Max file versions to retain (0 = disabled) |
| `--thumbnail-size` | `FERRO_THUMBNAIL_SIZE` | `256` | Max thumbnail dimension in pixels (64-1024) |
| `--multi-user` | `FERRO_MULTI_USER` | `false` | Enable multi-user mode with per-user home directories |

## TOML Config File

Ferro can load settings from a `ferro.toml` file:

```bash
ferro-server --config /path/to/ferro.toml
```

Auto-discovery searches the current directory, then `/etc/ferro/ferro.toml`.

### Example `ferro.toml`

```toml
host = "0.0.0.0"
port = 8080
storage = "local:/data/files"
data_dir = "/var/lib/ferro"
admin_user = "admin"
admin_password = "changeme"
log_level = "info"
log_format = "json"
external_url = "https://ferro.example.com"
federation_secret = "a-random-secret-string"
cas_enabled = true
wasm_enabled = true
storage_quota = "100GB"
trash_ttl = "30d"
cors_allowed_origins = "https://ferro.example.com"
```

### Size Format

Fields that accept byte sizes support human-readable formats:

```toml
max_body_size = "2GB"
storage_quota = "500MB"
```

Supported suffixes: `B`, `KB`, `MB`, `GB`.

## Environment Variables

All CLI flags have corresponding environment variables (see table above). Example:

```bash
FERRO_ADMIN_USER=admin FERRO_ADMIN_PASSWORD=secret FERRO_DATA_DIR=/var/lib/ferro ferro-server
```

## Config Layering (Include)

Configuration files support `include` directives for modular configuration:

```toml
# base.toml
host = "0.0.0.0"
port = 8080
log_level = "info"
include = ["production.toml"]
```

```toml
# production.toml
data_dir = "/var/lib/ferro"
log_format = "json"
external_url = "https://ferro.example.com"
```

Included files are merged with later files overriding earlier ones. Include paths are resolved relative to the including file. Circular includes are detected and rejected.

## Config Precedence

When the same setting is defined in multiple sources, the order of precedence is:

1. **CLI flags** always win
2. **Environment variables** override file values
3. **TOML config file** provides defaults

CLI flags set on the command line will never be overridden by config file values, even if the config file is loaded explicitly.

## Storage Backend Configuration

### In-Memory (default)

```bash
ferro-server
```

All data is lost on restart.

### Local Filesystem

```bash
ferro-server --storage local:/path/to/files
```

### Amazon S3

```bash
cargo build --release --features s3 --bin ferro-server
./target/release/ferro-server --storage s3://my-bucket
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=...
```

### Google Cloud Storage

```bash
cargo build --release --features gcs --bin ferro-server
./target/release/ferro-server --storage gs://my-bucket
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

### Azure Blob Storage

```bash
cargo build --release --features azure --bin ferro-server
./target/release/ferro-server --storage az://my-container
export AZURE_STORAGE_ACCOUNT_NAME=...
export AZURE_STORAGE_ACCOUNT_KEY=...
```

## Persistent Data

Use `--data-dir` to enable SQLite-backed persistence for metadata, CAS deduplication, snapshots, and audit logs in a single database:

```bash
ferro-server --data-dir /var/lib/ferro
```

This creates `/var/lib/ferro/ferro.db` with WAL mode for concurrent access. CAS deduplication is automatically enabled when `--data-dir` is set.

> **Warning:** Without `--data-dir`, all data will be lost on restart.
