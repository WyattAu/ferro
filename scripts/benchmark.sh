#!/usr/bin/env bash
set -euo pipefail

echo "=== Ferro Performance Benchmark ==="
echo ""

# Run crypto benchmarks (hash + xml escape)
echo "Running crypto benchmarks..."
cargo bench --package ferro-benchmarks --bench crypto 2>&1 | tee target/bench-crypto.txt

# Run crypto_ops benchmarks (password hash, hmac, sha256)
echo ""
echo "Running crypto_ops benchmarks..."
cargo bench --package ferro-benchmarks --bench crypto_ops 2>&1 | tee target/bench-crypto-ops.txt

# Run DAV benchmarks (ical + vcard parse/serialize)
echo ""
echo "Running DAV benchmarks..."
cargo bench --package ferro-benchmarks --bench dav 2>&1 | tee target/bench-dav.txt

# Run DAV parsing benchmarks
echo ""
echo "Running DAV parsing benchmarks..."
cargo bench --package ferro-benchmarks --bench dav_parsing 2>&1 | tee target/bench-dav-parsing.txt

# Run storage benchmarks
echo ""
echo "Running storage benchmarks..."
cargo bench --package ferro-benchmarks --bench storage 2>&1 | tee target/bench-storage.txt

# Run WebDAV operation benchmarks
echo ""
echo "Running WebDAV operation benchmarks..."
cargo bench --package ferro-benchmarks --bench webdav_ops 2>&1 | tee target/bench-webdav.txt

# Run DAV protocol benchmarks
echo ""
echo "Running DAV protocol benchmarks..."
cargo bench --package ferro-benchmarks --bench dav_protocol 2>&1 | tee target/bench-dav-protocol.txt

# Run auth TOTP benchmarks
echo ""
echo "Running auth TOTP benchmarks..."
cargo bench --package ferro-benchmarks --bench auth_totp 2>&1 | tee target/bench-auth-totp.txt

# Run ownCloud sync benchmark
echo ""
echo "Running ownCloud sync benchmark..."
cargo bench --package ferro-benchmarks --bench owncloud_sync 2>&1 | tee target/bench-owncloud.txt

echo ""
echo "Benchmark complete. Results saved to target/bench-*.txt"
