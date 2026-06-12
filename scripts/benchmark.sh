#!/usr/bin/env bash
set -euo pipefail

echo "=== Ferro Performance Benchmark ==="
echo ""

# Run ownCloud sync benchmark
echo "Running ownCloud sync benchmark..."
cargo bench --package ferro-benchmarks --bench owncloud_sync 2>&1 | tee target/bench-results.txt

echo ""
echo "Benchmark complete. Results saved to target/bench-results.txt"
