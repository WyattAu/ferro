# Performance Optimization Guide

## Overview

This document describes performance optimization techniques used in Ferro.

## Caching

### In-Memory Cache
- TTL-based expiration
- LRU eviction
- Thread-safe access
- Statistics tracking

### Cache Strategy
- User data: 5-minute TTL
- Calendar data: 10-minute TTL
- Event data: 5-minute TTL
- Maximum 100K entries

### Cache Configuration
```toml
[cache]
enabled = true
default_ttl = "5m"
max_size = 100000
eviction_policy = "lru"
```

## Connection Pooling

### Pool Configuration
- Maximum connections: 100
- Minimum connections: 10
- Connection timeout: 30s
- Idle timeout: 5m

### Pool Statistics
- Total acquired
- Total released
- Active connections
- Idle connections
- Average wait time

## Query Optimization

### Query Planning
- Cost estimation
- Index selection
- Query rewriting
- Plan caching

### Index Strategy
- Primary keys
- Foreign keys
- Search columns
- Date columns

### Query Statistics
- Total queries
- Cache hits/misses
- Average execution time
- Slow query logging

## Profiling

### CPU Profiling
```bash
# Generate flamegraph
cargo flamegraph --bin ferro-server

# Analyze with perf
perf record -g target/release/ferro-server
perf report
```

### Memory Profiling
```bash
# Valgrind
cargo valgrind

# Heaptrack
heaptrack target/release/ferro-server
```

### I/O Profiling
```bash
# strace
strace -c target/release/ferro-server

# ltrace
ltrace -c target/release/ferro-server
```

## Optimization Techniques

### Algorithm Optimization
- Use efficient data structures
- Reduce algorithmic complexity
- Cache expensive computations
- Parallelize independent operations

### Memory Optimization
- Reduce allocations
- Reuse buffers
- Use memory pools
- Avoid memory leaks

### I/O Optimization
- Batch operations
- Use async I/O
- Reduce syscalls
- Use direct I/O

### Network Optimization
- Connection pooling
- Request batching
- Compression
- Keep-alive

## Benchmarks

### Running Benchmarks
```bash
# Run all benchmarks
./scripts/benchmark.sh

# Run specific benchmark
cargo bench --package ferro-benchmarks --bench crypto
```

### Benchmark Results
- Content hash: 1ms (1KB), 50ms (1MB)
- iCal parse: 10us (small), 500us (medium)
- Storage write: 1ms (1KB), 10ms (1MB)

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| p50 latency | <10ms | 9.27ms |
| p99 latency | <100ms | 1.55s |
| Throughput | >1000 req/s | 48 req/s |
| Memory usage | <512MB | ~256MB |
| CPU usage | <50% | ~30% |
