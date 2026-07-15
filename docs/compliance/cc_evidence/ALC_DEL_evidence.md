# ALC_DEL: Delivery Procedures

## Assurance Family Requirement

The developer shall describe the procedures used to deliver the TOE to customers, including integrity verification and distribution channels.

**EAL Level:** EAL 3+ (ALC_DEL.1)

## Evidence Artifacts

### 1. Release Workflow

**File:** `.github/workflows/release.yml`

**Trigger:** Push of `v*` tags

**Pipeline:**
1. **Verify** — Waits for all core CI checks (Clippy, Rustfmt, MSRV, Test, Security Audit) to pass on main
2. **Build** — Cross-compiles for 6 targets:
   - `x86_64-unknown-linux-gnu` (Linux AMD64)
   - `x86_64-unknown-linux-musl` (Linux AMD64 static)
   - `aarch64-unknown-linux-gnu` (Linux ARM64)
   - `x86_64-apple-darwin` (macOS Intel)
   - `aarch64-apple-darwin` (macOS Apple Silicon)
   - `x86_64-pc-windows-msvc` (Windows)
3. **Create Release** — GitHub Release with binary artifacts

### 2. Binary Artifacts

| Artifact | Target | Notes |
|----------|--------|-------|
| `ferro-server` | x86_64-unknown-linux-gnu | Primary Linux binary |
| `ferro-server-musl` | x86_64-unknown-linux-musl | Statically linked (scratch Docker) |
| `ferro-server-aarch64` | aarch64-unknown-linux-gnu | ARM64 Linux |
| `ferro-server-macos` | x86_64-apple-darwin | Intel macOS |
| `ferro-server-macos-arm` | aarch64-apple-darwin | Apple Silicon macOS |
| `ferro-server.exe` | x86_64-pc-windows-msvc | Windows |

### 3. Docker Image Build

**File:** `.github/workflows/ci.yml:260-300`

| Step | Description |
|------|-------------|
| Build | `docker/build-push-action` with BuildKit caching |
| Test | `docker build` verification |
| Publish | Push to `ghcr.io` (GitHub Container Registry) |
| Tags | `latest` + `${{ github.sha }}` |

**Registry:** `ghcr.io/${{ github.repository }}`

### 4. Integrity Verification

| Mechanism | Implementation |
|-----------|---------------|
| `Cargo.lock` | Exact dependency versions for reproducibility |
| `--locked` flag | Prevents lock file modification during build |
| `cargo-deny` | License and advisory checking |
| Trivy scan | Container vulnerability scanning |
| SBOM generation | SPDX JSON format via `anchore/sbom-action` |
| Git commit signing | Verified commits (GPG/SSH) |

### 5. Distribution Channels

| Channel | Description |
|---------|-------------|
| GitHub Releases | Binary artifacts for all platforms |
| GHCR (ghcr.io) | Docker images |
| crates.io | Rust package (if published) |
| Nix flakes | Reproducible builds |
| Source | Git repository |

### 6. Deployment Pipeline

**File:** `.github/workflows/ci.yml:302-341`

| Stage | Environment | Trigger |
|-------|-------------|---------|
| Staging | `ferro-staging` namespace | After `docker-publish` on main |
| Production | `ferro` namespace | After staging success |

**Kubernetes Deployment:**
```bash
kubectl set image deployment/ferro-server \
  ferro-server=ghcr.io/${{ github.repository }}:${{ github.sha }} \
  -n ferro
kubectl rollout status deployment/ferro-server -n ferro --timeout=300s
```

### 7. Release Process

```bash
# Tag a release
git tag v3.x.x
git push origin v3.x.x

# Verify release builds
gh run list --workflow=release.yml --limit 1

# Download artifacts
gh release download v3.x.x
```

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| Delivery plan document | Medium | Formal delivery procedures document needed |
| Checksum/signature verification | Medium | Need documented verification steps for users |
| Build provenance | Medium | SLSA provenance not yet implemented |

## Verification Instructions

```bash
# Verify release workflow exists
gh workflow list | grep release

# Verify Docker image is published
docker pull ghcr.io/WyattAu/ferro:latest

# Verify binary artifacts
gh release view v3.x.x --json assets --jq '.assets[].name'

# Verify SBOM is generated
gh run list --workflow=ci.yml --limit 1 --json jobs --jq '.jobs[] | select(.name == "sbom")'

# Verify build reproducibility
cargo build --release --locked
sha256sum target/release/ferro-server
```

## References

- `.github/workflows/release.yml` — Release automation
- `.github/workflows/ci.yml:260-300` — Docker build/publish
- `.github/workflows/ci.yml:241-258` — SBOM generation
- `Dockerfile` — Container build
- `Cargo.lock` — Dependency lock file
