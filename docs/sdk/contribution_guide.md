# Contribution Guide

## Getting Started

### Prerequisites

- Rust 1.92+ (edition 2024, pinned in `rust-toolchain.toml`)
- Git
- GitHub account
- Just (optional, for common tasks)
- Node.js 18+ (for web UI development)

### Setup

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/ferro.git
   cd ferro
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/WyattAu/ferro.git
   ```
4. Create a feature branch:
   ```bash
   git checkout -b feat/short-description
   ```

## Development

### Code Style

- Follow Rust conventions and existing patterns in the codebase
- Use `cargo fmt --all` for formatting (CI enforces this)
- Use `cargo clippy --all -- -D warnings` for linting (must pass with zero warnings)
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

### Testing

- Write unit tests for new functionality
- Write integration tests for complex features
- Place tests in `#[cfg(test)] mod tests` at the bottom of source files
- Use `#[tokio::test]` for async tests
- Ensure all tests pass before submitting

```bash
# Run all tests
cargo test --all

# Run tests for specific crate
cargo test -p ferro-auth

# Run tests with output
cargo test --all -- --nocapture
```

### Documentation

- Update README if needed
- Update API documentation
- Add examples for new features
- Run `cargo doc --all --no-deps` to verify documentation builds

## Pull Request Process

### Before Submitting

Run the full check suite:

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
cargo doc --all --no-deps
```

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

### PR Description

Include:

1. **What** changed and **why**
2. Link to related issues (e.g., `Fixes #123`)
3. Screenshots for UI changes
4. Migration steps if applicable
5. Breaking changes called out explicitly

### Review Process

1. Automated checks must pass
2. At least one review required
3. Changes requested must be addressed
4. PR must be approved before merge
5. Squash and merge on approval

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
| ferro-auth | `handlers` |
| ferro-crypto | `ring`, `fips` |
| ferro-fuse | `offline-cache` |
| ferro-client | `ffi` |

## Issue Reporting

### Bug Reports

- Clear description of the issue
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment details (OS, Rust version, etc.)
- Relevant logs or error messages

### Feature Requests

- Use case description
- Proposed solution
- Alternatives considered
- Any implementation ideas

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive experience for everyone.

### Our Standards

- Respectful communication
- Constructive feedback
- Inclusive language
- Professional behavior

### Enforcement

Project maintainers have the right to remove, edit, or reject comments, commits, code, wiki edits, and issues that are not aligned with this Code of Conduct.

## License

All contributions are licensed under AGPL-3.0-or-later. By contributing, you agree to this license.
