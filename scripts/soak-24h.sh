#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_TARGET_DIR="$ROOT_DIR/target/soak"
DURATION="${1:-86400}"
LOGFILE="$ROOT_DIR/target/soak/soak-24h.log"
RESULTS="$ROOT_DIR/target/soak/soak-results.json"

echo "══════════════════════════════════════════════════════"
echo "  ferro 24h soak test"
echo "  Duration: ${DURATION}s ($((DURATION / 3600))h $((DURATION % 3600 / 60))m)"
echo "  Log:      $LOGFILE"
echo "  Results:  $RESULTS"
echo "══════════════════════════════════════════════════════"
echo

echo "[soak] Building ferro-server (release) into target/soak..."
cargo build -p ferro-server --release 2>&1 | tee -a "$LOGFILE"

echo "[soak] Starting soak test..."
SOAK_DURATION="$DURATION" \
  SOAK_USERS="50" \
  cargo test -p ferro-server --test soak_test --release -- --ignored --nocapture 2>&1 | tee -a "$LOGFILE"

EXIT_CODE=${PIPESTATUS[0]}

echo
if [ $EXIT_CODE -eq 0 ]; then
    echo "[soak] PASSED" | tee -a "$LOGFILE"
else
    echo "[soak] FAILED (exit code $EXIT_CODE)" | tee -a "$LOGFILE"
fi

if [ -f "$RESULTS" ]; then
    echo
    echo "  Summary:" | tee -a "$LOGFILE"
    echo "    Total requests: $(python3 -c "import json; d=json.load(open('$RESULTS')); print(d['total_requests'])" 2>/dev/null || echo 'N/A')" | tee -a "$LOGFILE"
    echo "    Req/sec:        $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['requests_per_second']:.1f}\")" 2>/dev/null || echo 'N/A')" | tee -a "$LOGFILE"
    echo "    Error rate:     $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['overall_error_rate_pct']:.2f}%\")" 2>/dev/null || echo 'N/A')" | tee -a "$LOGFILE"
    echo "    P99 latency:    $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['p99_ms']:.2f}ms\")" 2>/dev/null || echo 'N/A')" | tee -a "$LOGFILE"
    echo "    Peak RSS:       $(python3 -c "import json; d=json.load(open('$RESULTS')); print(f\"{d['peak_rss_kb']} KB\")" 2>/dev/null || echo 'N/A')" | tee -a "$LOGFILE"
fi

exit $EXIT_CODE
