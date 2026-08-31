# Ferro Roadmap

**Version:** 17.0 | **Date:** 2026-08-30 | **Status:** Deployment + Cross-platform mount

---

## Current State

| Metric | Value |
|--------|-------|
| Crates | 73 |
| Tests | 925+ (core library) / 2500+ (full workspace) |
| CI Workflows | 13 |
| Pre-commit | 5-stage (fmt, clippy, secret scan, TODO scan, tests) |
| MSRV | 1.92 |
| Toolchain | 1.95.0 |
| License | AGPL-3.0-or-later |
| Deployment | TrueNAS Docker (ghcr.io/wyattau/ferro:latest) |
| Web URL | https://ferro.wyattau.com |
| OIDC | Keycloak (company-realm, client: ferro) |

## Completed (v16.0 Audit Cycle)

### Phase 1: Code Quality
- Resolved 56 clippy warnings in ferro-ui (unused vars, missing Default impls, dead code)
- Fixed cargo-deny: advisory ignores, ferro-ui license field
- Resolved rustfmt.toml / .rustfmt.toml max_width conflict (both 120)
- Pre-commit hook optimized: cargo metadata resolution, single-pass secret scan, --all-features
- 925 tests pass across 12 critical library crates, 0 failures

### Phase 2: CI/CD Hardening
- Added persist-credentials: false to all 54 checkout steps across 13 workflows
- Added permissions block to sanitizers.yml (was missing entirely)
- Added timeout-minutes: 60 to all sanitizer jobs
- Fixed shell injection in release.yml (env var instead of direct interpolation)
- Pinned unpinned actions: android-actions/setup-android, grafana/k6-action
- Replaced curl-pipe-sh in formal_verification.yml with pinned elan version
- Added Swatinem/rust-cache to sanitizers.yml

### Phase 3: GUI/UX
- Added glass morphism tokens and classes (Spatial Materialism)
- Replaced rigid border-radius with organic asymmetric values (Amoebic UI)
- Added focus-visible styles to all interactive elements
- Increased touch targets to 44px minimum (WCAG 2.5.8)
- Added skip navigation link with #main-content anchor
- Added Escape key handler to Dialog component
- Fixed format_size bug: UI copies now handle GB/TB (was truncating at MB)

### Phase 4: Documentation
- README.md rewritten: 541 to ~180 lines, no emoji, updated to 73 crates
- Documentation site updated: crate count corrected from 46 to 73

### Phase 5: Version Control
- Commit e5f3dc9: CI/CD hardening, UI accessibility, documentation overhaul
- Commit ed8f8c0: Documentation site crate count update
- CI pipelines: Deploy Documentation passed, all failures pre-existing

### Phase 7: Functionality Audit
- Identified format_size triplication (fixed in ferro-ui)
- Identified build_client triple duplication (documented, requires shared crate extraction)
- Identified OIDC middleware duplication (documented, server + server-security-middleware)
- Identified comments/tags overlap between server-sharing and server-collaboration (documented)
- Identified frontend loading/error/empty state boilerplate (documented, needs use_data_loader hook)

---

## Completed (v17.0 Deployment + Security)

### OIDC Flow
- Fixed FERRO_EXTERNAL_URL for correct OIDC redirect_uri
- Added OIDC refresh_token storage and end_session_url
- Server callback returns refresh_token and logout_url
- Web client stores refresh token in localStorage
- Logout redirects to Keycloak end_session_endpoint for front-channel logout
- Created Keycloak client `ferro` (secret: [REDACTED - stored in Keycloak], audience: account)

### Deployment
- Deployed on TrueNAS Docker (ghcr.io/wyattau/ferro:latest)
- Cloudflare tunnel + DNS: ferro.wyattau.com → Traefik → Ferro
- Dockerfile: use pre-built WASM dist via COPY (not trunk build)
- Deployed crates/web/ dist (pure web, no Tauri dependency)
- Fixed Tauri fallback to HTTP in file_browser.rs

### Security
- 6 security headers via Traefik middleware (CSP, HSTS, X-Frame-Options, etc.)
- HTML meta tags (OG, Twitter Card, JSON-LD, canonical, accessibility headings)
- Fixed Cedar policy loading (load_policies instead of add_policy)
- Cedar policy: permit all (open for testing)

### Infrastructure
- Automated backups: SQLite backup script, daily at 3am, 7-day retention
- Headscale: both nodes online (cachyos + truenas)
- crawlkit audit: 43 findings (16 warnings addressed)

### Bugs Fixed
- GLIBC mismatch (binary built on glibc 2.43, container had 2.36)
- Socket-proxy EIR image (empty rootfs, switched to tecnativa/docker-socket-proxy)
- YAML duplicate key in dynamic.yml
- Missing CMD in Docker image (entrypoint.sh with no args)
- CORS + OIDC conflict (FERRO_CORS_ORIGINS)

