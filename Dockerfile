# ── Stage 1: Build Web UI ──────────────────────────────────────────────────
FROM node:20-slim AS ui-builder

LABEL org.opencontainers.image.title="Ferro"
LABEL org.opencontainers.image.description="Self-hosted file server with WebDAV, S3-compatible API, federation, and WASM workers"
LABEL org.opencontainers.image.url="https://github.com/WyattAu/ferro"
LABEL org.opencontainers.image.documentation="https://wyattau.github.io/ferro/"
LABEL org.opencontainers.image.source="https://github.com/WyattAu/ferro"
LABEL org.opencontainers.image.vendor="WyattAu"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

ENV PATH="/root/.cargo/bin:${PATH}" \
    CARGO_HOME="/root/.cargo"

RUN apt-get update && apt-get install -y --no-install-recommends build-essential ca-certificates curl pkg-config libssl-dev && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.95.0 && \
    . /root/.cargo/env && \
    rustup target add wasm32-unknown-unknown && \
    cargo install trunk && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Copy all workspace member Cargo.toml files (required by cargo metadata)
COPY crates/common/Cargo.toml crates/common/
COPY crates/core/Cargo.toml crates/core/
COPY crates/dav/Cargo.toml crates/dav/
COPY crates/server/Cargo.toml crates/server/
COPY crates/web/Cargo.toml crates/web/
COPY crates/desktop/Cargo.toml crates/desktop/
COPY crates/cli/Cargo.toml crates/cli/
COPY crates/crypto/Cargo.toml crates/crypto/
COPY crates/fuse/Cargo.toml crates/fuse/
COPY crates/client/Cargo.toml crates/client/
COPY crates/benchmarks/Cargo.toml crates/benchmarks/
COPY crates/admin/Cargo.toml crates/admin/
COPY crates/observability/Cargo.toml crates/observability/
COPY crates/auth/Cargo.toml crates/auth/
COPY crates/webdav-handler/Cargo.toml crates/webdav-handler/
COPY crates/server-activitypub/Cargo.toml crates/server-activitypub/
COPY crates/server-webrtc/Cargo.toml crates/server-webrtc/
COPY crates/server-wopi/Cargo.toml crates/server-wopi/
COPY crates/server-versioning/Cargo.toml crates/server-versioning/
COPY crates/graphql/Cargo.toml crates/graphql/

# Create stub source files for all workspace members
RUN for crate in common core dav server web desktop cli crypto fuse client benchmarks admin observability auth webdav-handler server-activitypub server-webrtc server-wopi server-versioning graphql; do \
    mkdir -p crates/$crate/src; \
    [ -f crates/$crate/src/lib.rs ] || echo '' > crates/$crate/src/lib.rs; \
    done
# Stubs for benchmark targets (required by Cargo.toml [[bench]] entries)
RUN mkdir -p crates/benchmarks/benches && \
    for bench in storage dav_parsing crypto_ops webdav_ops; do \
    [ -f crates/benchmarks/benches/$bench.rs ] || echo 'fn main() {}' > crates/benchmarks/benches/$bench.rs; \
    done
RUN mkdir -p crates/server/benches && \
    for bench in throughput latency webdav_ops wasm_dispatch storage_ops; do \
    [ -f crates/server/benches/$bench.rs ] || echo 'fn main() {}' > crates/server/benches/$bench.rs; \
    done

# Now copy actual sources for crates needed by the web frontend build
COPY crates/web/index.html crates/web/
COPY crates/web/src/ crates/web/src/
COPY crates/common/src/ crates/common/src/

WORKDIR /app/crates/web

RUN trunk build --release --public-url "/ui/"

# ── Stage 2: Build Rust server ────────────────────────────────────────────
FROM rust:1.95-bookworm AS builder

ARG BUILD_FEATURES=""

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Copy all workspace member Cargo.toml files (required by cargo metadata)
COPY crates/common/Cargo.toml crates/common/
COPY crates/core/Cargo.toml crates/core/
COPY crates/dav/Cargo.toml crates/dav/
COPY crates/server/Cargo.toml crates/server/
COPY crates/web/Cargo.toml crates/web/
COPY crates/desktop/Cargo.toml crates/desktop/
COPY crates/cli/Cargo.toml crates/cli/
COPY crates/crypto/Cargo.toml crates/crypto/
COPY crates/fuse/Cargo.toml crates/fuse/
COPY crates/client/Cargo.toml crates/client/
COPY crates/benchmarks/Cargo.toml crates/benchmarks/
COPY crates/admin/Cargo.toml crates/admin/
COPY crates/observability/Cargo.toml crates/observability/
COPY crates/auth/Cargo.toml crates/auth/
COPY crates/webdav-handler/Cargo.toml crates/webdav-handler/
COPY crates/server-activitypub/Cargo.toml crates/server-activitypub/
COPY crates/server-webrtc/Cargo.toml crates/server-webrtc/
COPY crates/server-wopi/Cargo.toml crates/server-wopi/
COPY crates/server-versioning/Cargo.toml crates/server-versioning/
COPY crates/graphql/Cargo.toml crates/graphql/

# Create stub source files for all workspace members (dependency caching layer)
RUN for crate in common core dav server web desktop cli crypto fuse client benchmarks admin observability auth webdav-handler server-activitypub server-webrtc server-wopi server-versioning graphql; do \
    mkdir -p crates/$crate/src; \
    echo '' > crates/$crate/src/lib.rs; \
    done
RUN mkdir -p crates/benchmarks/benches && \
    for bench in storage dav_parsing crypto_ops webdav_ops; do \
    echo 'fn main() {}' > crates/benchmarks/benches/$bench.rs; \
    done
RUN mkdir -p crates/server/benches && \
    for bench in throughput latency webdav_ops wasm_dispatch storage_ops; do \
    echo 'fn main() {}' > crates/server/benches/$bench.rs; \
    done
RUN echo 'fn main() {}' > crates/server/src/main.rs
RUN echo 'fn main() {}' > crates/cli/src/main.rs
RUN echo 'fn main() {}' > crates/desktop/src/main.rs

RUN cargo build --release --package ferro-server --package ferro-cli --features "${BUILD_FEATURES}" 2>/dev/null || true

COPY . .
RUN for crate in common core dav server web desktop cli crypto fuse client benchmarks admin observability auth webdav-handler server-activitypub server-webrtc server-wopi server-versioning graphql; do \
    touch crates/$crate/src/lib.rs 2>/dev/null || true; \
    done
RUN touch crates/server/src/main.rs crates/cli/src/main.rs crates/desktop/src/main.rs

RUN cargo build --release --package ferro-server --package ferro-cli --features "${BUILD_FEATURES}"

# ── Runtime stage ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --gid 1000 ferro && useradd --uid 1000 --gid ferro --create-home ferro

WORKDIR /app

COPY --from=builder --chown=ferro:ferro /app/target/release/ferro-server /app/ferro-server
COPY --from=builder --chown=ferro:ferro /app/target/release/ferro-cli   /app/ferro-cli
COPY --from=ui-builder --chown=ferro:ferro /app/crates/web/dist /app/ui

RUN mkdir -p /data && chown ferro:ferro /data

USER ferro

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -sf http://localhost:8080/.well-known/ferro > /dev/null || exit 1

ENTRYPOINT ["/app/ferro-server"]
CMD ["--host", "0.0.0.0", "--port", "8080", "--data-dir", "/data", "--static-dir", "/app/ui"]
