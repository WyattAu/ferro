#!/bin/bash
set -euo pipefail

echo "Running MIRI on crates with unsafe code..."

CRATES=(
    "ferro-client"
    "ferro-health"
    "ferro-mount-nfs"
    "ferro-fuse"
    "ferro-server"
    "ferro-desktop"
)

for crate in "${CRATES[@]}"; do
    echo "Checking $crate..."
    cargo miri test -p "$crate" -- -Zmiri-disable-isolation || {
        echo "MIRI failed for $crate"
        exit 1
    }
done

echo "MIRI checks complete."
