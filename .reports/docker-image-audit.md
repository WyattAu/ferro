# Ferro Docker Image Audit — EvergreenImageRegistry Standards Compliance

**Date:** 2026-06-12
**Auditor:** Nexus (Principal Systems Architect)
**Standard:** EvergreenImageRegistry v30.0.0 (982 images, 5 pillars, C001-C029)
**Scope:** 4 Ferro Dockerfiles

---

## Executive Summary

| Dockerfile | Status | Critical Issues | High Issues | Medium Issues |
|------------|--------|-----------------|-------------|---------------|
| `Dockerfile` (main) | **NEEDS WORK** | 3 | 4 | 3 |
| `deploy/Dockerfile.web` | **NON-COMPLIANT** | 5 | 2 | 1 |
| `deploy/Dockerfile.admin` | **NON-COMPLIANT** | 5 | 2 | 1 |
| `deploy/firecracker/ferro-rootfs/Dockerfile` | **NON-COMPLIANT** | 4 | 1 | 1 |

---

## Detailed Audit per Constraint

### C001: Non-root Execution (UID 65532) — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | Uses UID 1000 (`ferro:ferro`), not 65532 |
| `Dockerfile.web` | **FAIL** | No USER directive — runs as root |
| `Dockerfile.admin` | **FAIL** | No USER directive — runs as root |
| `Dockerfile.firecracker` | **FAIL** | No USER directive — runs as root |

**Required fix:** All images MUST run as `USER 65532:65532` (OpenShift nonroot range).

### C002: Read-Only Root Filesystem — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | Writes to `/data` at runtime, has `curl` for healthcheck |
| `Dockerfile.web` | **FAIL** | nginx writes logs to filesystem |
| `Dockerfile.admin` | **FAIL** | nginx writes logs to filesystem |
| `Dockerfile.firecracker` | **FAIL** | Creates `/data` directory |

**Required fix:** All writable paths must be mounted volumes. nginx must log to stdout.

### C003: No Shell — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | `debian:bookworm-slim` has `/bin/sh` |
| `Dockerfile.web` | **FAIL** | `nginx:alpine` has `/bin/sh` |
| `Dockerfile.admin` | **FAIL** | `nginx:alpine` has `/bin/sh` |
| `Dockerfile.firecracker` | **FAIL** | `alpine:3.19` has `/bin/sh` |

**Required fix:** Final stage MUST use `scratch`, `wolfi`, or `distroless`. Shell access is a critical attack vector.

### C004: No Package Manager — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | `apt-get` in final stage (ca-certificates, libssl3, curl) |
| `Dockerfile.web` | **FAIL** | nginx:alpine has `apk` |
| `Dockerfile.admin` | **FAIL** | nginx:alpine has `apk` |
| `Dockerfile.firecracker` | **FAIL** | `apk add` in final stage |

**Required fix:** All packages must be installed in builder stage and copied to final stage.

### C005: Static Linking — HIGH

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | Uses `libssl3` dynamic linking, not `CGO_ENABLED=0` |
| `Dockerfile.web` | N/A | nginx is not Rust |
| `Dockerfile.admin` | N/A | nginx is not Rust |
| `Dockerfile.firecracker` | N/A | Uses COPY from pre-built image |

**Required fix:** Build with `CGO_ENABLED=0` and use `rustls` instead of `openssl` for static linking.

### C007: Zero Critical/High CVEs — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **UNKNOWN** | No Trivy/Grype scan configured |
| `Dockerfile.web` | **UNKNOWN** | nginx:alpine likely has CVEs |
| `Dockerfile.admin` | **UNKNOWN** | nginx:alpine likely has CVEs |
| `Dockerfile.firecracker` | **UNKNOWN** | alpine:3.19 likely has CVEs |

**Required fix:** Add vulnerability scanning to CI/CD pipeline.

### C008: Signed via Cosign — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| All | **FAIL** | No Cosign signing configured |

### C009: SBOM Generated — HIGH

| Dockerfile | Status | Details |
|------------|--------|---------|
| All | **FAIL** | No SBOM generation configured |

### C015: No Debug Tools — HIGH

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | `curl` in final stage (debug tool) |
| `Dockerfile.web` | PASS | No debug tools |
| `Dockerfile.admin` | PASS | No debug tools |
| `Dockerfile.firecracker` | **FAIL** | `curl` in final stage |

### C016: No Hardcoded Secrets — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| All | PASS | No hardcoded secrets found |

### C018: No sudo/su — HIGH

| Dockerfile | Status | Details |
|------------|--------|---------|
| All | PASS | No sudo/su found |

### C019: Pinned Image Tags — CRITICAL

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | `rust:1.95-bookworm` — tag, not digest |
| `Dockerfile.web` | **FAIL** | `rust:1.92` — tag, not digest |
| `Dockerfile.admin` | **FAIL** | `rust:1.92` — tag, not digest |
| `Dockerfile.firecracker` | **FAIL** | `alpine:3.19` — tag, not digest |

