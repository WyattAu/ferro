#!/bin/bash
set -euo pipefail

echo "Running chaos experiments..."

EXPERIMENTS=(
    "network_partition"
    "disk_failure"
    "memory_pressure"
    "cpu_saturation"
)

for exp in "${EXPERIMENTS[@]}"; do
    echo "Running experiment: $exp"
    cargo test -p ferro-chaos --test integration -- "test_$exp" || {
        echo "Experiment $exp failed"
        continue
    }
done

echo "Chaos experiments complete."