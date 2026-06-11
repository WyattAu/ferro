#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DURATION="${1:-3600}"

echo "══════════════════════════════════════════════════════"
echo "  ferro soak test"
echo "  Duration: ${DURATION}s ($(($DURATION / 3600))h $(($DURATION % 3600 / 60))m)"
echo "══════════════════════════════════════════════════════"
echo

echo "[soak] Building ferro-server (release)..."
cargo build -p ferro-server --release 2>&1

echo "[soak] Running soak test (SOAK_DURATION=${DURATION})..."
SOAK_DURATION="$DURATION" \
  cargo test -p ferro-server --test soak_test --release -- --ignored 2>&1

EXIT_CODE=$?

echo
if [ $EXIT_CODE -eq 0 ]; then
    echo "[soak] PASSED"
else
    echo "[soak] FAILED (exit code $EXIT_CODE)"
fi

RESULTS="$ROOT_DIR/target/soak-results.json"
if [ -f "$RESULTS" ]; then
    echo "[soak] Results written to $RESULTS"
    echo
    echo "  Summary:"
    echo "    Total requests: $(python3 -c "import json; d=json.load(open('$RESULTS')); print(d['total_requests'])" 2>/dev/null || echo 'N/A')"
    echo "    Req/sec:        $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['requests_per_second']:.1f}\")" 2>/dev/null || echo 'N/A')"
    echo "    Error rate:     $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['overall_error_rate_pct']:.2f}%\")" 2>/dev/null || echo 'N/A')"
    echo "    P99 latency:    $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['p99_ms']:.2f}ms\")" 2>/dev/null || echo 'N/A')"
    echo "    Peak RSS:       $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['peak_rss_kb']} KB\")" 2>/dev/null || echo 'N/A')"
fi

exit $EXIT_CODE
