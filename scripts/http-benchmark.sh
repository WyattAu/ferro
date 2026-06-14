#!/bin/bash
BASE="http://127.0.0.1:8090"

echo "=== HTTP Performance Benchmark ==="
echo "Date: $(date)"
echo ""

# Sequential PUT (1KB)
echo "Sequential PUT (1KB, 10 ops)..."
START=$(date +%s%N)
for i in $(seq 1 10); do
    curl -sf -X PUT -d "test_data_$i" "$BASE/bench/file_$i.txt" -o /dev/null
done
END=$(date +%s%N)
PUT_SEQ_MS=$(( (END - START) / 1000000 ))
echo "  Time: ${PUT_SEQ_MS}ms"
echo "  Avg: $(( PUT_SEQ_MS / 10 ))ms/op"

# Sequential GET (1KB)
echo "Sequential GET (1KB, 10 ops)..."
START=$(date +%s%N)
for i in $(seq 1 10); do
    curl -sf "$BASE/bench/file_1.txt" -o /dev/null
done
END=$(date +%s%N)
GET_SEQ_MS=$(( (END - START) / 1000000 ))
echo "  Time: ${GET_SEQ_MS}ms"
echo "  Avg: $(( GET_SEQ_MS / 10 ))ms/op"

# Sequential DELETE
echo "Sequential DELETE (10 ops)..."
START=$(date +%s%N)
for i in $(seq 1 10); do
    curl -sf -X DELETE "$BASE/bench/file_$i.txt" -o /dev/null
done
END=$(date +%s%N)
DEL_SEQ_MS=$(( (END - START) / 1000000 ))
echo "  Time: ${DEL_SEQ_MS}ms"
echo "  Avg: $(( DEL_SEQ_MS / 10 ))ms/op"

# Medium file (100KB)
echo "Medium file PUT (100KB)..."
dd if=/dev/urandom of=/tmp/bench_100k.bin bs=1K count=100 2>/dev/null
START=$(date +%s%N)
curl -sf -X PUT -T /tmp/bench_100k.bin "$BASE/bench/medium.bin" -o /dev/null
END=$(date +%s%N)
MED_MS=$(( (END - START) / 1000000 ))
echo "  Time: ${MED_MS}ms"
rm -f /tmp/bench_100k.bin

echo ""
echo "=== Summary ==="
echo "Sequential PUT: ${PUT_SEQ_MS}ms total, $(( PUT_SEQ_MS / 10 ))ms/op"
echo "Sequential GET: ${GET_SEQ_MS}ms total, $(( GET_SEQ_MS / 10 ))ms/op"
echo "Sequential DELETE: ${DEL_SEQ_MS}ms total, $(( DEL_SEQ_MS / 10 ))ms/op"
echo "Medium file (100KB): ${MED_MS}ms"
echo "=== Benchmark Complete ==="
