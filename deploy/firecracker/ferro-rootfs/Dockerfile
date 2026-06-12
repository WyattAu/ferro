# Ferro Rootfs for Firecracker microVMs
# Minimal rootfs with pre-built binaries from official Ferro image
#
# EvergreenImageRegistry v30.0.0 Compliant:
# - Base: wolfi (alpine BANNED)
# - User: 65532:65532 (OpenShift nonroot)
# - Labels: OCI + Evergreen
# - Health: wget probe
# - Build type: repack

FROM cgr.dev/chainguard/wolfi-base

LABEL org.opencontainers.image.title="Ferro Rootfs" \
      org.opencontainers.image.description="Ferro rootfs for Firecracker microVMs" \
      org.opencontainers.image.vendor="Ferro" \
      org.opencontainers.image.source="https://github.com/WyattAu/ferro" \
      evergreen.image.tier="standard" \
      evergreen.base.image="wolfi" \
      evergreen.constraint.nonroot="true" \
      evergreen.build.type="repack" \
      evergreen.security.cap-drop="ALL" \
      evergreen.security.no-new-privileges="true" \
      evergreen.health.type="http"

# Install minimal runtime dependencies
RUN apk add --no-cache ca-certificates wget

# Copy binaries from official Ferro image
COPY --from=ghcr.io/wyattau/ferro:latest /app/ferro-server /usr/local/bin/ferro-server
COPY --from=ghcr.io/wyattau/ferro:latest /app/ferro-cli /usr/local/bin/ferro-cli
COPY --from=ghcr.io/wyattau/ferro:latest /app/ui /usr/local/share/ferro/ui

# Create data directory with correct ownership
RUN mkdir -p /data && chown 65532:65532 /data

# Non-root user (OpenShift nonroot range)
USER 65532:65532

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 --start-period=10s \
    CMD wget -q --spider http://localhost:8080/.well-known/ferro || exit 1

ENTRYPOINT ["/usr/local/bin/ferro-server", "--host", "0.0.0.0", "--port", "8080", "--data-dir", "/data"]
STOPSIGNAL SIGTERM