**Required fix:** Pin FROM to `@sha256:<digest>` for reproducibility.

### Multi-stage Builds — MANDATORY

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | PASS | 2 stages (builder + runtime) |
| `Dockerfile.web` | PASS | 2 stages (builder + nginx) |
| `Dockerfile.admin` | PASS | 2 stages (builder + nginx) |
| `Dockerfile.firecracker` | **FAIL** | Single stage (no builder) |

### HEALTHCHECK — MANDATORY

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **PARTIAL** | Has HEALTHCHECK but uses `curl` (not allowed in C003) |
| `Dockerfile.web` | **FAIL** | No HEALTHCHECK |
| `Dockerfile.admin` | **FAIL** | No HEALTHCHECK |
| `Dockerfile.firecracker` | **FAIL** | No HEALTHCHECK |

**Required fix:** Use TCP/HTTP probe via health-shim, not curl.

### OCI Labels — MANDATORY

| Dockerfile | Status | Details |
|------------|--------|---------|
| All | **FAIL** | No OCI labels (org.opencontainers.image.*) |

### Configurable UID (C028) — MANDATORY

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | Hardcoded UID 1000, no APP_UID/APP_GID |
| `Dockerfile.web` | **FAIL** | No UID configuration |
| `Dockerfile.admin` | **FAIL** | No UID configuration |
| `Dockerfile.firecracker` | **FAIL** | No UID configuration |

### Base Image Hierarchy (ADR-007) — MANDATORY

| Dockerfile | Status | Details |
|------------|--------|---------|
| `Dockerfile` | **FAIL** | `debian:bookworm-slim` — BANNED |
| `Dockerfile.web` | **FAIL** | `nginx:alpine` — BANNED (alpine) |
| `Dockerfile.admin` | **FAIL** | `nginx:alpine` — BANNED (alpine) |
| `Dockerfile.firecracker` | **FAIL** | `alpine:3.19` — BANNED |

**Permanently banned:** debian-slim, alpine, ubuntu, centos. Use wolfi, distroless, or scratch.

---

## Compliance Score

| Category | Max Score | Ferro Score |
|----------|-----------|-------------|
| Security (C001-C019) | 20 | 2 |
| Reliability (probes, signals) | 5 | 1 |
| Configuration (labels, UID) | 5 | 0 |
| Documentation (READMEs) | 5 | 0 |
| Structural (multi-stage, scratch) | 5 | 3 |
| **Total** | **40** | **6 (15%)** |

---

## Priority Remediation Plan

### P0: Critical (Must fix before any release)

1. **Replace all base images** with wolfi/scratch/distroless
2. **Set USER 65532:65532** on all images
3. **Pin FROM to digests** for reproducibility
4. **Remove shell and package managers** from final stages
5. **Add OCI labels** to all Dockerfiles
6. **Add health-shim** for health probes (replace curl)
7. **Enable CGO_ENABLED=0** for static binaries
8. **Add Trivy scanning** to CI/CD
9. **Add Cosign signing** to CI/CD
10. **Generate SBOMs** with Syft

### P1: High (Fix within 1 week)

1. **Add APP_UID/APP_GID** support with runtime re-creation
2. **Remove curl** from final stages
3. **Add read-only root filesystem** support
4. **Add nginx log-to-stdout** configuration
5. **Add HEALTHCHECK** to web/admin images

### P2: Medium (Fix within 1 month)

1. **Add .dockerignore** files
2. **Add per-image READMEs**
3. **Add SECURITY.md** for vulnerability reporting
4. **Optimize image sizes** (target <50MB for scratch, <200MB for wolfi)

---

## Recommended New Dockerfiles

### Main Server (scratch-based)

```dockerfile
# Stage 1: Build
FROM rust:1.95-bookworm AS builder
# ... build steps ...

# Stage 2: Runtime (scratch)
FROM scratch
COPY --from=builder /app/target/release/ferro-server /ferro-server
COPY --from=builder /app/crates/web/dist /ui
COPY --from=ghcr.io/wyattau/evergreenshim/cache-shim:latest /shim /shim
USER 65532:65532
HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD ["/shim", "healthcheck", "--http", "127.0.0.1:8080/.well-known/ferro"]
ENTRYPOINT ["/shim", "run", "-c", "/ferro-server"]
STOPSIGNAL SIGTERM
```

### Web Frontend (wolfi-based for nginx)

```dockerfile
# Stage 1: Build WASM
FROM rust:1.95-bookworm AS builder
# ... build WASM ...

# Stage 2: Runtime (wolfi with nginx)
FROM cgr.dev/chainguard/wolfi-base
RUN apk add --no-cache nginx ca-certificates
COPY --from=builder /app/crates/web/dist /usr/share/nginx/html
USER 65532:65532
HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD wget -q --spider http://localhost:80/ || exit 1
ENTRYPOINT ["nginx", "-g", "daemon off;"]
```
