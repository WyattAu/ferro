# Runbook: Memory Exhaustion

## Symptoms

- Process RSS grows continuously over hours/days
- `OOMKilled` in container exit status
- `Cannot allocate memory` errors in logs
- Swap usage spikes on the host

## Diagnosis

1. **Check current memory usage**
   ```bash
   ps aux --sort=-%mem | head -20
   # or per-process
   cat /proc/<pid>/status | grep -i "vmsize\|vmrss"
   ```

2. **Collect a heap profile (heaptrack or jemalloc)**
   ```bash
   # If using jemalloc profiling:
   MALLOC_CONF=prof:true PROF_PREFIX=/tmp/ferro-heap ./ferro
   jeprof --show_bytes /tmp/ferro-heap.*
   ```

3. **Check for known memory-hungry operations**
   - Large `PROPFIND` responses (1000+ items in a collection)
   - Concurrent bulk uploads/downloads
   - WebSocket connections with large message buffers
   - WASM module execution without resource limits

4. **Check connection pool sizing**
   ```bash
   grep -i "pool\|max_conn\|worker" /etc/ferro/ferro.toml
   ```

## Mitigation

- **Reduce concurrency**: Lower `[server.max_connections]` and `[server.worker_threads]` in `ferro.toml`.
- **Limit PROPFIND depth**: Enforce `Depth: 1` on large collections at the reverse proxy.
- **Set WASM memory limits**: Configure `[wasm.max_memory_pages]` in `ferro.toml`.
- **Enable connection pooling**: Tune `[storage.pool_size]` for the active backend.
- **For containers**: Set `memory` limit in Docker/Kubernetes spec; use `--oom-kill-disable=false`.

## Long-term

- Profile under expected production load before deploying.
- Add memory budget tests in CI.
- Consider moving to a streaming model for large XML responses.
