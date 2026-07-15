# AGD_OPE: Operational User Guidance

## Assurance Family Requirement

The developer shall provide operational user guidance describing how to securely use the TOE, including installation, configuration, and operational procedures.

**EAL Level:** EAL 3+ (AGD_OPE.1)

## Evidence Artifacts

### 1. Quick Start (README.md)

**File:** `README.md:40-68`

| Method | Instructions | Notes |
|--------|-------------|-------|
| Binary download | `curl -sL https://github.com/WyattAu/ferro/releases/latest/download/ferro-server-linux` | Direct binary |
| Docker | `docker compose up -d` | Includes Caddy reverse proxy with auto-HTTPS |
| From source | `cargo build --release --bin ferro-server` | Requires Rust 1.92+ |

### 2. Configuration Reference

**File:** `README.md:70-136` (CLI Flags table)

| Category | Key Flags |
|----------|-----------|
| Network | `--host`, `--port` |
| Storage | `--storage`, `--data-dir` |
| Authentication | `--oidc-issuer`, `--oidc-client-id`, `--admin-user`, `--admin-password` |
| Authorization | `--cedar-policy-file` |
| Encryption | `--wopi-token-secret` |
| Limits | `--max-body-size`, `--storage-quota`, `--trash-ttl` |
| Logging | `--log-level`, `--log-format` |
| Security | `--maintenance-mode`, `--cors-allowed-origins` |

### 3. Storage Backend Configuration

**File:** `README.md:138-169`

| Backend | Command | Credentials Required |
|---------|---------|---------------------|
| In-memory | `ferro-server` | None |
| Local filesystem | `--storage local:/path` | None |
| S3 | `--storage s3://bucket` | AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION |
| GCS | `--storage gs://bucket` | GOOGLE_APPLICATION_CREDENTIALS |
| Azure | `--storage az://container` | AZURE_STORAGE_ACCOUNT_NAME, AZURE_STORAGE_ACCOUNT_KEY |

### 4. Authentication Configuration

**File:** `README.md:205-229`

| Method | Configuration | Notes |
|--------|--------------|-------|
| Simple auth | `--admin-user admin --admin-password secret` | HTTP Basic Auth |
| OIDC | `--oidc-issuer URL --oidc-client-id ID` | PKCE flow, auto-enables Cedar |
| LDAP | `--ldap-url URL --ldap-bind-dn DN` | Feature-gated (`ldap`) |

### 5. Docker Compose Deployment

**File:** `README.md:231-277`, `docker-compose.yml`

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| ferro | Custom build | 8080 | Core server |
| caddy | `caddy:2-alpine` | 80, 443 | Reverse proxy, auto-HTTPS |

### 6. Development Guide

**File:** `CONTRIBUTING.md`

| Topic | Details |
|-------|---------|
| Prerequisites | Rust 1.92+, Git |
| Code style | rustfmt, clippy, doc comments for public APIs |
| Testing | `cargo test`, integration tests |
| PR process | Automated checks + 1 review required |

### 7. Documentation Site

**File:** `README.md:486-494`

| Document | URL |
|----------|-----|
| Introduction | `https://wyattau.github.io/ferro/introduction.html` |
| Quick Start | `https://wyattau.github.io/ferro/quickstart.html` |
| Configuration | `https://wyattau.github.io/ferro/configuration.html` |
| Deployment | `https://wyattau.github.io/ferro/deployment/docker.html` |
| API Reference | `https://wyattau.github.io/ferro/api/rest.html` |
| Security | `https://wyattau.github.io/ferro/security.html` |

### 8. docs/ Directory Structure

```
docs/
├── compliance/          # SOC 2, ISO 27001, HIPAA, PCI DSS, GDPR, CC
├── deployment/          # Docker, Nix, multi-region
├── sdk/                 # Developer guide, API reference, examples
├── security/            # Controls matrix, attack scenarios, pentest scope
├── sre/                 # Observability, performance, incident response
├── reliability/         # Monitoring, DR, chaos engineering
├── community/           # Governance, code of conduct
├── enterprise/          # SCIM, multi-tenancy, SAML/OIDC
├── features/            # CalDAV, push notifications
├── incident_response/   # Runbooks, tabletop exercises
├── monitoring/          # Prometheus, Grafana
├── performance/         # Optimization guides
└── strategy/            # Business, market analysis
```

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| User manual | Medium | Detailed end-user guide missing |
| Administration manual | Medium | System admin guide missing |
| Security hardening guide | Medium | Production hardening checklist missing |

## Verification Instructions

```bash
# Verify documentation site builds
nix develop -c cargo doc --workspace

# Verify Docker deployment works
docker compose up -d
curl -sf http://localhost:8080/.well-known/ferro

# Verify configuration validation
./ferro-server --validate-config --config ferro.toml

# Verify man page generation
./ferro-server --print-man-page
```

## References

- `README.md` — Primary user documentation
- `CONTRIBUTING.md` — Developer guide
- `docs/deployment/` — Deployment documentation
- `docs/sdk/developer_guide.md` — SDK developer guide
