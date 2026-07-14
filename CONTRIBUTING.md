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
| `core` | Storage engine trait + InMemoryStorageEngine |
| `server` | Main binary, handlers, middleware, startup |
| `server-config` | Configuration parsing, CLI, validation |
| `server-storage-ops` | Storage operations (upload, download, thumbnails, snapshots) |
| `server-security-middleware` | Auth middleware, CORS, rate limiting, canonical ApiError |
| `server-webdav-core` | WebDAV/CalDAV/CardDAV protocol handlers |
| `server-collaboration` | Real-time collaboration, comments, tags |
| `server-compliance` | WORM, retention, antivirus, DLP |
| `server-sharing` | Share links, favorites, federation |

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
