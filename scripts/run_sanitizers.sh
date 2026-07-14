#!/bin/bash
set -euo pipefail

echo "Running AddressSanitizer..."
RUSTFLAGS="-Z sanitizer=address" \
RUSTDOCFLAGS="-Z sanitizer=address" \
cargo test -Zbuild-std --target x86_64-unknown-linux-gnu \
  -p ferro-common -p ferro-core -p ferro-auth

echo "Running ThreadSanitizer..."
RUSTFLAGS="-Z sanitizer=thread" \
RUSTDOCFLAGS="-Z sanitizer=thread" \
cargo test -Zbuild-std --target x86_64-unknown-linux-gnu \
  -p ferro-server -p ferro-server-collaboration

echo "Sanitizers complete."
