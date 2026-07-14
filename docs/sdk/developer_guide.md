# Developer Guide

## Getting Started

### Prerequisites

- Rust 1.92+ (edition 2024, pinned in `rust-toolchain.toml`)
- SQLite 3.35+ (for persistence features)
- OpenSSL (for PostgreSQL support)

### Setup

```bash
# Clone the repository
git clone https://github.com/WyattAu/ferro.git
cd ferro

# Build all crates
cargo build --all

# Build with specific features
cargo build --features s3,gcs,azure

# Build release binary
cargo build --release --bin ferro-server
```

### Run the Server

```bash
# Quick start (in-memory storage)
cargo run --bin ferro-server

# With persistent data
cargo run --bin ferro-server -- --data-dir /tmp/ferro-data

# With all features
cargo run --bin ferro-server -- --data-dir /tmp/ferro-data --storage local:/tmp/ferro-files
```

## Architecture

### Crate Structure

```
ferro/
├── crates/
│   ├── common/          # Core types and traits
│   ├── auth/            # Authentication and authorization
│   ├── dav/             # CalDAV/CardDAV/WebDAV
│   ├── core/            # Storage backends, search, WASM
│   ├── crypto/          # Cryptographic primitives
│   ├── server/          # Main server binary
│   ├── web/             # Leptos web UI
│   ├── desktop/         # Tauri desktop app
│   ├── fuse/            # FUSE filesystem mount
│   ├── cli/             # Admin CLI
│   ├── admin/           # Leptos admin dashboard
│   └── ...              # Additional crates
```

### Key Concepts

#### Authentication

- **Simple Auth**: HTTP Basic authentication with bcrypt password hashing
- **OIDC**: OpenID Connect with PKCE flow for token-based authentication
- **Cedar**: Fine-grained policy-based authorization
- **API Keys**: Key-based authentication for programmatic access
- **WebAuthn**: Passwordless authentication support
- **TOTP**: Two-factor authentication

#### DAV Protocol

- **CalDAV**: Calendar operations (RFC 4791)
- **CardDAV**: Contact operations (RFC 6352)
- **WebDAV**: File operations (RFC 4918)

#### Storage

- **In-Memory**: Default storage for development
- **Local Filesystem**: Persistent local storage
- **S3/GCS/Azure**: Cloud storage backends
- **Content-Addressable Storage**: SHA-256 deduplication

## Development Workflow

### Adding a New Feature

1. Create a feature branch:
   ```bash
   git checkout -b feat/short-description
   ```

2. Implement the feature with tests

3. Run the full check suite:
   ```bash
   cargo fmt --all
   cargo clippy --all -- -D warnings
   cargo test --all
   cargo doc --all --no-deps
   ```

4. Submit a pull request

### Running Tests

```bash
# All tests
cargo test --all

# Single crate
cargo test -p ferro-auth

# With output
cargo test --all -- --nocapture

# Integration tests
cargo test --test integration
```

### Code Style

- Follow Rust conventions
- Use `cargo fmt --all` for formatting
- Use `cargo clippy --all -- -D warnings` for linting
- Write documentation for all public items
- No `unwrap()` in production code paths (use `?` or `map_err`)

### Error Handling

```rust
// Good: Use ? operator
fn process() -> Result<(), FerroError> {
    let data = read_file().map_err(|e| FerroError::Internal(e.to_string()))?;
    Ok(())
}

// Good: Use unwrap_or for defaults
let value = config.get("key").unwrap_or("default");

// Bad: Don't use unwrap in production
let value = config.get("key").unwrap(); // Avoid this!
```

## Debugging

### Logging

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Enable trace logging for specific modules
RUST_LOG=ferro_auth=trace,ferro_dav=debug cargo run
```

### Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph
```

### Memory Analysis

```bash
# Install cargo-valgrind
cargo install cargo-valgrind

# Run with valgrind
cargo valgrind
```

## Feature Flags

### ferro-auth

| Flag | Description |
|------|-------------|
| `handlers` | Enable Axum middleware handlers (default) |

### ferro-dav

| Flag | Description |
|------|-------------|
| `handlers` | Enable CalDAV/CardDAV Axum handlers (default) |
| `persistence` | Enable SQLite persistence for calendar/address book stores |

### ferro-core

| Flag | Description |
|------|-------------|
| `sqlite` | Enable SQLite storage backend |
| `search` | Enable Tantivy full-text search |
| `wasm` | Enable WASM worker runtime |
| `object_store` | Enable object store abstraction |
| `s3` | Enable Amazon S3 storage backend |
| `gcs` | Enable Google Cloud Storage backend |
| `azure` | Enable Azure Blob Storage backend |
| `postgres` | Enable PostgreSQL metadata backend |

## Contributing

### Pull Request Process

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Update documentation if needed
5. Run the full check suite
6. Submit a pull request with a clear description

### Code Review

- All PRs require review
- Tests must pass
- Documentation must be updated
- Changelog must be updated for user-facing changes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add chunked upload API
fix: resolve path traversal in WebDAV
docs: update API reference
refactor: extract security headers module
test: add integration tests for PROPFIND
chore: update dependencies
```

## Architecture Decision Records

Significant architectural decisions should be documented as ADRs:

1. Create a file in `docs/adr/` with format `NNNN-title.md`
2. Use the template in `CONTRIBUTING.md`
3. Reference the ADR in your PR description

## Security

- Never commit secrets or tokens
- Use constant-time comparison for secrets
- Validate all user input (paths, headers, bodies)
- Follow OWASP guidelines
- Report vulnerabilities per `SECURITY.md`
