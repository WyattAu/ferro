#!/bin/sh
# Ferro FUSE mount script for Linux
# Usage: ./ferro-mount-linux.sh [server-url] [mount-point] [token]
#
# Prerequisites:
#   - ferro-fuse-cross binary installed
#   - FUSE3 installed (apt install fuse3 / dnf install fuse3)
#   - User in 'fuse' group (sudo usermod -aG fuse $USER)

set -e

SERVER_URL="${1:-${FERRO_URL:-https://ferro.wyattau.com}}"
MOUNT_POINT="${2:-${FERRO_MOUNT:-$HOME/ferro}}"
TOKEN="${3:-${FERRO_TOKEN}}"

# Check dependencies
if ! command -v ferro-fuse-cross >/dev/null 2>&1; then
    echo "Error: ferro-fuse-cross not found. Install with: cargo install ferro-fuse-cross" >&2
    exit 1
fi

if [ ! -e /dev/fuse ]; then
    echo "Error: /dev/fuse not found. Install FUSE: sudo apt install fuse3" >&2
    exit 1
fi

if [ -z "$TOKEN" ]; then
    echo "Error: No token provided. Set FERRO_TOKEN or pass as third argument." >&2
    exit 1
fi

# Create mount point
mkdir -p "$MOUNT_POINT"

# Unmount if already mounted
if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
    echo "Unmounting existing mount at $MOUNT_POINT..."
    fusermount -u "$MOUNT_POINT"
fi

echo "Mounting Ferro at $MOUNT_POINT from $SERVER_URL"
ferro-fuse-cross --server-url "$SERVER_URL" --mount "$MOUNT_POINT" --token "$TOKEN"

echo "Ferro unmounted."
