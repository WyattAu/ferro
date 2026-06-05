#!/bin/bash
# Ferro Full Stack Integration Test Runner
# ======================================
# Starts server, seeds data, runs WASM traversal, then Desktop traversal.
# Usage: ./scripts/full-integration.sh [--desktop] [--skip-wasm]
set -euo pipefail
trap 'echo "CLEANUP: killing server (PID=$SERVER_PID)"; kill $SERVER_PID 2>/dev/null; wait $SERVER_PID 2>/dev/null' EXIT

PORT="${FERRO_PORT:-18089}"
BASE_URL="http://localhost:$PORT"
STORAGE="/tmp/ferro-int-$$"
OUTPUT="/tmp/ferro-results-$$"
SERVER_BIN="${FERO_SERVER_BIN:-target/debug/ferro-server}"
STATIC_DIR="${FERO_STATIC_DIR:-crates/web/dist}"
SKIP_WASM=false
RUN_DESKTOP=false

for arg in "$@"; do
    case "$arg" in
        --desktop) RUN_DESKTOP=true ;;
        --skip-wasm) SKIP_WASM=true ;;
        --port=*) PORT="${arg#*=}"; BASE_URL="http://localhost:$PORT" ;;
        --help) echo "Usage: $0 [--desktop] [--skip-wasm] [--port=N]"; exit 0 ;;
    esac
done

echo "=========================================="
echo "  Ferro Full Stack Integration Test"
echo "=========================================="
echo "  Port: $PORT"
echo "  Storage: $STORAGE"
echo "  Output: $OUTPUT"
echo "  WASM: $([ $SKIP_WASM = false ] && echo yes || echo no)"
echo "  Desktop: $([ $RUN_DESKTOP = true ] && echo yes || echo no)"
echo ""

# -- 0. Create output directory --
mkdir -p "$OUTPUT"

# -- 1. Kill conflicting servers --
echo "[1] Preparing..."
lsof -ti:"$PORT" 2>/dev/null | xargs -r kill -9 2>/dev/null || true
sleep 1

# -- 2. Seed storage --
echo "[2] Seeding storage at $STORAGE"
rm -rf "$STORAGE" && mkdir -p "$STORAGE"/{documents/reports,photos,music}
echo "root readme" > "$STORAGE/readme.md"
echo "root notes" > "$STORAGE/notes.txt"
echo "quarterly report" > "$STORAGE/documents/report.md"
echo "vacation photo" > "$STORAGE/photos/vacation.jpg"
echo "music track" > "$STORAGE/music/track.mp3"
echo "deep nested file" > "$STORAGE/documents/reports/analysis.csv"

# -- 3. Start server --
echo "[3] Starting server on port $PORT..."
RUST_LOG=warn,ferro_server=info \
    "$SERVER_BIN" \
    --storage "local:$STORAGE" \
    --port "$PORT" \
    --static-dir "$STATIC_DIR" \
    > "$OUTPUT/server.log" 2>&1 &
SERVER_PID=$!
echo "    PID=$SERVER_PID"

# Wait for health
for i in $(seq 1 20); do
    if curl -sf "$BASE_URL/healthz" > /dev/null 2>&1; then
        echo "    Server healthy (${i}s)"
        break
    fi
    sleep 1
done
if ! curl -sf "$BASE_URL/healthz" > /dev/null 2>&1; then
    echo "    FAIL: server not healthy"
    echo "    Server log (last 20 lines):"
    tail -20 "$OUTPUT/server.log"
    exit 1
fi

# -- 4. Verify PROPFIND --
echo "[4] Verifying PROPFIND..."
PROPFIND_N=$(curl -s -X PROPFIND "$BASE_URL/" -H "Depth: 1" | grep -o '<D:href>' | wc -l)
echo "    PROPFIND entries: $PROPFIND_N"
if [ "$PROPFIND_N" -lt 2 ]; then
    echo "    FAIL: PROPFIND not listing children"
    tail -10 "$OUTPUT/server.log"
    exit 1
fi

# -- 5. WASM traversal --
WASM_EXIT=0
if [ "$SKIP_WASM" = false ]; then
    echo "[5] Running WASM traversal..."
    mkdir -p "$OUTPUT/wasm"
    python3 scripts/ferro-traverse.py --mode wasm --url "$BASE_URL" --output "$OUTPUT/wasm" 2>&1 \
        | tee "$OUTPUT/wasm/stdout.log" || WASM_EXIT=$?

    # Parse WASM results
    echo ""
    echo "    WASM Results:"
    OUTPUT_JSON="$OUTPUT/wasm/report.json"
    python3 -c "
