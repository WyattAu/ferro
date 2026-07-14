#!/bin/bash
set -euo pipefail

echo "Running quality gate checks..."

# Format check
echo "Checking format..."
cargo fmt --all -- --check

# Clippy check
echo "Running clippy..."
cargo clippy --workspace --all-targets --features "s3,gcs,azure,pg,redis,ldap" --locked -- -D warnings

# Test check
echo "Running tests..."
cargo test --locked --all --features "s3,gcs,azure,pg,redis,ldap"

# Security audit
echo "Running security audit..."
if ! command -v cargo-deny &>/dev/null; then
    cargo install cargo-deny --locked --version 0.19.4
fi
cargo deny check advisories bans licenses sources

# Coverage check
echo "Checking coverage..."
if ! command -v cargo-tarpaulin &>/dev/null; then
    cargo install cargo-tarpaulin --locked
fi
cargo tarpaulin --all --features "s3,gcs,azure,pg,redis,ldap" --out Xml --output-dir coverage
COVERAGE=$(cargo tarpaulin --all --features "s3,gcs,azure,pg,redis,ldap" --quiet 2>/dev/null | grep -oP 'Coverage.*?\K[0-9.]+' | head -1 || echo "0")
if (( $(echo "$COVERAGE < 85" | bc -l) )); then
    echo "Coverage ${COVERAGE}% is below 85% threshold"
    exit 1
fi
echo "Coverage: ${COVERAGE}%"

echo "All quality gate checks passed."
