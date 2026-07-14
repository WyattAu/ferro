#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Ferro Performance Optimization ==="
echo ""

cd "$REPO_ROOT"

# 1. Build with optimizations
echo "[1/4] Building with release optimizations..."
cargo build --release 2>&1 | tail -5
echo "  Done."
echo ""

# 2. Run crypto benchmarks
echo "[2/4] Running crypto benchmarks..."
cargo bench --package ferro-benchmarks --bench crypto_ops 2>&1 | tee target/bench-crypto.txt
echo ""

# 3. Run DAV protocol benchmarks
echo "[3/4] Running DAV protocol benchmarks..."
cargo bench --package ferro-benchmarks --bench dav_protocol 2>&1 | tee target/bench-dav-protocol.txt
echo ""

# 4. Run auth TOTP benchmarks
echo "[4/4] Running auth TOTP benchmarks..."
cargo bench --package ferro-benchmarks --bench auth_totp 2>&1 | tee target/bench-auth-totp.txt
echo ""

echo "=== Performance Optimization Complete ==="
echo ""
echo "Results saved to:"
echo "  target/bench-crypto.txt"
echo "  target/bench-dav-protocol.txt"
echo "  target/bench-auth-totp.txt"
echo ""
echo "To run all benchmarks:"
echo "  cargo bench --package ferro-benchmarks"
echo ""
echo "To profile a specific binary with perf:"
echo "  perf record -g target/release/ferro-server"
echo "  perf report"
