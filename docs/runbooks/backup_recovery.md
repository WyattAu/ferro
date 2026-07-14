# Runbook: Backup and Recovery

## Overview

This runbook covers backup procedures, recovery operations, and disaster recovery for Ferro database and configuration.

## Severity Level

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Data loss, backup failure | Immediate |
| P2 | Recovery required, corruption detected | < 1 hour |
| P3 | Backup verification failed | < 24 hours |

## Prerequisites

- [ ] Backup storage location accessible
- [ ] Sufficient disk space for backups
- [ ] Backup scripts available and tested
- [ ] Recovery environment ready
- [ ] Encryption keys available (if encrypted backups)

## Backup Procedures

### Full Database Backup

```bash
# Stop server for consistent backup (optional for SQLite with WAL)
systemctl stop ferro

# Checkpoint WAL to main database
sqlite3 /var/lib/ferro/ferro.db "PRAGMA wal_checkpoint(TRUNCATE);"

# Create backup with timestamp
BACKUP_PATH="/var/lib/ferro/backups/ferro-$(date +%Y%m%d-%H%M%S).db"
cp /var/lib/ferro/ferro.db "$BACKUP_PATH"

# Verify backup integrity
sqlite3 "$BACKUP_PATH" "PRAGMA integrity_check;"

# Calculate checksum for verification
sha256sum "$BACKUP_PATH" > "${BACKUP_PATH}.sha256"

# Restart server
systemctl start ferro
```

### Configuration Backup

```bash
# Backup configuration files
tar czf /var/lib/ferro/backups/ferro-config-$(date +%Y%m%d-%H%M%S).tar.gz \
  /etc/ferro/ferro.toml \
  /etc/ferro/*.pem \
  /etc/ferro/*.key

# Backup environment variables
env | grep -i "FERRO_\|DATABASE_\|STORAGE_" > /var/lib/ferro/backups/env-$(date +%Y%m%d-%H%M%S).txt
```

### Automated Backup Script

Create `/usr/local/bin/ferro-backup.sh`:

```bash
#!/bin/bash
set -euo pipefail

BACKUP_DIR="/var/lib/ferro/backups"
RETENTION_DAYS=30
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Database backup
sqlite3 /var/lib/ferro/ferro.db "PRAGMA wal_checkpoint(TRUNCATE);"
cp /var/lib/ferro/ferro.db "$BACKUP_DIR/ferro-$TIMESTAMP.db"
sqlite3 "$BACKUP_DIR/ferro-$TIMESTAMP.db" "PRAGMA integrity_check;" | grep -q "ok"

# Configuration backup
tar czf "$BACKUP_DIR/ferro-config-$TIMESTAMP.tar.gz" /etc/ferro/

# Cleanup old backups
find "$BACKUP_DIR" -name "*.db" -mtime +$RETENTION_DAYS -delete
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +$RETENTION_DAYS -delete

echo "Backup completed: $TIMESTAMP"
```

### Schedule Automated Backups

```bash
# Add to crontab
crontab -e

# Daily backup at 2 AM
0 2 * * * /usr/local/bin/ferro-backup.sh >> /var/log/ferro-backup.log 2>&1
```

## Recovery Procedures

### Recovery from Backup

#### 1. Stop the Server

```bash
systemctl stop ferro
```

#### 2. Verify Backup Integrity

```bash
# Find most recent backup
ls -lt /var/lib/ferro/backups/ferro-*.db | head -1

# Verify integrity
sqlite3 /var/lib/ferro/backups/ferro-<timestamp>.db "PRAGMA integrity_check;"

# Verify checksum
sha256sum -c /var/lib/ferro/backups/ferro-<timestamp>.db.sha256
```

#### 3. Restore Database

```bash
# Backup current (potentially corrupted) database
cp /var/lib/ferro/ferro.db /var/lib/ferro/ferro.db.failed-$(date +%s)

# Restore from backup
cp /var/lib/ferro/backups/ferro-<timestamp>.db /var/lib/ferro/ferro.db

# Verify restored database
sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"
sqlite3 /var/lib/ferro/ferro.db "PRAGMA user_version;"
```

#### 4. Restore Configuration (if needed)

```bash
# Find configuration backup
ls -lt /var/lib/ferro/backups/ferro-config-*.tar.gz | head -1

# Restore configuration
tar xzf /var/lib/ferro/backups/ferro-config-<timestamp>.tar.gz -C /
```

#### 5. Start and Verify

```bash
# Start server
systemctl start ferro

# Verify startup
systemctl status ferro
journalctl -u ferro --since "2 minutes ago" --no-pager

# Health check
curl -f http://localhost:8080/healthz

# Verify data
ferro-admin status
```

### Point-in-Time Recovery

```bash
# Stop server
systemctl stop ferro

# Restore to specific backup
cp /var/lib/ferro/backups/ferro-<desired-timestamp>.db /var/lib/ferro/ferro.db

# Verify
sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"

# Start server
systemctl start ferro
```

## Verification Checklist

- [ ] Backup integrity verified (`PRAGMA integrity_check` returns `ok`)
- [ ] Checksum matches
- [ ] Backup size reasonable
- [ ] Server starts successfully after restore
- [ ] All services responding
- [ ] Data verified (check key records)
- [ ] No errors in logs

## Disaster Recovery

### RPO and RTO Targets

| Metric | Target |
|--------|--------|
| RPO (Recovery Point Objective) | 24 hours (daily backups) |
| RTO (Recovery Time Objective) | 1 hour |

### Offsite Backup

```bash
# Sync to remote storage (S3 example)
aws s3 sync /var/lib/ferro/backups/ s3://ferro-backups/$(hostname)/ \
  --storage-class STANDARD_IA

# Or rsync to remote server
rsync -avz /var/lib/ferro/backups/ backup-server:/ferro/backups/$(hostname)/
```

## Escalation

- If backup fails and cannot be resolved, escalate to on-call engineering.
- If recovery fails, escalate to database administrator.
- If data loss is confirmed, activate full disaster recovery plan.

## Contact Information

| Role | Contact |
|------|---------|
| On-Call Engineer | @oncall |
| Database Administrator | @db-admin |
| Infrastructure Lead | @infra-lead |
| Engineering Lead | @eng-lead |
