# Ferro Roadmap

**Version:** 16.0 | **Date:** 2026-07-23 | **Status:** Audit cycle complete

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

## Next: v17.0 Production Hardening

### P0 — Critical

| Item | Scope | Effort |
|------|-------|--------|
| Fix pre-existing CI failures | Lean4 proof syntax, benchmark flakiness, miri toolchain, audit test | 2-3 days |
| Extract shared HTTP client | Create ferro-http-client crate, consolidate gui.rs + mobile.rs | 1 day |
| Extract shared MobileError | Single error enum in common or shared crate | 0.5 days |
| Delete duplicate OIDC middleware | Remove server/src/auth/oidc.rs, use server-security-middleware re-export | 0.5 days |
| Consolidate comments/tags | Make server-collaboration the single source of truth, remove from server-sharing | 1 day |

### P1 — High

| Item | Scope | Effort |
|------|-------|--------|
| Frontend use_data_loader hook | Extract loading/error/empty state pattern into reusable hook | 1 day |
| Frontend error states | Add error display to all domain components (currently only file_browser shows errors) | 1 day |
| Frontend loading spinners | Fix unused _loading signals, render spinners in all components | 0.5 days |
| Dialog focus trap | Implement Tab cycling within dialog, auto-focus first element | 1 day |
| Responsive sidebar collapse | Mobile hamburger menu, sidebar toggle on small screens | 2 days |
| ARIA tab panels | Admin/Settings: role="tablist", role="tab", role="tabpanel" | 1 day |

### P2 — Medium

| Item | Scope | Effort |
|------|-------|--------|
| Reduce unwrap() in production code | notes.rs (109), tcp_transport.rs (82), backup.rs (70) | 3 days |
| Add xl/2xl responsive utilities | Extend CSS breakpoint system | 0.5 days |
| Deduplicate ShellLayout vs Shell | Remove unused shell.rs, use routes/mod.rs ShellLayout only | 0.5 days |
| Add entrance/exit animations | Modals, toasts, list items | 2 days |
| Micro-interactions | Button press scale, card hover elevation | 1 day |
| Spring-based transitions | Organic motion curves for Amoebic UI feel | 1 day |

### P3 — Low

| Item | Scope | Effort |
|------|-------|--------|
| cargo-llvm-cov config file | .cargo/config.toml alias for local coverage | 0.5 days |
| Codecov config | codecov.yml with coverage thresholds | 0.5 days |
| Status badges | Add CI/security/release badges to README | 0.5 days |
| Composite action for cargo install | Reduce duplication of cargo-deny, cargo-fuzz, etc. across workflows | 1 day |

---

## v18.0 Feature Expansion

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

---

**End of Roadmap v16.0**
