# Ferro v1.0.0-beta.1 — First Public Beta

## What is Ferro?

Ferro is a self-hosted file storage orchestrator written in 100% Rust. Think Nextcloud, but fast, lightweight, and without the PHP. It speaks WebDAV natively, so it works with every file manager on every operating system.

## Quick Start

### Docker (recommended)
```bash
git clone https://github.com/WyattAu/ferro.git
cd ferro
docker compose up -d
# Open http://localhost:8080/ui/
```

### From Source
```bash
cargo build --release -p ferro-server -p ferro-cli
./target/release/ferro-server --data-dir /path/to/data --static-dir crates/web/dist
# Open http://localhost:8080/ui/
```

### With HTTPS (Caddy)
```bash
DOMAIN=files.example.com docker compose up -d
# Caddy handles TLS automatically via Let's Encrypt
```

## Features

### Core
- **WebDAV Class 1/2/3** — Full RFC 4918 compliance with LOCK/UNLOCK
- **Multiple storage backends** — Local filesystem, S3, GCS, Azure Blob (feature flags)
- **Content-addressable dedup** — Same file stored once, referenced by SHA-256 hash
- **Full-text search** — Tantivy-powered, auto-indexed on upload
- **WASM worker runtime** — Run custom logic on file events, sandboxed with fuel + memory limits

### Security
- **HTTP Basic Auth** — Simple `--admin-user`/`--admin-password` for personal deployments
- **OIDC authentication** — PKCE flow with Keycloak, Auth0, Google, etc.
- **Cedar authorization** — Policy-based access control (enterprise)
- **Security headers** — CSP, X-Frame-Options, HSTS, nosniff
- **Path traversal protection** — All file paths sanitized
- **Rate limiting** — 10,000 requests/minute per IP
- **WOPI protocol** — Collabora/WPS Office integration with HMAC-signed tokens

### Web UI
- **File browser** — Upload, download, create folders, drag-and-drop, breadcrumbs
- **Share links** — Password-protected, time-limited public URLs
- **Responsive** — Works on desktop and mobile
- **Authentication-aware** — Login flow, user display, sign-out

### Operations
- **Docker support** — Multi-stage build with bundled web UI
- **HTTPS via Caddy** — Automatic TLS with Let's Encrypt
- **Graceful shutdown** — SIGINT with connection draining
- **Structured logging** — JSON-compatible with request IDs
- **Health check** — `/.well-known/ferro` with subsystem status
- **Metrics** — `GET /metrics` for monitoring
- **Config file** — `ferro.toml` for persistent configuration
- **Audit logging** — All file operations logged with timestamps

## Configuration

All options via CLI flags, environment variables, or `ferro.toml`:

| Flag | Env | Default | Description |
|------|-----|---------|-------------|
| `--host` | `FERRO_HOST` | `0.0.0.0` | Bind address |
| `--port` | `FERRO_PORT` | `8080` | Bind port |
| `--data-dir` | `FERRO_DATA_DIR` | none | Persistent storage directory |
| `--static-dir` | `FERRO_STATIC_DIR` | none | Web UI static files |
| `--admin-user` | `FERRO_ADMIN_USER` | none | Admin username (enables auth) |
| `--admin-password` | `FERRO_ADMIN_PASSWORD` | none | Admin password |
| `--log-level` | `FERRO_LOG_LEVEL` | `info` | Log level |
| `--max-body-size` | `FERRO_MAX_BODY_SIZE` | `1GB` | Max upload size |
| `--config` | `FERRO_CONFIG` | none | Config file path |
| `--storage` | `FERRO_STORAGE` | `memory` | Storage backend |
| `--external-url` | `FERRO_EXTERNAL_URL` | none | Public URL (for OIDC) |
| `--wasm-enabled` | `FERRO_WASM_ENABLED` | `true` | Enable WASM workers |
| `--oidc-issuer` | `FERRO_OIDC_ISSUER` | none | OIDC issuer URL |
| `--oidc-client-id` | `FERRO_OIDC_CLIENT_ID` | none | OIDC client ID |

## WebDAV Clients

Ferro works with any WebDAV client:
- **macOS**: Finder → Connect to Server → `http://localhost:8080/`
- **Windows**: Map Network Drive → `http://localhost:8080/`
- **Linux**: Nautilus/GIO → `dav://localhost:8080/`
- **rclone**: `rclone lsf :webdav,url=http://localhost:8080/`
- **Cyberduck**: WebDAV connection at `http://localhost:8080/`

## Downloads

| File | Architecture | Size |
|------|-------------|------|
| `ferro-server-linux-gnu` | x86_64 Linux (glibc) | ~39 MB |
| `ferro-server-linux-musl` | x86_64 Linux (static) | ~42 MB |
| `ferro-server-macos` | x86_64 macOS | ~40 MB |
| `ferro-cli-linux-gnu` | x86_64 Linux (glibc) | ~8 MB |
| `ferro-cli-macos` | x86_64 macOS | ~9 MB |

## What's Next

- [ ] Multi-user support with user management
- [ ] Server-side rendering for the web UI
- [ ] Tauri desktop application
- [ ] File versioning with diff view
- [ ] Thumbnail generation for images
- [ ] Mobile app (iOS/Android via Tauri Mobile)
- [ ] WebRTC-based file sharing (peer-to-peer)
- [ ] LDAP/Active Directory authentication

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting. This release has:
- 0 known vulnerabilities in Ferro's own code
- 4 documented transitive advisories (no fix available upstream)
- OWASP Top 10 compliance checklist available in `docs/security/`

## Contributing

Bug reports, feature requests, and pull requests welcome. See the existing code style (220 tests, 0 clippy warnings) for contribution guidelines.

## License

[To be determined]
