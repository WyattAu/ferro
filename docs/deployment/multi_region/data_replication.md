# Data Replication

## Replication Topology

```
                    ┌─────────────────┐
                    │    US East       │
                    │    (Primary)     │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
     ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
     │   US West     │ │   EU West     │ │  Asia Pacific │
     │   (Replica)   │ │   (Replica)   │ │   (Replica)   │
     └──────────────┘ └──────────────┘ └──────────────┘

Replication path: US East → US West → EU West → Asia Pacific
                  (mesh topology for resilience)
```

### Within Region

- **Primary-Replica**: PostgreSQL streaming replication
- **Synchronous**: For critical data (user accounts, auth)
- **Synchronous commit**: `on` for critical writes, `remote_apply` for cross-region
- **Latency impact**: 1-5ms

### Cross-Region

- **Asynchronous**: WAL shipping via dedicated replication channel
- **Batch size**: 1MB or 1 second (whichever first)
- **Compression**: lz4 for replication traffic
- **Encryption**: TLS 1.3 for all cross-region traffic
- **Latency impact**: 50-200ms (depending on distance)

## Replication Methods

### PostgreSQL Streaming Replication

```sql
-- Primary: Configure for streaming replication
ALTER SYSTEM SET wal_level = 'logical';
ALTER SYSTEM SET max_wal_senders = 10;
ALTER SYSTEM SET wal_keep_size = '1GB';
ALTER SYSTEM SET synchronous_standby_names = 'US-West-1';

-- Replica: Configure for async replication
ALTER SYSTEM SET primary_conninfo = 'host=us-east.ferro.internal port=5432 user=replicator password=...';
ALTER SYSTEM SET hot_standby = on;
```

### S3 Cross-Region Replication

```json
{
  "Rules": [
    {
      "ID": "FerroCrossRegionReplication",
      "Status": "Enabled",
      "Priority": 1,
      "Filter": {
        "Prefix": ""
      },
      "Destination": {
        "Bucket": "arn:aws:s3:::ferro-replica-us-west-2",
        "StorageClass": "STANDARD_IA",
        "ReplicationTime": {
          "Status": "Enabled",
          "Time": {
            "Minutes": 15
          }
        },
        "Metrics": {
          "Status": "Enabled",
          "EventThreshold": {
            "Minutes": 15
          }
        }
      }
    }
  ]
}
```

## Consistency Levels

| Level | Scope | Use Case | Latency |
|-------|-------|----------|---------|
| **Strong** | Within region | User auth, calendar writes | 1-5ms |
| **Eventual** | Cross-region | File metadata, contact sync | 50-200ms |
| **Weak** | Local only | Audit logs, analytics | <1ms |

## Data Types and Replication Strategy

### Strong Consistency Required

| Data Type | Replication | RPO | Notes |
|-----------|-------------|-----|-------|
| User accounts | Synchronous | 0 | Auth-critical |
| Authentication tokens | Synchronous | 0 | Security-critical |
| Calendar events | Synchronous | 0 | User-facing |
| Contact data | Synchronous | 0 | User-facing |
| File metadata | Synchronous | 0 | User-facing |

### Eventual Consistency Acceptable

| Data Type | Replication | RPO | Notes |
|-----------|-------------|-----|-------|
| File content (S3) | Async (15min) | 15 min | Large objects |
| Audit logs | Async (5min) | 5 min | Compliance |
| Analytics events | Async (1min) | 1 min | Non-critical |
| Plugin state | Async (1min) | 1 min | Non-critical |

## Conflict Resolution

### Last-Writer-Wins (LWW)

- Used for: file metadata, non-critical state
- Resolution: latest timestamp wins
- Risk: may lose concurrent writes
- Implementation: PostgreSQL `xmin` or application timestamp

### Vector Clocks

- Used for: collaborative editing, shared calendars
- Resolution: causal ordering preserved
- Risk: requires conflict merge logic
- Implementation: embedded in document metadata

### Manual Resolution

- Used for: critical data conflicts
- Resolution: admin intervention via `ferro-admin`
- Trigger: when automatic resolution fails
- Notification: alert sent to admin

```bash
# List conflicts
ferro-admin conflicts list --region us-east-1

# Resolve conflict (keep version)
ferro-admin conflicts resolve --id <conflict-id> --keep-version <version>

# Force resolution (accept remote)
ferro-admin conflicts resolve --id <conflict-id> --accept-remote
```

## Monitoring Replication

### Metrics to Track

```sql
-- Replication lag (seconds)
SELECT client_addr,
       state,
       sent_lsn,
       replay_lsn,
       replay_lag
FROM pg_stat_replication;

-- WAL generation rate
SELECT pg_current_wal_lsn(),
       pg_walfile_name(pg_current_wal_lsn());

-- Replication slot health
SELECT slot_name,
       active,
       restart_lsn,
       confirmed_flush_lsn,
       pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)) AS retained_wal
FROM pg_replication_slots;
```

### Prometheus Queries

```promql
# Replication lag in seconds
pg_replication_lag_seconds{region!=""}

# WAL generation rate
rate(pg_wal_bytes_total[5m])

# Replication slot retained WAL
pg_replication_slot_retained_bytes

# Failed replication attempts
rate(pg_replication_attempts_failed_total[5m])
```

### Alert Thresholds

| Metric | Warning | Critical |
|--------|---------|----------|
| Replication lag (intra-region) | >1s | >5s |
| Replication lag (cross-region) | >5s | >30s |
| Replication slot inactive | >5min | >30min |
| WAL retained | >500MB | >2GB |
| Replication failure rate | >0.1% | >1% |

## Backup Strategy

### Continuous (WAL Archiving)

- Archive WAL segments to S3
- Retention: 7 days
- Enables point-in-time recovery
- RPO: near-zero

### Daily Full Backups

- Full PostgreSQL dump at 02:00 UTC
- Stored in separate region (us-west-2)
- Retention: 30 days
- Compressed with pg_dump custom format

### Cross-Region Backup

- Replicated to secondary region (eu-west-1)
- Used for disaster recovery
- Retention: 90 days
- Encrypted with regional KMS key

### Backup Verification

```bash
# Verify backup integrity
pg_restore --list /backups/ferro-daily-latest.dump

# Test restore to isolated instance
pg_restore -h test-instance -U ferro -d ferro_test /backups/ferro-daily-latest.dump

# Verify data integrity
psql -h test-instance -U ferro -d ferro_test -c "SELECT count(*) FROM users;"
```

## Replication Lag Handling

### Client-Side

- Read-after-write consistency: read from primary for 5 seconds after write
- Stale read tolerance: configurable per endpoint (default: 60s)
- Conflict detection: return version mismatch errors

### Server-Side

- Lag monitoring: continuous monitoring of replication lag
- Automatic throttling: reduce write throughput if lag exceeds threshold
- Cross-region routing: route reads to lowest-lag replica
