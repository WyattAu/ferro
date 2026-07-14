#!/bin/bash
set -euo pipefail

# Ensure nightly toolchain is available (rust-toolchain.toml pins stable)
rustup toolchain install nightly --component rust-src 2>/dev/null || true
export RUSTUP_TOOLCHAIN=nightly

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
