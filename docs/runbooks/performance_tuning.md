# Runbook: Performance Tuning

## Overview

This runbook provides procedures for diagnosing and resolving performance issues in Ferro, including database optimization, memory tuning, and connection management.

## Severity Level

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Service degradation, timeouts | Immediate |
| P2 | Slow response times | < 1 hour |
| P3 | Optimization opportunities | < 24 hours |

## Prerequisites

- [ ] Access to monitoring dashboards
- [ ] Performance baseline established
- [ ] System metrics collection enabled
- [ ] Database profiling available

## Diagnosis

### 1. Check System Resources

```bash
# CPU and memory usage
top -bn1 | head -20

# Memory details
free -m

# Disk I/O
iostat -x 1 5

# Network connections
ss -s
```

### 2. Check Ferro Metrics

```bash
# Health endpoint with metrics
curl -s http://localhost:8080/healthz | jq .

# Connection count
ss -tlnp | grep ferro | wc -l

# Process metrics
ps aux | grep ferro
cat /proc/<pid>/status | grep -E "VmRSS|VmSize|Threads"
```

### 3. Database Performance

```bash
# Check WAL mode and size
ls -lh /var/lib/ferro/ferro.db*

# SQLite statistics
sqlite3 /var/lib/ferro/ferro.db "PRAGMA stats;"

# Check for slow queries (if logging enabled)
grep -i "slow\|query" /var/log/ferro/*.log | tail -50
```

### 4. Application Logs

```bash
# Recent errors
journalctl -u ferro --since "1 hour ago" --no-pager | grep -i "error\|timeout\|slow"

# Request latency
grep "request_time\|duration" /var/log/ferro/access.log | tail -100
```

## Performance Tuning

### Database Optimization

```bash
# Enable WAL mode (if not already)
sqlite3 /var/lib/ferro/ferro.db "PRAGMA journal_mode=WAL;"

# Optimize page size
sqlite3 /var/lib/ferro/ferro.db "PRAGMA page_size=4096;"

# Enable memory-mapped I/O (1GB)
sqlite3 /var/lib/ferro/ferro.db "PRAGMA mmap_size=268435456;"

# Set cache size (64MB)
sqlite3 /var/lib/ferro/ferro.db "PRAGMA cache_size=-65536;"

# Optimize temp store
sqlite3 /var/lib/ferro/ferro.db "PRAGMA temp_store=MEMORY;"
```

### Connection Pool Tuning

Edit `/etc/ferro/ferro.toml`:

```toml
[server]
max_connections = 200
worker_threads = 8

[storage]
pool_size = 20
pool_timeout = 30

[database]
busy_timeout = 5000
wal_autocheckpoint = 1000
```

### Memory Configuration

```toml
[server]
# Increase if handling large uploads/downloads
max_request_body_size = "100MB"

[wasm]
# Limit WASM memory per module
max_memory_pages = 256

[cache]
# Enable and size caching
enabled = true
max_size = "1GB"
ttl = "1h"
```

### Network Tuning

```bash
# Increase file descriptor limits
ulimit -n 65536

# TCP tuning
sysctl -w net.core.somaxconn=65535
sysctl -w net.ipv4.tcp_max_syn_backlog=65535
sysctl -w net.ipv4.ip_local_port_range="1024 65535"
```

## Monitoring Commands

### Real-time Metrics

```bash
# Watch connections
watch -n 1 "ss -tlnp | grep ferro | wc -l"

# Watch memory
watch -n 1 "ps aux | grep ferro | grep -v grep"

# Watch logs
tail -f /var/log/ferro/access.log
```

### Load Testing

```bash
# Using hey (Go-based load testing tool)
hey -n 10000 -c 100 http://localhost:8080/healthz

# Using wrk
wrk -t4 -c100 -d30s http://localhost:8080/healthz

# Using Apache Bench
ab -n 10000 -c 100 http://localhost:8080/healthz
```

## Performance Checklist

- [ ] Database in WAL mode
- [ ] Connection pool sized appropriately
- [ ] Memory limits configured
- [ ] File descriptor limits increased
- [ ] Monitoring enabled
- [ ] Baseline metrics established
- [ ] Load testing completed

## Common Issues and Solutions

### High Memory Usage

```bash
# Check for memory leaks
valgrind --leak-check=full ./ferro

# Reduce worker threads
# In ferro.toml:
# [server]
# worker_threads = 4
```

### Slow Database Queries

```bash
# Enable query logging
sqlite3 /var/lib/ferro/ferro.db "PRAGMA vdbe_debug=1;"

# Analyze query plans
EXPLAIN QUERY PLAN SELECT * FROM your_table;
```

### Connection Exhaustion

```bash
# Check connection count
ss -tlnp | grep ferro | wc -l

# Increase pool size or reduce timeout
# In ferro.toml:
# [storage]
# pool_size = 30
# pool_timeout = 60
```

## Escalation

- If performance issues persist after tuning, escalate to engineering lead.
- If database performance is critical, escalate to database administrator.
- If memory issues persist, escalate to infrastructure team.

## Contact Information

| Role | Contact |
|------|---------|
| On-Call Engineer | @oncall |
| Engineering Lead | @eng-lead |
| Database Administrator | @db-admin |
| Infrastructure Lead | @infra-lead |
