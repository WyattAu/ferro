ARG RUST_VERSION=1.95
FROM rust:${RUST_VERSION}-bookworm AS builder

ARG BUILD_FEATURES=""

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY migrations/ migrations/

# Cache cargo registry and git deps, but NOT the target directory
# (build output must persist for COPY --from=builder)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --package ferro-server --package ferro-cli --features "${BUILD_FEATURES}"

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

RUN mkdir -p /data && chown ferro:ferro /data

USER ferro

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -sf http://localhost:8080/.well-known/ferro > /dev/null || exit 1

ENTRYPOINT ["/app/ferro-server"]
CMD ["--host", "0.0.0.0", "--port", "8080", "--data-dir", "/data"]
