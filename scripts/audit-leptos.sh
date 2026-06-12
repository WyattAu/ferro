#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

SERVER=target/debug/ferro-server
PORT=8081
RESULTS_DIR=target/audit-results/leptos

# Build if needed
if [ ! -f "$SERVER" ]; then
  echo "Building server..."
  cargo build -p ferro-server --bin ferro-server
fi

# Clean results
rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR"

# Start server
echo "Starting server on port $PORT..."
"$SERVER" --host 127.0.0.1 --port "$PORT" --static-dir crates/web/dist &
SPID=$!
trap "kill $SPID 2>/dev/null" EXIT

# Wait for server to be ready
echo "Waiting for server..."
for i in $(seq 1 30); do
  sleep 1
  if curl -sf http://127.0.0.1:$PORT/.well-known/ferro >/dev/null 2>&1; then
    echo "Server ready after ${i}s"
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo "Server did not start in time"
    exit 1
  fi
done

# Run the audit
echo "Running deep audit..."
BASE_URL="http://127.0.0.1:$PORT" timeout 300 node e2e/deep-audit.js 2>&1 || AUDIT_EXIT=$?

# Print the report
echo ""
echo "══════════════════════════════════════════════════════"
echo "  AUDIT REPORT"
echo "══════════════════════════════════════════════════════"
echo ""
if [ -f "$RESULTS_DIR/REPORT.md" ]; then
  cat "$RESULTS_DIR/REPORT.md"
else
  echo "Report not found at $RESULTS_DIR/REPORT.md"
fi
echo ""
echo "══════════════════════════════════════════════════════"
echo "  Screenshots saved to $RESULTS_DIR/"
echo "══════════════════════════════════════════════════════"
find "$RESULTS_DIR" -name "*.png" | sort
echo ""

exit ${AUDIT_EXIT:-0}
