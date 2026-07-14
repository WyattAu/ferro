# ADR-002: Deprecation Policy

**Status:** Accepted
**Date:** 2026-07-12
**Deciders:** Wyatt (Sole developer)

## Context

Ferro has a growing public surface area: WebDAV endpoints (RFC 4918), REST API (`/api/*`), CalDAV/CardDAV, GraphQL, CLI flags, configuration file (`ferro.toml`), and WASM worker APIs. With 58 crates and active development, APIs that were designed for earlier iterations may become obsolete or need breaking changes. Without a formal deprecation policy, users have no predictability for when their integrations will break, and the developer has no discipline around maintaining backward compatibility.

The project is pre-1.0 (currently v3.1.0-rc) but has real users syncing real data. Breaking changes need a structured migration path, not silent breakage.

## Decision

### Deprecation Cycle

All public APIs follow a **6-month deprecation cycle**:

| Phase | Duration | Action |
|-------|----------|--------|
| 1. Announcement | Month 0 | Mark API as deprecated with `#[deprecated]` (Rust), HTTP `Deprecation` header, CLI `--deprecated` warning, documentation update |
| 2. Migration Window | Months 1-5 | API remains fully functional; warnings emitted on every use; migration guide published |
| 3. Removal | Month 6 | API removed in next major/minor release; removed from OpenAPI spec, CLI help, docs |

**Exceptions (immediate removal allowed):**
- Security vulnerabilities requiring API surface reduction
- APIs that were never documented or publicly released
- APIs marked experimental (prefixed with `x-` or `unstable_`)

### Migration Guides

Every deprecated API must include a migration guide in `docs/migrations/`:

```
docs/migrations/
  v3.2-migrate-graphql-v1.md
  v3.3-migrate-webdav-locks.md
  ...
```

Migration guide template:
```markdown
# Migrating from [Old API] to [New API]

## What Changed
[Description of the breaking change]

## Before
[Old usage example]

## After
[New usage example]

## Timeline
- Deprecated: [date]
- Removed: [date]
```

### Compatibility Matrix

Maintain a public compatibility matrix in `docs/COMPATIBILITY.md`:

| API | Stable Since | Last Breaking Change | Next Planned Breaking Change |
|-----|-------------|---------------------|------------------------------|
| WebDAV (RFC 4918) | v1.0.0 | v3.0.0 (crate rewrite) | None planned |
| REST /api/* | v2.0.0 | v3.0.0 (error format) | None planned |
| CalDAV/CardDAV | v3.0.0 | v3.0.0 (initial) | None planned |
| GraphQL | v3.0.0 | v3.0.0 (initial) | TBD |
| CLI flags | v1.0.0 | v3.0.0 (config file) | None planned |
| ferro.toml | v3.0.0 | v3.0.0 (initial) | None planned |
| WASM worker API | v3.0.0 | v3.0.0 (initial) | None planned |

### Deprecation Annotation Standards

**Rust code:**
```rust
#[deprecated(since = "3.2.0", note = "Use `new_api()` instead. See docs/migrations/v3.2-migrate.md")]
pub fn old_api() { }
```

**HTTP responses:**
```
Deprecation: true
Sunset: Sat, 12 Jan 2027 00:00:00 GMT
Link: <docs/migrations/v3.2-migrate.md>; rel="deprecation"
```

**CLI:**
```
[DEPRECATED] --old-flag is deprecated since v3.2.0 and will be removed in v4.0.0. Use --new-flag instead.
```

### Versioning Alignment

Deprecation cycles align with the release train:
- Deprecations are announced in the minor release (e.g., v3.2.0)
- Removals happen in the next major or minor release after 6 months (e.g., v4.0.0 or v3.8.0)
- Pre-1.0: deprecation cycle is shortened to **3 months** for APIs introduced before v1.0

## Consequences

### Positive
- Users have predictable timelines for migration (6 months minimum)
- `#[deprecated]` compiler warnings catch usage at build time for Rust consumers
- HTTP `Deprecation` + `Sunset` headers allow programmatic detection
- Migration guides reduce user friction and support burden

### Negative
- Deprecated code must be maintained for 6 months (test coverage, CI passing)
- Solo developer must remember to remove deprecated APIs after the window
- Documentation overhead for migration guides

### Risks
- If the project ships very frequently, 6-month windows may overlap, creating multiple deprecated APIs simultaneously
- HTTP clients may not respect `Deprecation`/`Sunset` headers, making silent breakage possible when removed
- WASM worker API deprecation is hard to enforce (no compile-time warnings for WASM consumers)

## Alternatives Considered

### No Deprecation Period (Just Break)
- **Description:** Remove APIs immediately when replaced
- **Pros:** Zero maintenance overhead, clean codebase
- **Cons:** Breaks user integrations without warning; unacceptable for a data-sync server where trust is critical
- **Why Rejected:** Users sync critical data through these APIs; silent breakage could cause data loss

### Semantic Versioning Strict Mode
- **Description:** Never break public API until v2.0.0 (SemVer strict)
- **Pros:** Maximum stability for consumers
- **Cons:** Pre-1.0 project; SemVer strict would lock in early design mistakes forever
- **Why Rejected:** Project is still iterating on core APIs; strict SemVer is premature

### Permanent Deprecation (Never Remove)
- **Description:** Deprecate but keep old APIs forever for compatibility
- **Pros:** Zero breakage risk
- **Cons:** Codebase bloat, testing burden grows unboundedly, security surface expands
- **Why Rejected:** With 58 crates, maintaining dead APIs is unsustainable for a solo developer

## Related ADRs
- [ADR-003](ADR-003-api-versioning-strategy.md) -- API Versioning Strategy (defines what constitutes a breaking change)

## References
- RFC 8594: The Sunset HTTP Header Field
- Semantic Versioning 2.0.0: https://semver.org/
- Rust `#[deprecated]` attribute: https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute
