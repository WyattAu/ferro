# Contributing to Ferro

Thank you for your interest in contributing to Ferro! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before contributing.

## Getting Started

### Prerequisites
- Rust 1.92+
- Git
- GitHub account

### Setup
1. Fork the repository
2. Clone your fork
3. Create a feature branch
4. Make changes
5. Submit a pull request

## Development

### Code Style
- Follow Rust conventions
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Write documentation for public APIs

### Crate Architecture

The workspace is organized into domain-specific crates:

| Crate | Purpose |
|-------|---------|
| `common` | Shared types (DbHandle, AuditEntry, AuditLogTrait, error types) |
| `core` | Storage engine trait, CAS dedup, search engine, WASM runtime |
| `auth` | OIDC, simple auth, LDAP, WebAuthn, API keys, Cedar authorization |
| `crypto` | Cryptographic primitives (SHA, HMAC, password hashing) |
| `dav` | WebDAV/CalDAV/CardDAV protocol implementation (RFC 4918/4791/6352) |
| `scim` | SCIM 2.0 user/group provisioning |
| `server` | Main binary, handlers, middleware, startup |
| `server-config` | Configuration parsing, CLI, validation |
| `server-storage-ops` | Storage operations (upload, download, thumbnails, snapshots, dedup) |
| `server-security-middleware` | Auth middleware, CORS, rate limiting, canonical ApiError |
| `server-webdav-core` | WebDAV/CalDAV/CardDAV protocol handlers |
| `server-collaboration` | Real-time collaboration, comments, tags |
| `server-compliance` | WORM, retention, antivirus, DLP |
| `server-sharing` | Share links, favorites, federation, QR codes |
| `server-admin-api` | Server-side admin API handlers |
| `server-automation` | Workflow engine, smart collections, event triggers, OCR |
| `server-api` | REST API routes and handlers |
| `server-api-core` | File requests, API types |
| `server-content` | Content processing and transformation |
| `server-federation` | ActivityPub federation |
| `server-health` | Health check and readiness probes |
| `server-infra` | Infrastructure and deployment utilities |
| `server-integrations` | Third-party integrations |
| `server-plugins` | Plugin marketplace and management |
| `server-productivity` | Productivity features (calendars, contacts) |
| `server-resilience` | Circuit breaker, chaos engineering |
| `server-router` | Storage backend routing |
| `server-routes` | Route definitions |
| `server-security` | Security policies and enforcement |
| `server-slo` | Service level objectives |
| `server-state` | Application state management |
| `server-sync-handlers` | Cross-node sync handlers |
| `server-user-mgmt` | User management, remote wipe |
| `server-versioning` | File versioning and auto-versioning |
| `server-wopi` | WOPI protocol (Office Online) |
| `server-webrtc` | WebRTC signaling |
| `server-activitypub` | ActivityPub federation |
| `web` | Leptos WASM web frontend (14 themes, photo map, slideshow, EPUB, graph view, dual pane, block editor, audio player) |
| `admin` | Admin dashboard (Leptos), plugin marketplace |
| `desktop` | Tauri desktop application |
| `mobile` | Tauri v2 mobile bindings (iOS/Android) |
| `client` | Rust client SDK with C-FFI, remote wipe |
| `fuse` | FUSE filesystem mount |
| `mount-nfs` | NFS mount support |
| `crdt` | CRDT-based collaborative data structures |
| `sync-protocol` | Sync protocol for multi-node replication |
| `offline` | Offline-first sync and local queue |
| `selective-sync` | Selective file sync policies |
| `observability` | Metrics, health checks, Prometheus export |
| `event-bus` | Internal event bus for decoupled messaging |
| `rate-limiter` | Per-IP token-bucket rate limiting |
| `cache` | Caching layer for metadata and content |
| `health` | Health check and readiness probes |
| `audit-log` | Audit logging for file operations |
| `webhook` | Outgoing webhook delivery |
| `backend-router` | Storage backend routing and selection |
| `consistent-hash` | Consistent hashing for distributed nodes |
| `wasm-host` | WASM runtime host for file processing |
| `ai` | AI integration and smart features |
| `graphql` | GraphQL API layer |
| `distributed` | Distributed storage and consensus |
| `multi-tenant` | Multi-tenant isolation and management |
| `cli` | Admin CLI tool |
| `benchmarks` | Criterion benchmark suite |
| `migrate` | Data migration utilities |
| `webdav-handler` | WebDAV XML request/response parsing |
| `plugin` | Plugin system and runtime |
| `chaos` | Chaos engineering for testing |
| `circuit-breaker` | Circuit breaker pattern |
| `feature-flags` | Feature flag management |

**Key design principles:**
- Types are defined once in their canonical crate and re-exported everywhere
- `common::DbHandle` is the single source of truth (not 19 copies)
- `server-security-middleware::ApiError` is the canonical error type
- `common::audit::AuditEntry` and `AuditLogTrait` are shared across all crates
- Feature flags (pg, redis, ldap, s3, gcs, azure) are tested in CI matrix

### Testing
- Write unit tests for new functionality
- Write integration tests for complex features
- Ensure all tests pass before submitting

### Documentation
- Update README if needed
- Update API documentation
- Add examples for new features

## Pull Request Process

### Before Submitting
1. Run `cargo test`
2. Run `cargo clippy`
3. Run `cargo fmt`
4. Update documentation
5. Update changelog

### PR Template
```markdown
## Description
[Describe your changes]

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests pass

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Changelog updated
```

### Review Process
1. Automated checks must pass
2. At least one review required
3. Changes requested must be addressed
4. PR must be approved before merge

## Issue Reporting

### Bug Reports
- Clear description
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment details

### Feature Requests
- Use case description
- Proposed solution
- Alternatives considered

## Communication

### Channels
- GitHub Issues
- GitHub Discussions
- Discord
- Twitter/X

### Guidelines
- Be respectful
- Be inclusive
- Be constructive
- Be professional

## Recognition

### Contributors
- GitHub badges
- Annual awards
- Swag

### Maintainers
- Commit access
- Review authority
- Release management

## License

By contributing to Ferro, you agree that your contributions will be licensed under the MIT License.
