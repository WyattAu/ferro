#!/usr/bin/env bash
set -euo pipefail

# Configuration
PORT=${PORT:-8090}
DATA_DIR=${DATA_DIR:-/tmp/ferro-prod-test}
ADMIN_USER=${ADMIN_USER:-admin}
ADMIN_PASSWORD=${ADMIN_PASSWORD:-testpass123}
STATIC_DIR=${STATIC_DIR:-crates/web/dist}
RESULTS_DIR=${RESULTS_DIR:-target/production-test-results}

# Clean previous run
rm -rf "$DATA_DIR" "$RESULTS_DIR"
mkdir -p "$DATA_DIR" "$RESULTS_DIR"

echo "=== Ferro Production Test Suite ==="
echo "Port: $PORT"
echo "Data: $DATA_DIR"
echo "Results: $RESULTS_DIR"
echo ""

# Build if needed
if [ ! -f target/debug/ferro-server ]; then
    echo "Building server..."
    cargo build -p ferro-server --bin ferro-server
fi

# Start server
echo "Starting server..."
target/debug/ferro-server \
    --host 127.0.0.1 \
    --port "$PORT" \
    --data-dir "$DATA_DIR" \
    --storage "local:$DATA_DIR/data" \
    --static-dir "$STATIC_DIR" \
    --admin-user "$ADMIN_USER" \
    --admin-password "$ADMIN_PASSWORD" &
SERVER_PID=$!
trap "kill $SERVER_PID 2>/dev/null; wait $SERVER_PID" EXIT

# Wait for server (longer with auth - bcrypt is slow in debug)
echo "Waiting for server..."
for i in $(seq 1 120); do
    sleep 1
    if curl -sf "http://127.0.0.1:$PORT/.well-known/ferro" >/dev/null 2>&1; then
        echo "Server ready after ${i}s"
        break
    fi
done

# Test functions
PASS=0; FAIL=0; TOTAL=0
RESULTS=""

check() {
    local name="$1" expected="$2" actual="$3" duration="$4"
    TOTAL=$((TOTAL+1))
    if [ "$expected" = "$actual" ]; then
        echo "  PASS: $name (${duration}ms)"
        PASS=$((PASS+1))
        RESULTS="$RESULTS\nPASS,$name,$expected,$actual,$duration"
    else
        echo "  FAIL: $name (expected=$expected actual=$actual)"
        FAIL=$((FAIL+1))
        RESULTS="$RESULTS\nFAIL,$name,$expected,$actual,$duration"
    fi
}

measure() {
    local start=$(date +%s%N)
    eval "$1" >/dev/null 2>&1
    local end=$(date +%s%N)
    echo $(( (end - start) / 1000000 ))
}

echo ""
echo "=== 1. Health & Discovery ==="
D=$(measure "curl -sf http://127.0.0.1:$PORT/.well-known/ferro -o /dev/null -w '%{http_code}'")
check "health endpoint" "200" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/.well-known/ferro)" "$D"

echo ""
echo "=== 2. WebDAV Operations ==="
# Create test structure
D=$(measure "curl -sf -X MKCOL http://127.0.0.1:$PORT/prod-test -o /dev/null -w '%{http_code}'")
check "MKCOL root folder" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MKCOL http://127.0.0.1:$PORT/prod-test)" "$D"

D=$(measure "curl -sf -X MKCOL http://127.0.0.1:$PORT/prod-test/docs -o /dev/null -w '%{http_code}'")
check "MKCOL subfolder" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MKCOL http://127.0.0.1:$PORT/prod-test/docs)" "$D"

# Upload files
echo "Uploading test files..."
for i in $(seq 1 10); do
    dd if=/dev/urandom of="/tmp/testfile_$i.bin" bs=1K count=$((i*10)) 2>/dev/null
done

D=$(measure "curl -sf -X PUT -T /tmp/testfile_1.bin http://127.0.0.1:$PORT/prod-test/file_1KB.bin -o /dev/null -w '%{http_code}'")
check "PUT 10KB file" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -T /tmp/testfile_1.bin http://127.0.0.1:$PORT/prod-test/file_1KB.bin)" "$D"

D=$(measure "curl -sf -X PUT -T /tmp/testfile_5.bin http://127.0.0.1:$PORT/prod-test/file_50KB.bin -o /dev/null -w '%{http_code}'")
check "PUT 50KB file" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -T /tmp/testfile_5.bin http://127.0.0.1:$PORT/prod-test/file_50KB.bin)" "$D"

D=$(measure "curl -sf -X PUT -T /tmp/testfile_10.bin http://127.0.0.1:$PORT/prod-test/file_100KB.bin -o /dev/null -w '%{http_code}'")
check "PUT 100KB file" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -T /tmp/testfile_10.bin http://127.0.0.1:$PORT/prod-test/file_100KB.bin)" "$D"

# Upload large file
echo "Uploading 10MB file..."
dd if=/dev/urandom of=/tmp/testfile_large.bin bs=1M count=10 2>/dev/null
D=$(measure "curl -sf -X PUT -T /tmp/testfile_large.bin http://127.0.0.1:$PORT/prod-test/file_10MB.bin -o /dev/null -w '%{http_code}'")
check "PUT 10MB file" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X PUT -T /tmp/testfile_large.bin http://127.0.0.1:$PORT/prod-test/file_10MB.bin)" "$D"