---

## Next: v18.0 Cross-Platform Mount

### P0 — Critical

| Item | Scope | Effort | Status |
|------|-------|--------|--------|
| Cross-platform FUSE mount | Migrate from fuse3 (Linux-only) to fuser (Linux/macOS/Windows) | 3-5 days | Done |
| macOS mount support | macFUSE or FUSE-T integration via fuser crate | 1-2 days | Done (via fuser) |
| Windows mount support | WinFSP integration via fuser crate | 1-2 days | Done (via fuser) |
| External WebDAV via Headscale | ACL rules, mount documentation | 0.5 days | Done |
| External WebDAV via Cloudflare | Tunnel ingress for /dav/ path | 0.5 days | Done |

### P1 — High

| Item | Scope | Effort | Status |
|------|-------|--------|--------|
| Native OS mount integration | Automount (launchd/fstab/systemd), Finder/Explorer sidebar | 2 days | Done |
| Token refresh interceptor | Proactive token refresh in web client before expiry | 1 day | Done |
| Desktop client OIDC | PKCE flow in Tauri for desktop app | 3-5 days | Done |
| Lock down Cedar policy | Replace permit-all with per-user rules from Keycloak groups | 1 day | Done |
| Crawlkit re-crawl | Verify security headers + meta tags fix all warnings | 0.5 days | Done |

### P2 — Medium

| Item | Scope | Effort | Status |
|------|-------|--------|--------|
| Extract shared HTTP client | Create ferro-http-client crate, consolidate gui.rs + mobile.rs | 1 day | Done (uses common::http_client) |
| Extract shared MobileError | Single error enum in common or shared crate | 0.5 days | Pending |
| Delete duplicate OIDC middleware | Remove server/src/auth/oidc.rs, use server-security-middleware re-export | 0.5 days | Done (server-security-middleware re-exports ferro-auth) |
| Consolidate comments/tags | Make server-collaboration the single source of truth | 1 day | Pending |
| Frontend use_data_loader hook | Extract loading/error/empty state pattern | 1 day | Pending |

### P3 — Low

| Item | Scope | Effort | Status |
|------|-------|--------|--------|
| Responsive sidebar collapse | Mobile hamburger menu, sidebar toggle | 2 days | Done |
| ARIA tab panels | Admin/Settings: role="tablist", role="tab", role="tabpanel" | 1 day | Done |
| Entrance/exit animations | Modals, toasts, list items | 2 days | Pending |
| Micro-interactions | Button press scale, card hover elevation | 1 day | Pending |
| Spring-based transitions | Organic motion curves for Amoebic UI feel | 1 day | Pending |

---

## v19.0 Feature Expansion

### Storage & Performance
- Erasure coding for distributed storage
- Geo-replication with conflict resolution
- Block-level delta sync (currently file-level)
- WebSocket-based real-time file change notifications

### Collaboration
- Real-time CRDT co-editing (currently stubbed)
- Comment threads with @mentions
- Task assignment and due dates

### Security
- End-to-end encryption (AES-256-GCM per-file keys)
- Hardware key support (YubiKey, Titan)
- Audit log tamper-evident chaining verification
- SOC 2 Type II certification preparation

### Infrastructure
- Kubernetes Helm chart
- Terraform modules for AWS/GCP/Azure
- Prometheus/Grafana dashboard templates
- SLO/SLI error budget automation

---

## v20.0 Scale & Distribution

### Multi-Node
- Raft consensus for metadata
- Consistent hashing for data placement
- Automatic shard rebalancing
- Cross-datacenter replication

### Federation
- ActivityPub federation (already scaffolded in server-activitypub)
- Cross-instance file sharing
- Federated calendar/contact sync

### Enterprise
- SAML 2.0 authentication
- SCIM 2.0 provisioning (already in ferro-scim)
- LDAP group sync
- Custom branding/white-label

---

## Architecture Decision Records

| ADR | Status | Date |
|-----|--------|------|
| ADR-001: GUI Rewrite (Leptos) | Accepted | 2026-07-14 |
| ADR-002: ServerState Trait Abstraction | Accepted | 2026-07-14 |
| ADR-003: Crate Decomposition Strategy | Accepted | 2026-07-14 |
| ADR-004: Pre-commit Hook Design | Accepted | 2026-07-23 |
| ADR-005: CI/CD Security Hardening | Accepted | 2026-07-23 |
| ADR-006: Spatial Materialism + Amoebic UI | Accepted | 2026-07-23 |
| ADR-007: Cross-platform FUSE via fuser | Accepted | 2026-08-30 |

---

**End of Roadmap v17.0**
