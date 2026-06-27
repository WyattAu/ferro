# Contributing to Ferro

Thank you for your interest in contributing. This guide covers everything you need.

## Development Setup

### Prerequisites

- Rust 1.92+ (edition 2024, pinned in `rust-toolchain.toml`)
- Just (optional, for common tasks)
- Node.js 18+ (for web UI development)
- Playwright (for E2E tests, optional)
- OpenSSL (for PostgreSQL support)

### Clone and Build

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/ferro.git
cd ferro

# Build everything
cargo build --all

# Build with all storage backends
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

### Run Tests

```bash
# All tests
cargo test --all

# Single crate
cargo test -p ferro-core

# With output
cargo test --all -- --nocapture
```

### Run Linting

```bash
# Format
cargo fmt --all

# Clippy (must pass with zero warnings)
cargo clippy --all -- -D warnings

# Doc generation
cargo doc --all --no-deps
```

### Nix Development

```bash
nix develop           # Full dev environment
nix develop .#web     # WASM build environment
nix develop .#desktop # Tauri desktop environment
```

## Project Structure

```
ferro/
├── crates/
│   ├── common/          # Core types and traits (publishable)
│   ├── core/            # Storage backends, search, WASM (publishable)
│   ├── dav/             # CalDAV/CardDAV/WebDAV (publishable)
│   ├── crypto/          # Cryptographic primitives (publishable)
│   ├── client/          # WebDAV client SDK (publishable)
│   ├── server/          # Main server binary
│   ├── web/             # Leptos web UI
│   ├── desktop/         # Tauri desktop app
│   ├── fuse/            # FUSE filesystem mount
│   ├── cli/             # Admin CLI
│   ├── admin/           # Leptos admin dashboard
│   ├── auth/            # Authentication and authorization
│   ├── webdav-handler/  # WebDAV XML builders
│   ├── server-activitypub/  # ActivityPub federation
│   ├── server-webrtc/       # WebRTC signaling
│   ├── server-wopi/         # WOPI protocol
│   ├── server-versioning/   # File versioning
│   ├── graphql/         # GraphQL API
│   ├── observability/   # Metrics and health
│   ├── crdt/            # CRDT collaboration
│   ├── cache/           # In-memory cache
│   ├── event-bus/       # Event system
│   ├── audit-log/       # Audit trail
│   ├── webhook/         # Outbound webhooks
│   ├── rate-limiter/    # Token bucket rate limiting
│   ├── wasm-host/       # WASM plugin hosting
│   ├── multi-tenant/    # Tenant isolation
│   ├── offline/         # Offline mode
│   ├── selective-sync/  # Selective folder sync
│   ├── ai/              # Semantic search and tagging
│   ├── backend-router/  # Backend routing
│   ├── consistent-hash/ # Consistent hashing
│   ├── distributed/     # Raft consensus
│   ├── sync-protocol/   # Client-server sync
│   ├── mount-nfs/       # NFS/SMB mount trait
│   ├── migrate/         # Data migration
│   └── benchmarks/      # Criterion benchmarks
├── deploy/              # Deployment configs
├── docs/                # mdBook documentation
└── web-e2e/             # Playwright E2E tests
```

## Code Style

### Formatting

- Run `cargo fmt --all` before committing
- CI enforces formatting

### Linting

- `cargo clippy --all -- -D warnings` must pass
- No allowed warnings

### Documentation

- All public items must have doc comments
- `cargo doc --all --no-deps` must produce zero warnings from our code

### Error Handling

- No `unwrap()` in production code paths
- Use `?` operator or `map_err` for error propagation
- Use `unwrap_or`, `unwrap_or_default`, `unwrap_or_else` for safe defaults
- Document any intentional `unwrap()` with a `// SAFETY:` comment

### Conventions

- Follow existing patterns in neighboring files
- Use workspace dependency versions from `[workspace.dependencies]` in the root `Cargo.toml`
- Prefer pure-Rust implementations over C bindings
- Validate all user input (paths, headers, bodies)
- Use constant-time comparison for secrets

## Testing

### Unit Tests

- All new features must include unit tests
- Place tests in `#[cfg(test)] mod tests` at the bottom of source files
- Use `#[tokio::test]` for async tests

### Integration Tests

- Place integration tests in `tests/` directories within each crate
- Test against real storage backends when possible
- Use `tempfile` for temporary directories

### E2E Tests

- Playwright-based E2E tests live in `web-e2e/`
- Test the full stack (server + web UI)
- Run with: `npm run test` in `web-e2e/`

### Coverage

- Aim for >80% branch coverage
- Critical paths must have >95% coverage
- Use `cargo-tarpaulin` for coverage reports

## Architecture Decisions

### ADR Process

Significant architectural decisions should be documented as Architecture Decision Records (ADRs):

1. Create a file in `docs/adr/` with the format `NNNN-title.md`
2. Use the template:

```markdown
# NNNN. Title

## Status

Proposed | Accepted | Deprecated | Superseded by [NNNN](NNNN-title.md)

## Context

What is the issue that we're seeing that motivates this decision?

## Decision

What is the change that we're proposing and/or doing?

## Consequences

What becomes easier or more difficult to do because of this change?
```

3. Reference the ADR in your PR description

## Pull Request Process

### Branch Naming

```
feat/short-description      # New features
fix/short-description       # Bug fixes
docs/short-description      # Documentation changes
refactor/short-description  # Code refactoring
test/short-description      # Test additions/fixes
chore/short-description     # Maintenance tasks
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add chunked upload API
fix: resolve path traversal in WebDAV
docs: update API reference
refactor: extract security headers module
test: add integration tests for PROPFIND
chore: update dependencies
ci: add release workflow
```

### Before Submitting

Run the full check suite:

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
cargo doc --all --no-deps
```

### PR Description

Include:

1. **What** changed and **why**
2. Link to related issues (e.g., `Fixes #123`)
3. Screenshots for UI changes
4. Migration steps if applicable
5. Breaking changes called out explicitly

### Review Process

1. Create a feature branch from `main`
2. Make your changes with tests
3. Update documentation if needed
4. Open a PR with a clear description
5. Address review feedback
6. Squash and merge on approval

## Adding Features

### New API Endpoints

1. Add handler in appropriate module under `crates/server/src/`
2. Add route in `crates/server/src/lib.rs`
3. Add tests (unit + integration)
4. Update API documentation in `docs/src/api/`

### New Crate

1. Add directory under `crates/`
2. Add to workspace `Cargo.toml` members list
3. Add `Cargo.toml` with publish metadata
4. Add `README.md`
5. Add to documentation in `docs/src/crates/`

### New Dependencies

1. Check for existing transitive dependency first
2. Prefer minimal dependencies
3. Avoid dependencies with known CVEs
4. Document the reason in PR description
5. Run `cargo audit` after adding

## Feature Flags

Each library crate uses feature flags for optional functionality:

| Crate | Flags |
|-------|-------|
| ferro-core | `sqlite`, `search`, `wasm`, `object_store`, `s3`, `gcs`, `azure`, `postgres` |
| ferro-dav | `handlers`, `persistence` |
| ferro-crypto | `ring`, `fips` |
| ferro-fuse | `offline-cache` |
| ferro-client | `ffi` |

## Security

- Report vulnerabilities per [SECURITY.md](https://github.com/WyattAu/ferro/blob/main/SECURITY.md)
- Never commit secrets or tokens
- Use constant-time comparison for secrets
- Validate all user input (paths, headers, bodies)
- Follow OWASP guidelines

## License

All contributions are licensed under AGPL-3.0-or-later. By contributing, you agree to this license.
