# ADV_ARC: Security Architecture Description

## Assurance Family Requirement

The developer shall provide a security architecture description of the TOE, including:
- Security objectives and security functions
- Architectural decomposition and component interactions
- Trust boundaries and data flow
- Protection of security functions from external interference

**EAL Level:** EAL 3+ (ADV_ARC.3)

## Evidence Artifacts

### 1. Architecture Overview

| Artifact | Path | Description |
|----------|------|-------------|
| Crate dependency graph | `.specs/02_architecture/crate_dependency_graph.md` | Mermaid graph of 57-crate workspace with 6 architectural layers |
| README architecture section | `README.md:387-482` | Crate table with descriptions for all workspace components |
| Blue Papers | `.specs/02_architecture/` | Detailed design specifications (if present) |

### 2. Architectural Layers

| Layer | Crates | Trust Level |
|-------|--------|-------------|
| Layer 0: Foundation | `common` | High — shared types, zero internal deps |
| Layer 1: Core | `core`, `auth`, `crypto`, `circuit-breaker`, `crdt`, `rate-limiter`, `event-bus`, `cache`, `health` | High — security primitives |
| Layer 2: Domain/Protocol | `dav`, `caldav`, `webdav-handler`, `sync-protocol`, `offline`, `distributed`, `graphql` | Medium — protocol handling |
| Layer 3: Infrastructure | `server-security`, `server-security-middleware`, `server-compliance`, `server-content`, `server-storage-ops`, etc. (20+ crates) | Medium — middleware enforcement |
| Layer 4: Application | `server` | Medium — HTTP/HTTPS entry point |
| Layer 5: Clients | `web`, `cli`, `client`, `desktop`, `mobile`, `admin` | Low — external-facing |

### 3. Security-Relevant Components

| Component | Crate | Security Function |
|-----------|-------|-------------------|
| Access control | `auth` (OIDC, Cedar RBAC, TOTP, WebAuthn, LDAP) | SF-AC |
| Audit logging | `audit-log` | SF-AU |
| Cryptography | `crypto` (SHA-256, AES-GCM, ECDSA) | SF-CP |
| Key management | `server-security` (3-level hierarchy) | SF-KM |
| Data protection | `server-compliance` (WORM, retention) | SF-DP |
| Integrity | `core` (CAS, hash chain) | SF-IC |
| Recovery | `server-storage-ops` (snapshots) | SF-RC |

### 4. Trust Boundaries

| Boundary | Description |
|----------|-------------|
| TOE boundary | Server binary (`ferro-server`), CLI, Web UI, Desktop client |
| Network boundary | HTTPS (port 8080), WebDAV (port 8080) |
| User roles | Administrators, Regular users, Guest users |
| External integrations | OIDC providers, S3/GCS/Azure storage, ActivityPub federation |

### 5. Architectural Decisions

No formal ADRs found in `.adrs/` directory. Architectural decisions are documented within:
- `.specs/02_architecture/crate_dependency_graph.md` — layer separation rationale
- `docs/compliance/common_criteria_preparation.md` — security function decisions
- `docs/compliance/nist_sp80053_mapping.md` — control implementation decisions

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Data flow diagrams | High | Missing — need network-level data flow |
| Trust boundary diagrams | High | Missing — need visual TOE boundary |
| TOE boundary diagram | High | Missing — required for ST document |

## Verification Instructions

```bash
# Verify crate dependency graph is current
cargo metadata --format-version 1 | jq '.packages | length'
# Should return 57+ crates

# Verify architecture matches graph
cargo tree --workspace --depth 1 | head -60

# Verify security crate isolation
cargo tree -p ferro-auth -e normal --depth 0
cargo tree -p ferro-crypto -e normal --depth 0
```

## References

- `docs/compliance/common_criteria_preparation.md` — Full CC preparation document
- `docs/compliance/nist_sp80053_mapping.md` — NIST control mapping
- `docs/security/security_controls_matrix.md` — Controls matrix
- `docs/security/attack_scenarios.md` — STRIDE threat model
