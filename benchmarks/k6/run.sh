#!/usr/bin/env bash
set -euo pipefail

FERRO_URL="${FERRO_URL:-http://localhost:9999}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Ferro Load Test Suite ==="
echo "Target: $FERRO_URL"
echo ""

# Check if k6 is installed
if ! command -v k6 &>/dev/null; then
    echo "Installing k6..."
    sudo gpg -k
    sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
    echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
    sudo apt-get update && sudo apt-get install k6 -y
fi

echo "1. REST API load test..."
k6 run "$SCRIPT_DIR/rest-load.js" \
  --env FERRO_URL="$FERRO_URL" \
  --summary-trend-stats="avg,min,med,max,p(90),p(95),p(99)"

echo ""
echo "2. WebDAV load test..."
k6 run "$SCRIPT_DIR/webdav-load.js" \
  --env FERRO_URL="$FERRO_URL" \
  --summary-trend-stats="avg,min,med,max,p(90),p(95),p(99)"

echo ""
echo "=== Load test complete ==="
