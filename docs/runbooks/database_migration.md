# Runbook: Database Migration

## Overview

This runbook covers procedures for migrating the Ferro database schema, including schema changes, data migrations, and version upgrades.

## Severity Level

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Production migration failure | Immediate |
| P2 | Migration performance degradation | < 1 hour |
| P3 | Non-urgent schema changes | < 24 hours |

## Prerequisites

- [ ] Database backup completed and verified
- [ ] Migration scripts tested in staging environment
- [ ] Rollback scripts prepared and tested
- [ ] Maintenance window scheduled (if required)
- [ ] Stakeholders notified
- [ ] `ferro-admin` CLI available with admin privileges

## Migration Procedure

### 1. Pre-Migration Checks

```bash
# Verify current schema version
sqlite3 /var/lib/ferro/ferro.db "PRAGMA user_version;"

# Check database integrity
sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"

# Verify disk space
df -h /var/lib/ferro

# Check for active connections
lsof /var/lib/ferro/ferro.db
```

### 2. Stop the Server

```bash
systemctl stop ferro
# Verify it's stopped
systemctl status ferro
```

### 3. Create Backup

```bash
# Checkpoint WAL to main database
sqlite3 /var/lib/ferro/ferro.db "PRAGMA wal_checkpoint(TRUNCATE);"

# Create timestamped backup
cp /var/lib/ferro/ferro.db /var/lib/ferro/ferro.db.pre-migration.$(date +%s)

# Verify backup integrity
sqlite3 /var/lib/ferro/ferro.db.pre-migration.* "PRAGMA integrity_check;"
```

### 4. Run Migration

```bash
# For cargo-based migrations
cargo migrate --database /var/lib/ferro/ferro.db

# Or manual migration
sqlite3 /var/lib/ferro/ferro.db < /path/to/migration.sql
```

### 5. Verify Schema

```bash
# Check new schema version
sqlite3 /var/lib/ferro/ferro.db "PRAGMA user_version;"

# Verify integrity after migration
sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"

# Check table structure
sqlite3 /var/lib/ferro/ferro.db ".schema"
```

### 6. Start Server

```bash
systemctl start ferro
# Verify startup
systemctl status ferro
journalctl -u ferro --since "2 minutes ago" --no-pager
```

### 7. Verify Functionality

```bash
# Health check
curl -f http://localhost:8080/healthz

# Run integration tests
cargo test --test integration

# Check API responses
curl -u admin:password http://localhost:8080/api/v1/status
```

## Rollback Procedure

If migration fails or causes issues:

### 1. Stop the Server

```bash
systemctl stop ferro
```

### 2. Restore from Backup

```bash
# Remove corrupted/migrated database
rm /var/lib/ferro/ferro.db

# Restore from pre-migration backup
cp /var/lib/ferro/ferro.db.pre-migration.<timestamp> /var/lib/ferro/ferro.db
```

### 3. Verify Schema

```bash
# Confirm schema version restored
sqlite3 /var/lib/ferro/ferro.db "PRAGMA user_version;"

# Verify integrity
sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"
```

### 4. Start Server

```bash
systemctl start ferro
systemctl status ferro
```

## Verification Checklist

- [ ] Schema version matches expected value
- [ ] `PRAGMA integrity_check` returns `ok`
- [ ] All tests pass
- [ ] No data loss verified
- [ ] Performance metrics within acceptable range
- [ ] API endpoints responding correctly
- [ ] WebDAV/CalDAV/CardDAV functionality verified

## Escalation

- If migration fails and rollback also fails, escalate immediately to on-call engineering.
- If data loss is suspected, activate disaster recovery plan and notify stakeholders.
- File an issue with full migration logs and error output.

## Contact Information

| Role | Contact |
|------|---------|
| Database Administrator | @db-admin |
| On-Call Engineer | @oncall |
| Engineering Lead | @eng-lead |
