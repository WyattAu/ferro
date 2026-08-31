#!/bin/sh
set -e

# In scratch containers, UID/GID remapping is not possible (no shell utilities).
# The container runs as USER 65532:65532 (OpenShift nonroot).
# For custom UID/GID, use a different base image (e.g., wolfi or distroless).

exec "$@"
