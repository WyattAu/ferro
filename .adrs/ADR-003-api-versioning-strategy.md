# ADR-003: API Versioning Strategy

**Status:** Accepted
**Date:** 2026-07-12
**Deciders:** Wyatt (Sole developer)

## Context

Ferro exposes multiple API surfaces: WebDAV (RFC 4918), REST (`/api/*`), CalDAV/CardDAV (RFC 4791/6352), GraphQL, CLI, and a WASM worker API. The project is currently at v3.1.0-rc and has already undergone two major breaking changes (v1→v2: initial API, v2→v3: crate rewrite with new error format). Users and third-party integrators need a clear contract for what constitutes a breaking change and how API versions are communicated.

The solo-developer context means versioning discipline must be lightweight -- no multi-team coordination, no complex version negotiation protocols.

## Decision

### Semantic Versioning

Ferro follows [Semantic Versioning 2.0.0](https://semver.org/) for the crate workspace:

```
MAJOR.MINOR.PATCH
```

- **MAJOR**: Breaking changes to any public API surface (crate API, CLI interface, configuration format, HTTP API contracts)
- **MINOR**: New functionality that is backward-compatible (new endpoints, new CLI flags, new config options)
- **PATCH**: Backward-compatible bug fixes, dependency updates, documentation improvements

**Workspace version**: All 58 crates share a single workspace version (`3.1.0` in `Cargo.toml`). Individual crates do not have independent version numbers.

### Breaking Change Definition

A **breaking change** is any modification that requires users to change their code, configuration, or workflows:

| Category | Breaking Change Example | Non-Breaking Example |
|----------|------------------------|---------------------|
| REST API | Remove/rename endpoint, change response schema, change error format | Add new field to response, add new endpoint |
| WebDAV | Change default behavior of existing method, alter header requirements | Add support for new property |
| CalDAV | Alter sync semantics, change REPORT response format | Add new calendar property |
| GraphQL | Remove/modify field, change argument types | Add new field, add new type |
| CLI | Remove flag, rename flag, change default values | Add new flag, add new subcommand |
| Config | Rename key, change value semantics, remove option | Add new key, add new optional section |
| WASM | Change host function signatures, alter fuel semantics | Add new host function |
| Storage | Change on-disk format, alter CAS algorithm | Add new metadata field |

### API Versioning Scheme

**HTTP API:**
- Current version is implicit in the base URL: `/api/` (no version prefix)
- If a breaking change requires a parallel API, use URL path prefix: `/api/v2/`
- The old version continues to work for the deprecation window (see ADR-003)
- Maximum two versions served simultaneously (current + one previous)

**CalDAV/CardDAV:**
- Protocol-level versioning via `DAV:` header compliance (RFC 4918)
- No URL-based versioning (standard WebDAV convention)
- Extensions negotiated via `Accept-Extensions` / `DAV:` header

**GraphQL:**
- Schema version tracked in `crates/graphql/` via schema hash
- Breaking changes require a new schema version (field removal, type change)
- Non-breaking additions (new fields, new types) do not require version bump
- Client can query `__schema { version }` to check compatibility

**CLI:**
- CLI version matches workspace version (`ferro --version`)
- Deprecated flags emit warnings but continue working (see ADR-003)
- Breaking changes: remove flag or change semantics

**WASM Worker API:**
- Host function interface versioned via `ferro_worker_version()` host function
- Workers built against older host interfaces continue to work (backward-compatible host)
- Breaking host changes: new major version, old workers get 1-year grace period

### Release Process

1. Update `VERSION.md` with new version and changelog
2. Update `Cargo.toml` workspace version
3. Tag release: `git tag v3.2.0`
4. CI creates GitHub Release with binary artifacts
5. Docker image published: `ghcr.io/wyattau/ferro:v3.2.0`

### Pre-1.0 Exception

Since the project is pre-1.0 (currently v3.x but the API contract is still stabilizing), **minor version bumps may include breaking changes** if:
- The breaking change is clearly documented in release notes
- A migration guide is provided (per ADR-003)
- The change affects fewer than 3 existing endpoints/features

After v1.0.0, strict SemVer applies with no exceptions.

## Consequences

### Positive
- Clear, industry-standard versioning scheme that Rust developers already understand
- Workspace-level versioning simplifies release management (single version to bump)
- Pre-1.0 flexibility allows rapid iteration without MAJOR version inflation

### Negative
- Workspace versioning means a breaking change in one crate bumps the version for all 58 crates
- Some API surfaces (WebDAV protocol) don't follow SemVer natively
- WASM worker versioning is underdeveloped -- backward compatibility for WASM is hard to test

### Risks
- Pre-1.0 minor-version breaking changes may frustrate early adopters
- No automated enforcement of SemVer (cargo-semver-checks not yet integrated)
- GraphQL schema evolution is hard to track without tooling

## Alternatives Considered

### Separate Crate Versioning
- **Description:** Each of the 58 crates has independent version numbers
- **Pros:** Granular versioning, breaking changes in one crate don't bump others
- **Cons:** Massive release management overhead; dependency resolution becomes complex; solo developer cannot manage 58 independent release cycles
- **Why Rejected:** Unsustainable for a solo-developer project; workspace versioning is the Rust convention

### API Version in URL (e.g., /api/v1/, /api/v2/)
- **Description:** Every API version gets a URL prefix
- **Pros:** Explicit versioning, easy routing, no ambiguity
- **Cons:** Clutters URL space; WebDAV/CalDAV have no version prefix convention; multiple code paths to maintain
- **Why Rejected:** Only useful for REST API; other surfaces (WebDAV, GraphQL, CLI) don't benefit

### Header-Based Versioning (Accept: application/vnd.ferro.v2+json)
- **Description:** Clients specify version via Accept header
- **Pros:** Clean URLs, content negotiation
- **Cons:** Complex routing logic, hard to test, non-standard for WebDAV, most HTTP clients don't support custom Accept headers easily
- **Why Rejected:** Overengineered for a solo-developer project

## Related ADRs
- [ADR-002](ADR-002-deprecation-policy.md) -- Deprecation Policy (defines when breaking changes can ship)
- [ADR-004](ADR-004-security-review-process.md) -- Security Review Process (security fixes may bypass versioning)

## References
- Semantic Versioning 2.0.0: https://semver.org/
- RFC 4918: HTTP Extensions for Web Distributed Authoring and Versioning (WebDAV)
- RFC 4791: Calendaring Extensions to WebDAV (CalDAV)
- RFC 6352: vCard Extensions to WebDAV (CardDAV)
- cargo-semver-checks: https://github.com/obi1kenobi/cargo-semver-checks
