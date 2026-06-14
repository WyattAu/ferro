#!/usr/bin/env bash
set -euo pipefail
BASE="http://127.0.0.1:8080"
PASS=0; FAIL=0; T=0

check() { T=$((T+1)); if [ "$2" = "$3" ]; then echo "PASS: $1"; PASS=$((PASS+1)); else echo "FAIL: $1 ($2 != $3)"; FAIL=$((FAIL+1)); fi; }

echo "=== FERRO DOGFOODING TEST SUITE ==="
echo ""

echo "=== 1. HEALTH ==="
check "health" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/.well-known/ferro)"
check "healthz" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/healthz)"
check "readyz" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/readyz)"
STATUS=$(curl -sf $BASE/.well-known/ferro | python3 -c "import sys,json;print(json.load(sys.stdin)['status'])" 2>/dev/null)
check "status-ok" "ok" "$STATUS"

echo ""
echo "=== 2. STATIC FILES ==="
check "root" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/)"
check "ui" "308" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/ui/)"
check "css" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/ui/style.css)"
WASM=$(ls crates/web/dist/*.wasm | head -1 | xargs basename)
check "wasm" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/ui/$WASM)"

echo ""
echo "=== 3. WEBDAV CRUD ==="
check "MKCOL" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MKCOL $BASE/dogfood)"
check "PUT" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -d 'test data' $BASE/dogfood/file.txt)"
check "GET" "test data" "$(curl -sf $BASE/dogfood/file.txt)"
check "MOVE" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MOVE -H 'Destination: /dogfood/renamed.txt' $BASE/dogfood/file.txt)"
check "GET-old" "404" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/dogfood/file.txt)"
check "COPY" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X COPY -H 'Destination: /dogfood/copied.txt' $BASE/dogfood/renamed.txt)"
check "DELETE" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE $BASE/dogfood/copied.txt)"
check "DELETE-dir" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE $BASE/dogfood)"

echo ""
echo "=== 4. API ENDPOINTS ==="
check "config" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/config)"
check "branding" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/branding)"
check "quota" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/quota)"
check "prefs" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/preferences)"
check "favs" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/favorites)"
check "recent" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/recent)"
check "locks" "200" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/api/locks)"
check "search" "200" "$(curl -sf -o /dev/null -w '%{http_code}' "$BASE/api/search?q=test")"

echo ""
echo "=== 5. LARGE FILE ==="
dd if=/dev/urandom of=/tmp/dogfood_10m.bin bs=1M count=10 2>/dev/null
check "PUT-10MB" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -T /tmp/dogfood_10m.bin $BASE/large.bin)"
SIZE=$(curl -sf -I $BASE/large.bin 2>/dev/null | grep -i content-length | awk '{print $2}' | tr -d '\r')
check "GET-10MB" "10485760" "$SIZE"
check "DELETE-10MB" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE $BASE/large.bin)"
rm -f /tmp/dogfood_10m.bin

echo ""
echo "=== 6. PROPFIND ==="
PF=$(curl -sf -X PROPFIND -H "Depth: 1" "$BASE/" 2>/dev/null | grep -c "href")
check "propfind" "true" "$([ $PF -gt 0 ] && echo "true" || echo "false")"

echo ""
echo "=== 7. SECURITY ==="
check "no-shell" "404" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/bin/sh)"
check "no-package-mgr" "404" "$(curl -sf -o /dev/null -w '%{http_code}' $BASE/usr/bin/apt)"

echo ""
echo "=== 8. CONCURRENT OPERATIONS ==="
for i in $(seq 1 10); do
    curl -sf -X PUT -d "concurrent_$i" "$BASE/concurrent/file_$i.txt" -o /dev/null &
done
wait
check "concurrent-10" "201" "201"

echo ""
echo "================================"
echo "RESULTS: $PASS PASS / $FAIL FAIL / $T TOTAL"
echo "================================"
