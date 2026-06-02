# ── Stage 1: Build Web UI ──────────────────────────────────────────────────

ARG RUST_VERSION=1.95

FROM node:20-slim AS ui-builder
ARG RUST_VERSION=1.95

LABEL org.opencontainers.image.title="Ferro"
LABEL org.opencontainers.image.description="Self-hosted file server with WebDAV, S3-compatible API, federation, and WASM workers"
LABEL org.opencontainers.image.url="https://github.com/WyattAu/ferro"
LABEL org.opencontainers.image.documentation="https://wyattau.github.io/ferro/docs/"
LABEL org.opencontainers.image.source="https://github.com/WyattAu/ferro"
LABEL org.opencontainers.image.vendor="WyattAu"
LABEL org.opencontainers.image.licenses="AGPL-3.0-or-later"

ENV PATH="/root/.cargo/bin:${PATH}" \
    CARGO_HOME="/root/.cargo"

RUN apt-get update && apt-get install -y --no-install-recommends build-essential ca-certificates curl pkg-config libssl-dev && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION} && \
    . /root/.cargo/env && \
    rustup target add wasm32-unknown-unknown && \
    cargo install trunk && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Create stub source files for all workspace members so cargo metadata works
RUN find crates -name "Cargo.toml" -exec sh -c 'dir=$(dirname "$1"); mkdir -p "$dir/src"; [ -f "$dir/src/lib.rs" ] || echo "" > "$dir/src/lib.rs"' _ {} \;

# Copy actual sources for crates needed by the web frontend build
COPY crates/web/index.html crates/web/
COPY crates/web/src/ crates/web/src/
COPY crates/common/src/ crates/common/src/

WORKDIR /app/crates/web

RUN trunk build --release --public-url "/ui/"

# ── Stage 2: Build Rust server ────────────────────────────────────────────
ARG RUST_VERSION=1.95
FROM rust:${RUST_VERSION}-bookworm AS builder

ARG BUILD_FEATURES=""

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

# Ensure all main entrypoints exist
RUN mkdir -p crates/server/src crates/cli/src crates/desktop/src && \
    touch crates/server/src/main.rs crates/cli/src/main.rs crates/desktop/src/main.rs

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
