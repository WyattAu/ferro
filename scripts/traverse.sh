#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
SERVER=target/debug/ferro-server
PORT=8081

# Build if needed
if [ ! -f "$SERVER" ]; then
  echo "Building server..."
  cargo build -p ferro-server --bin ferro-server
fi

# Clean results
rm -rf target/playwright-results
mkdir -p target/playwright-results

# Start server
echo "Starting server on port $PORT..."
$SERVER --host 127.0.0.1 --port $PORT --static-dir crates/web/dist &
SPID=$!
trap "kill $SPID 2>/dev/null" EXIT

# Wait for server
for i in $(seq 1 30); do
  sleep 1
  if curl -sf http://127.0.0.1:$PORT/.well-known/ferro >/dev/null 2>&1; then
    echo "Server ready"
    break
  fi
done

# Check what's served
echo "Root status: $(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/ 2>/dev/null)"
echo "Root size: $(curl -s http://127.0.0.1:$PORT/ 2>/dev/null | wc -c) bytes"
echo "Root first line: $(curl -s http://127.0.0.1:$PORT/ 2>/dev/null | head -1)"
echo "UI status: $(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/ui/ 2>/dev/null)"
echo "WASM status: $(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/ui/web-dbc0036a7b4ec360_bg.wasm 2>/dev/null)"

# Run capture
BASE_URL="http://127.0.0.1:$PORT" timeout 120 node e2e/capture.js 2>&1 || echo "Capture timed out or failed"

# Print results
echo ""
echo "=== CAPTURE RESULTS ==="
find target/playwright-results -name "*.png" | sort
echo ""
find target/playwright-results -name "summary.json" -exec sh -c 'echo "--- $(dirname "$1" | xargs basename) ---"; cat "$1"' _ {} \;

echo "Done"
