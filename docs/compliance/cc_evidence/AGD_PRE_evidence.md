# AGD_PRE: Preparative Procedures

## Assurance Family Requirement

The developer shall provide preparative procedures describing how to install and configure the TOE in a secure manner.

**EAL Level:** EAL 3+ (AGD_PRE.1)

## Evidence Artifacts

### 1. Docker Build (Dockerfile)

**File:** `Dockerfile`

| Stage | Base Image | Purpose |
|-------|-----------|---------|
| Builder | `rust:1.95-bookworm` | Full Rust build with system deps |
| Final | `scratch` | Minimal attack surface |

**Build Steps:**
1. Install system deps (pkg-config, libssl-dev, binaryen)
2. Add `wasm32-unknown-unknown` target for WASM frontend
3. Install `trunk` 0.21.14 for WASM build
4. Copy manifests for dependency caching
5. Build WASM frontend (`trunk build --release`)
6. Build server and CLI binaries (`cargo build --release --locked`)
7. Copy binaries to scratch image
8. Add OCI labels for EvergreenImageRegistry compliance

**Security Features:**
- `CGO_ENABLED=0` — Static linking, no C dependencies
- `scratch` base — No shell, no OS, minimal attack surface
- Pinned toolchain (`rust:1.95-bookworm`)
- `--locked` flag — Reproducible builds from Cargo.lock

### 2. Docker Compose

**File:** `docker-compose.yml`

| Component | Configuration |
|-----------|--------------|
| ferro service | Build from Dockerfile, volumes for persistence, health check |
| caddy service | Reverse proxy, auto-HTTPS via Let's Encrypt |
| Health check | `curl -sf http://localhost:8080/.well-known/ferro` (30s interval) |
| Volumes | Named volumes for data persistence |
| Restart policy | `unless-stopped` |

Additional compose files:
- `docker-compose.pg.yml` — PostgreSQL integration
- `docker-compose.redis.yml` — Redis caching
- `docker-compose.chaos.yml` — Chaos engineering
- `deploy/docker-compose.production.yml` — Production config
- `deploy/docker-compose.ecosystem.yml` — Full ecosystem

### 3. Nix Build (flake.nix)

**Path:** `flake.nix`

| Dev Shell | Purpose |
|-----------|---------|
| `nix develop` | Full development environment |
| `nix develop .#web` | WASM build environment |
| `nix develop .#desktop` | Tauri desktop environment |

Nix provides reproducible builds with pinned dependencies.

### 4. CI/CD Pipeline

**File:** `.github/workflows/ci.yml`

| Job | Purpose | Timeout |
|-----|---------|---------|
| `fmt` | Code formatting check | 30 min |
| `clippy` | Linting with `-D warnings` | 30 min |
| `msrv` | MSRV verification (Rust 1.92) | 30 min |
| `test` | Unit + integration tests | 30 min |
| `test-features` | Individual feature flag tests (6 configs) | 30 min |
| `test-feature-combos` | Feature combination tests (12 combos) | 30 min |
| `test-pg` | PostgreSQL integration tests | 30 min |
| `audit` | Security audit (cargo-deny) | 30 min |
| `build` | Release build | 30 min |
| `security-scan` | Trivy vulnerability scan | 30 min |
| `sbom` | SBOM generation (SPDX) | 30 min |
| `docker` | Docker image build | 30 min |
| `docker-publish` | Push to GHCR (main only) | 30 min |
| `deploy-staging` | K8s staging deploy | 15 min |
| `deploy-production` | K8s production deploy | 15 min |

### 5. System Dependencies

| Dependency | Version | Required For |
|-----------|---------|-------------|
| Rust | 1.92+ (pinned 1.95 in toolchain) | Compilation |
| OpenSSL | System package | PostgreSQL support |
| pkg-config | System package | Build scripts |
| protobuf-compiler | System package | gRPC |

### 6. Build Verification

```bash
# Verify reproducible build
cargo build --release --locked
# Uses Cargo.lock for exact dependency versions

# Verify no unsafe code in security paths
grep -r "unsafe" crates/server-security/
grep -r "unsafe" crates/crypto/
grep -r "unsafe" crates/auth/

# Verify static linking
file target/release/ferro-server
# Should show "statically linked"
```

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Verification procedures | High | Need documented build verification steps |
| Secure configuration checklist | Medium | Production hardening guide needed |

## Verification Instructions

```bash
# Reproduce Docker build
docker build -t ferro:verify .
docker run --rm ferro:verify --version

# Reproduce Nix build
nix build
./result/bin/ferro-server --version

# Verify Cargo.lock is committed
git diff HEAD -- Cargo.lock
# Should show no changes

# Verify MSRV
rustup run 1.92 cargo check --all
```

## References

- `Dockerfile` — Multi-stage Docker build
- `docker-compose.yml` — Container orchestration
- `flake.nix` — Nix reproducible builds
- `.github/workflows/ci.yml` — CI/CD pipeline
- `rust-toolchain.toml` — Pinned Rust version
- `Cargo.lock` — Locked dependencies
