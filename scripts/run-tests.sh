#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

PORT=${1:-8100}
RESULTS_DIR="target/production-results-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo "=== Ferro Production Test Run ===" | tee "$RESULTS_DIR/run.log"
echo "Start: $(date -Iseconds)" | tee -a "$RESULTS_DIR/run.log"
echo "Port: $PORT" | tee -a "$RESULTS_DIR/run.log"

# Start server
echo "Starting server..." | tee -a "$RESULTS_DIR/run.log"
target/debug/ferro-server --host 127.0.0.1 --port "$PORT" --static-dir crates/web/dist > "$RESULTS_DIR/server.log" 2>&1 &
SERVER_PID=$!
echo "Server PID: $SERVER_PID" | tee -a "$RESULTS_DIR/run.log"

# Wait for server
for i in $(seq 1 20); do
    sleep 1
    if curl -sf "http://127.0.0.1:$PORT/.well-known/ferro" >/dev/null 2>&1; then
        echo "Server ready after ${i}s" | tee -a "$RESULTS_DIR/run.log"
        break
    fi
done

BASE="http://127.0.0.1:$PORT"
PASS=0; FAIL=0; T=0; RESULTS=""

check() {
    local name="$1" expected="$2" actual="$3"
    T=$((T+1))
    if [ "$expected" = "$actual" ]; then
        echo "PASS,$name,$expected,$actual" >> "$RESULTS_DIR/results.csv"
        PASS=$((PASS+1))
    else
        echo "FAIL,$name,$expected,$actual" >> "$RESULTS_DIR/results.csv"
        FAIL=$((FAIL+1))
    fi
}

echo "status,name,expected,actual" > "$RESULTS_DIR/results.csv"

echo "=== HEALTH ===" | tee -a "$RESULTS_DIR/run.log"
check "health" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/.well-known/ferro")"
check "healthz" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/healthz")"
check "readyz" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/readyz")"

echo "=== STATIC ===" | tee -a "$RESULTS_DIR/run.log"
check "root" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/")"
check "css" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/ui/style.css")"
check "wasm" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/ui/web-3957003e3b07a813_bg.wasm")"

echo "=== WEBDAV ===" | tee -a "$RESULTS_DIR/run.log"
check "MKCOL" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MKCOL "$BASE/prod-test")"
check "MKCOL2" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MKCOL "$BASE/prod-test/docs")"
check "PUT" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -d 'hello' "$BASE/prod-test/hello.txt")"
check "GET" "hello" "$(curl -sf "$BASE/prod-test/hello.txt")"
check "MOVE" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MOVE -H 'Destination: /prod-test/renamed.txt' "$BASE/prod-test/hello.txt")"
check "GET-old" "404" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/prod-test/hello.txt")"
check "COPY" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X COPY -H 'Destination: /prod-test/copied.txt' "$BASE/prod-test/renamed.txt")"
check "DELETE" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE "$BASE/prod-test/copied.txt")"
check "DELETE-dir" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE "$BASE/prod-test")"

echo "=== API ===" | tee -a "$RESULTS_DIR/run.log"
check "config" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/config")"
check "branding" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/branding")"
check "quota" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/quota")"
check "prefs" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/preferences")"
check "favs" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/favorites")"
check "recent" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/recent")"
check "locks" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/locks")"
check "search" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/search?q=test")"

echo "=== LARGE FILE ===" | tee -a "$RESULTS_DIR/run.log"
dd if=/dev/urandom of=/tmp/prod.bin bs=1M count=10 2>/dev/null
check "PUT-10MB" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -T /tmp/prod.bin "$BASE/large.bin")"
check "GET-10MB" "10485760" "$(curl -sf -I "$BASE/large.bin" 2>/dev/null | grep -i content-length | awk '{print $2}' | tr -d '\r')"
check "DELETE-10MB" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE "$BASE/large.bin")"
rm -f /tmp/prod.bin

echo "=== SECURITY ===" | tee -a "$RESULTS_DIR/run.log"
check "no-shell" "404" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/bin/sh")"

echo "" | tee -a "$RESULTS_DIR/run.log"
echo "RESULTS: $PASS PASS / $FAIL FAIL / $T TOTAL" | tee -a "$RESULTS_DIR/run.log"

# Save server log
cp "$RESULTS_DIR/server.log" "$RESULTS_DIR/server-final.log"

echo "End: $(date -Iseconds)" | tee -a "$RESULTS_DIR/run.log"
echo "Results: $RESULTS_DIR/" | tee -a "$RESULTS_DIR/run.log"