# Download files
D=$(measure "curl -sf http://127.0.0.1:$PORT/prod-test/file_1KB.bin -o /dev/null -w '%{http_code}'")
check "GET 10KB file" "200" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/prod-test/file_1KB.bin)" "$D"

# Verify content
CONTENT=$(curl -sf http://127.0.0.1:$PORT/prod-test/file_1KB.bin | wc -c)
check "GET 10KB content size" "10240" "$CONTENT" "0"

# MOVE
D=$(measure "curl -sf -X MOVE -H 'Destination: /prod-test/renamed_1KB.bin' http://127.0.0.1:$PORT/prod-test/file_1KB.bin -o /dev/null -w '%{http_code}'")
check "MOVE rename" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X MOVE -H 'Destination: /prod-test/renamed_1KB.bin' http://127.0.0.1:$PORT/prod-test/file_1KB.bin)" "$D"

# COPY
D=$(measure "curl -sf -X COPY -H 'Destination: /prod-test/copy_1KB.bin' http://127.0.0.1:$PORT/prod-test/renamed_1KB.bin -o /dev/null -w '%{http_code}'")
check "COPY file" "201" "$(curl -sf -o /dev/null -w '%{http_code}' -X COPY -H 'Destination: /prod-test/copy_1KB.bin' http://127.0.0.1:$PORT/prod-test/renamed_1KB.bin)" "$D"

# DELETE
D=$(measure "curl -sf -X DELETE http://127.0.0.1:$PORT/prod-test/copy_1KB.bin -o /dev/null -w '%{http_code}'")
check "DELETE file" "204" "$(curl -sf -o /dev/null -w '%{http_code}' -X DELETE http://127.0.0.1:$PORT/prod-test/copy_1KB.bin)" "$D"

echo ""
echo "=== 3. API Endpoints ==="
D=$(measure "curl -sf http://127.0.0.1:$PORT/api/config -o /dev/null -w '%{http_code}'")
check "GET /api/config" "200" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/api/config)" "$D"

D=$(measure "curl -sf http://127.0.0.1:$PORT/api/search?q=test -o /dev/null -w '%{http_code}'")
check "GET /api/search" "200" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/api/search?q=test)" "$D"

echo ""
echo "=== 4. Performance Tests ==="
echo "Running throughput test (50 sequential PUTs)..."
START=$(date +%s%N)
for i in $(seq 1 50); do
    dd if=/dev/urandom of="/tmp/perf_$i.bin" bs=1K count=100 2>/dev/null
    curl -sf -X PUT -T "/tmp/perf_$i.bin" "http://127.0.0.1:$PORT/prod-test/perf_$i.bin" -o /dev/null
done
END=$(date +%s%N)
THROUGHPUT_MS=$(( (END - START) / 1000000 ))
echo "  50 PUTs completed in ${THROUGHPUT_MS}ms"
echo "  Average: $((THROUGHPUT_MS / 50))ms per PUT"

echo ""
echo "=== 5. Concurrent Test ==="
echo "Running 10 concurrent PUTs..."
START=$(date +%s%N)
for i in $(seq 1 10); do
    curl -sf -X PUT -d "concurrent data $i" "http://127.0.0.1:$PORT/prod-test/concurrent_$i.txt" -o /dev/null &
done
wait
END=$(date +%s%N)
CONCURRENT_MS=$(( (END - START) / 1000000 ))
echo "  10 concurrent PUTs completed in ${CONCURRENT_MS}ms"

echo ""
echo "=== 6. Directory Listing ==="
DIR_CONTENT=$(curl -sf -X PROPFIND -H "Depth: 1" "http://127.0.0.1:$PORT/prod-test/" | grep -c "href")
check "PROPFIND lists files" "true" "$([ $DIR_CONTENT -gt 0 ] && echo "true" || echo "false")" "0"

echo ""
echo "=== 7. Static File Serving ==="
D=$(measure "curl -sf http://127.0.0.1:$PORT/ -o /dev/null -w '%{http_code}'")
check "Root serves HTML" "200" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/)" "$D"

echo ""
echo "=== 8. Security ==="
check "No shell access" "404" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/bin/sh)" "0"
check "No package manager" "404" "$(curl -sf -o /dev/null -w '%{http_code}' http://127.0.0.1:$PORT/usr/bin/apt)" "0"

echo ""
echo "=== Cleanup ==="
rm -f /tmp/testfile_*.bin /tmp/testfile_large.bin /tmp/perf_*.bin

echo ""
echo "================================"
echo "PRODUCTION TEST RESULTS: $PASS PASS / $FAIL FAIL / $TOTAL TOTAL"
echo "================================"

# Write CSV report
echo "status,name,expected,actual,duration_ms" > "$RESULTS_DIR/report.csv"
echo -e "$RESULTS" >> "$RESULTS_DIR/report.csv"
echo "Report saved to $RESULTS_DIR/report.csv"
