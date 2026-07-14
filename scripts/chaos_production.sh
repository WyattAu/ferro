#!/bin/bash
set -euo pipefail

echo "Running chaos tests in production-like environment..."

# Setup production-like environment
echo "Setting up environment..."
docker-compose -f docker-compose.chaos.yml up -d

# Wait for services
echo "Waiting for services..."
sleep 30

# Run experiments
echo "Running network partition experiment..."
cargo test -p ferro-chaos --test integration -- test_network_partition

echo "Running disk failure experiment..."
cargo test -p ferro-chaos --test integration -- test_disk_failure

echo "Running memory pressure experiment..."
cargo test -p ferro-chaos --test integration -- test_memory_pressure

echo "Running CPU saturation experiment..."
cargo test -p ferro-chaos --test integration -- test_cpu_saturation

# Cleanup
echo "Cleaning up..."
docker-compose -f docker-compose.chaos.yml down

echo "Chaos tests complete."
