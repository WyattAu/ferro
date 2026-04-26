# Security Policy

## Reporting Vulnerabilities

If you discover a security vulnerability in Ferro, please report it responsibly:

1. **Email**: security@ferro.dev (or open a private GitHub Security Advisory)
2. **Do not** open a public issue for security vulnerabilities
3. Include: description, steps to reproduce, potential impact, and any suggested fix

## Known Vulnerabilities

### RUSTSEC-2023-0071 — RSA Marvin Attack (Medium)

- **Crate**: `rsa` 0.9.10 (transitive via `sqlx-mysql`)
- **Issue**: Timing side-channel in RSA key decryption ("Marvin Attack")
- **Impact**: Limited — only affects MySQL connections using TLS
- **Status**: No fix available upstream
- **Workaround**: Use SQLite or PostgreSQL instead of MySQL

### RUSTSEC-2025-0134 — rustls-pemfile Unmaintained (Medium)

- **Crate**: `rustls-pemfile` 2.2.0 (transitive via `reqwest`/`rustls`)
- **Issue**: Crate is unmaintained; no future security fixes
- **Impact**: Low — only affects TLS certificate loading
- **Status**: No fix available; upstream recommends migrating to `rustls-pki-types`
- **Workaround**: Monitor for reqwest updates that drop the dependency

### RUSTSEC-2026-0002 — LRU Stacked Borrows Violation (Medium)

- **Crate**: `lru` 0.12.5 (transitive via `object_store`)
- **Issue**: `IterMut` violates Stacked Borrows by invalidating internal pointer
- **Impact**: Low — potential memory unsafety under specific iterator patterns
- **Status**: No fix available upstream
- **Workaround**: No direct workaround; unlikely to be triggered in normal operation

### RUSTSEC-2026-0097 — Rand Logger Unsoundness (Medium)

- **Crate**: `rand` 0.7.3 (transitive via older dependency)
- **Issue**: Unsound when using `rand::rng()` with a custom logger
- **Impact**: Very low — only triggered by custom logger implementations
- **Status**: No fix available (version-specific)
- **Workaround**: Ferro does not use custom loggers with rand

### Tauri/GTK Ecosystem Advisories (Low)

Multiple advisories exist in the GTK/Tauri dependency chain (`atk`, `gdk`, `glib`, `gtk`, etc.).

- **Impact**: Only affects builds with `--features tauri` (desktop GUI mode)
- **Status**: These crates are only pulled in when the optional `tauri` feature is enabled
- **Workaround**: The default server/CLI builds do not include these dependencies
- **Mitigation**: CI pipeline excludes the `tauri` feature from clippy and test jobs

### Dependency Audit

Run `cargo audit` to check for known vulnerabilities:

```sh
cargo audit
```

As of the latest release:
- **4 advisories with no fix**: `rsa`, `rustls-pemfile`, `lru`, `rand` (all transitive, limited impact)
- **Multiple advisories in Tauri/GTK**: only with `--features tauri` (not in default builds)
- **0 advisories in Ferro's own code**

## Security Features

| Feature | Status | Details |
|---------|--------|---------|
| OIDC Authentication | Working | PKCE flow with Keycloak, Auth0, Google, etc. |
| Cedar Authorization | Working | Policy-based access control |
| Rate Limiting | Working | Per-IP sliding window (10k req/min) |
| CORS | Working | Conditional CORS for cross-origin requests |
| Body Size Limits | Working | Configurable max request body size |
| WOPI Token Security | Working | HMAC-SHA256 signed tokens with configurable secret |
| WASM Sandboxing | Working | Fuel limits, memory limits, timeout enforcement |

## Security Configuration

### Production Checklist

- [ ] Set `--wopi-token-secret` to a strong random value (min 32 chars)
- [ ] Set `--external-url` to your public URL (required for OIDC)
- [ ] Configure OIDC with a trusted provider
- [ ] Set up Cedar authorization policies
- [ ] Use HTTPS (via reverse proxy)
- [ ] Set `--max-body-size` to reasonable limit
- [ ] Use `--data-dir` for persistent storage
- [ ] Don't expose the server directly; use a reverse proxy (nginx/Caddy)
- [ ] Enable rate limiting (enabled by default)