import json, sys
try:
    with open('$OUTPUT_JSON') as f:
        d = json.load(f)
    p = sum(1 for r in d['results'] if r['status'] == 'PASS')
    xf = sum(1 for r in d['results'] if r['status'] == 'XFAIL')
    t = len(d['results'])
    print(f'    Passed: {p}/{t} ({100*p//max(t,1)}%)  XFAIL: {xf}')
    for r in d['results']:
        if r['status'] == 'FAIL':
            reason = ''
            if r.get('result') and isinstance(r['result'], dict):
                reason = r['result'].get('reason', r['result'].get('r', ''))
            print(f'    FAIL: [{r[\"section\"]}] {r[\"name\"]} {reason}')
except Exception as e:
    print(f'    Could not parse WASM report: {e}')
"

    echo "    JS Errors: $(python3 -c "import json; d=json.load(open('$OUTPUT/wasm/report.json')); print(len(d.get('errors',[])))" 2>/dev/null || echo '?')"
    echo "    Net Errors: $(python3 -c "import json; d=json.load(open('$OUTPUT/wasm/report.json')); print(len(d.get('network_errors',[])))" 2>/dev/null || echo '?')"
    echo "    CSP Violations: $(python3 -c "import json; d=json.load(open('$OUTPUT/wasm/report.json')); print(len(d.get('csp_violations',[])))" 2>/dev/null || echo '?')"
else
    echo "[5] Skipping WASM traversal"
fi

# -- 6. Desktop traversal --
DESKTOP_EXIT=0
if [ "$RUN_DESKTOP" = true ]; then
    echo "[6] Running Desktop traversal..."
    mkdir -p "$OUTPUT/desktop"

    # Check if desktop app is running
    if xdotool search --name "Ferro" 2>/dev/null | head -1 | grep -q '[0-9]'; then
        python3 scripts/ferro-traverse.py --mode desktop --output "$OUTPUT/desktop" 2>&1 \
            | tee "$OUTPUT/desktop/stdout.log" || DESKTOP_EXIT=$?
    else
        echo "    No Ferro desktop window found. Launching..."
        WEBKIT_DISABLE_DMABUF_RENDERER=1 WAYLAND_DISPLAY= \
            ./target/debug/ferro-desktop \
            --server-url "$BASE_URL" --debug \
            > "$OUTPUT/desktop/app.log" 2>&1 &
        DESKTOP_PID=$!
        sleep 5
        python3 scripts/ferro-traverse.py --mode desktop --output "$OUTPUT/desktop" 2>&1 \
            | tee "$OUTPUT/desktop/stdout.log" || DESKTOP_EXIT=$?
        kill $DESKTOP_PID 2>/dev/null || true
    fi

    # Parse Desktop results
    echo ""
    echo "    Desktop Results:"
    python3 -c "
import json
try:
    with open('$OUTPUT/desktop/report.json') as f:
        d = json.load(f)
    p = sum(1 for r in d['results'] if r['status'] == 'PASS')
    t = len(d['results'])
    print(f'    Passed: {p}/{t} ({100*p//max(t,1)}%)')
except Exception as e:
    print(f'    Could not parse Desktop report: {e}')
"
else
    echo "[6] Skipping Desktop traversal (use --desktop to enable)"
fi

# -- 7. Summary --
echo ""
echo "=========================================="
echo "  INTEGRATION TEST COMPLETE"
echo "=========================================="
echo "  Server log:  $OUTPUT/server.log"
echo "  WASM report:  $OUTPUT/wasm/report.json"
echo "  WASM errors:  $OUTPUT/wasm/errors.log"
echo "  WASM screenshots: $OUTPUT/wasm/"
if [ "$RUN_DESKTOP" = true ]; then
    echo "  Desktop report:    $OUTPUT/desktop/report.json"
    echo "  Desktop screenshots: $OUTPUT/desktop/"
fi
echo ""

# Exit with failure if any traversal failed
if [ "$WASM_EXIT" -ne 0 ] || [ "$RUN_DESKTOP" = true ] && [ "$DESKTOP_EXIT" -ne 0 ]; then
    exit 1
fi
