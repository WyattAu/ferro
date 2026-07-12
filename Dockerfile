# syntax=docker/dockerfile:1
# ==============================================================================
# Stage 1: Build (official Rust image for reliable C compilation)
#
# Note: The builder uses rust:1.95-bookworm (Debian full, not slim) because
# wolfi's GCC versions have compatibility issues with libdeflate-sys (trunk dep).
# The final stage uses scratch for minimal attack surface.
#
# EvergreenImageRegistry Compliance:
# - Pin FROM to @sha256:<digest> for reproducibility
# - To pin: replace tags with digest from `docker inspect --format='{{index .RepoDigests 0}}'`
# ==============================================================================
# Pin: run `docker inspect --format='{{index .RepoDigests 0}}' rust:1.95-bookworm` to get digest
FROM rust:1.97-bookworm AS builder

ARG BUILD_FEATURES=""

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    binaryen \
    && rm -rf /var/lib/apt/lists/*

# Add wasm32 target for WASM frontend build
RUN rustup target add wasm32-unknown-unknown

# Install trunk for WASM frontend build
RUN cargo install trunk --version 0.21.14 --locked

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY migrations/ migrations/

# Build WASM frontend first (trunk needs wasm32 target)
# For BuildKit, add cache mounts: --mount=type=cache,target=/usr/local/cargo/registry
WORKDIR /app/crates/web
RUN trunk build --release --dist dist

WORKDIR /app

# Build server and CLI binaries with static linking
ENV CGO_ENABLED=0
RUN cargo build --release --locked --package ferro-server --package ferro-cli --features "${BUILD_FEATURES}"

# ==============================================================================
# Stage 2: Final (scratch - minimal attack surface)
# ==============================================================================
FROM scratch

# EvergreenImageRegistry v30.0.0 OCI labels
LABEL org.opencontainers.image.title="Ferro" \
      org.opencontainers.image.description="Self-hosted file sync server" \
      org.opencontainers.image.vendor="Ferro" \
      org.opencontainers.image.source="https://github.com/WyattAu/ferro" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later" \
      evergreen.image.tier="standard" \
      evergreen.base.image="scratch" \
      evergreen.constraint.nonroot="true" \
      evergreen.constraint.scratch="true" \
      evergreen.build.type="source-build" \
      evergreen.security.cap-drop="ALL" \
      evergreen.security.no-new-privileges="true" \
      evergreen.security.read-only-rootfs="true"

# Copy CA certificates from builder (scratch has none)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy application binaries
COPY --from=builder /app/target/release/ferro-server /ferro-server
COPY --from=builder /app/target/release/ferro-cli /ferro-cli
COPY --from=builder /app/crates/web/dist /ui

# Copy health-shim (TCP probe, no curl needed)
# Pin: replace `latest` with specific version tag or @sha256:digest from registry
COPY --from=ghcr.io/wyattau/evergreenshim/cache-shim:latest /shim /shim

# Non-root user (OpenShift nonroot range)
USER 65532:65532

# Health check via health-shim TCP probe
HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD ["/shim", "healthcheck", "--tcp", "127.0.0.1:8080"]

# Expose port
EXPOSE 8080

# Entrypoint wrapper for UID/GID remapping (e.g. OpenShift)
COPY --chmod=755 deploy/entrypoint.sh /entrypoint.sh

# Entrypoint with shim for graceful lifecycle management
# Data directory must be mounted at runtime: -v /path/to/data:/data
ENTRYPOINT ["/entrypoint.sh", "/shim", "run", "-c", "/ferro-server", "--host", "0.0.0.0", "--port", "8080", "--data-dir", "/data"]

# Graceful shutdown signal
STOPSIGNAL SIGTERM
