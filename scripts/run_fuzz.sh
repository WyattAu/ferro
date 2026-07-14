#!/bin/bash
set -euo pipefail

echo "Running fuzz targets for 60 seconds each..."

for target in fuzz_xml fuzz_json fuzz_vcard fuzz_ical; do
    echo "Fuzzing $target..."
    cargo fuzz run $target -- -max_total_time=60 || true
done

echo "Fuzzing complete."
