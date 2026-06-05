# Contributing to Ferro

Thank you for your interest in contributing. This guide covers everything you need.

## Quick Start

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/ferro.git`
3. Build: `cargo build --all`
4. Test: `cargo test --all`
5. Check: `cargo clippy --all -- -D warnings`

## Development Prerequisites

- Rust 1.92+ (edition 2024)
- Just (optional, for common tasks)
- Node.js 18+ (for web UI development)
- Playwright (for E2E tests, optional)

## Project Structure

```
ferro/
├── crates/
│   ├── common/    # Core types and traits (publishable)
│   ├── core/      # Storage backends, search, WASM (publishable)
│   ├── dav/       # CalDAV/CardDAV/WebDAV (publishable)
│   ├── crypto/    # Cryptographic primitives (publishable)
│   ├── client/    # WebDAV client SDK (publishable)
│   ├── server/    # Main server binary
│   ├── web/       # Leptos web UI
│   ├── desktop/   # Tauri desktop app
│   ├── fuse/      # FUSE filesystem mount
│   ├── cli/       # Admin CLI
│   ├── admin/     # Leptos admin dashboard
│   ├── auth/      # Authentication and authorization
│   ├── webdav-handler/ # WebDAV XML builders
│   ├── server-activitypub/ # ActivityPub federation
│   ├── server-webrtc/     # WebRTC signaling
│   ├── server-wopi/       # WOPI protocol
│   ├── server-versioning/ # File versioning
│   ├── graphql/   # GraphQL API
│   ├── observability/ # Metrics and health
│   └── benchmarks/ # Criterion benchmarks
├── deploy/        # Deployment configs
├── docs/          # mdBook documentation
└── web-e2e/       # Playwright E2E tests
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
- Document any intentional `unwrap()` with a SAFETY comment

### Testing
- All new features must include tests
- Aim for >80% branch coverage
- Critical paths must have >95% coverage
- Integration tests in `tests/`
- E2E tests in `web-e2e/`

## Commit Messages

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

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes with tests
3. Run full check suite:
   ```bash
   cargo fmt --all
   cargo clippy --all -- -D warnings
   cargo test --all
   cargo doc --all --no-deps
   ```
4. Update documentation if needed
5. Open a PR with a clear description
6. Address review feedback
7. Squash and merge on approval

## Adding Features

### New API Endpoints
1. Add handler in appropriate module under `crates/server/src/`
2. Add route in `crates/server/src/lib.rs`
3. Add tests (unit + integration)
4. Update API documentation in `docs/src/api/`

### New Crate
1. Add directory under `crates/`
2. Add to workspace `Cargo.toml`
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
| ferro-core | sqlite, search, wasm, object_store, s3, gcs, azure, postgres |
| ferro-dav | handlers, persistence |
| ferro-crypto | ring, fips |
| ferro-fuse | offline-cache |
| ferro-client | ffi |

## Security

- Report vulnerabilities per SECURITY.md
- Never commit secrets or tokens
- Use constant-time comparison for secrets
- Validate all user input (paths, headers, bodies)
- Follow OWASP guidelines

## License

All contributions are licensed under AGPL-3.0-or-later. By contributing, you agree to this license.
