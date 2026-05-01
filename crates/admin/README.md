# Ferro Admin

Standalone administration panel for Ferro file servers. Connects to any Ferro server via its REST API.

## Building

Requires [Rust](https://rustup.rs/) and [trunk](https://trunkrs.dev/):

```bash
# Install trunk (WASM build tool)
cargo install trunk

# Build for development
trunk build --release

# The output will be in dist/
```

Alternatively, using [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
# Install wasm-pack
cargo install wasm-pack

# Build
wasm-pack build --target web
```

## Running Locally

```bash
trunk serve --open
```

This opens the admin panel in your browser at `http://localhost:8080`.

## Configuration

The admin panel does not embed a server. It connects to a running Ferro server:

1. Navigate to the admin UI
2. Enter your Ferro server URL (e.g., `https://ferro.example.com`)
3. Enter your admin token or credentials
4. Click "Connect"

Credentials are stored in your browser's localStorage and persist across sessions.

## Pages

### Dashboard
Overview of server health showing total files, storage usage, uptime, server version, and health status. Includes a storage usage chart and recent activity feed from the audit log.

### Users
User management table with create/delete functionality. Supports role assignment (admin, editor, viewer) with role-based badge indicators. Includes search/filter by username.

### Storage
Storage statistics including total usage, file count, and directory count. Displays file size distribution chart, files-by-type pie chart, and a table of recent files with paths and sizes.

### Monitoring
Server metrics including uptime, version, authentication type, and storage backend. Displays server feature flags, raw Prometheus metrics output, and an optional external Grafana dashboard link.

### Settings
Server information display and configuration panels for authentication (session timeout), CORS (allowed origins), and rate limiting (max requests per minute, max file size). Note: some settings require server restart.

### Federation
ActivityPub federation management showing followers, following, inbox, and outbox. Displays node information (software name and version). Tabbed interface for viewing each federation category.

### Webhooks
Webhook management with create/delete/test functionality. Supports event selection (file events, share events, user events) and optional signing secrets.

### Audit Log
Paginated audit log viewer with filtering by user and action type. Supports CSV export. Displays timestamp, user, action type, resource path, and status for each entry.

## Connecting to a Server

The admin panel authenticates using a Bearer token sent with each API request. The token is stored in localStorage and sent with the `Authorization: Bearer <token>` header.

For servers using basic auth, the token should be a Base64-encoded `username:password` string, or the server's admin token.
