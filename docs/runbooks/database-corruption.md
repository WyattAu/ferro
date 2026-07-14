# Runbook: Database Corruption

## Symptoms

- `SQLITE_CORRUPT` or `SQLITE_NOTADB` errors in logs
- Queries return unexpected results or fail
- `cargo` migrations fail with integrity errors

## Diagnosis

1. **Verify integrity**
   ```bash
   sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"
   ```

2. **Check WAL state**
   ```bash
   ls -la /var/lib/ferro/ferro.db-wal /var/lib/ferro/ferro.db-shm
   ```

3. **Check recent write failures**
   ```bash
   grep -i "disk\|corrupt\|io error" /var/log/ferro/*.log
   ```

## Backup and Restore

1. **Stop the server**
   ```bash
   systemctl stop ferro
   ```

2. **Checkpoint WAL before backup**
   ```bash
   sqlite3 /var/lib/ferro/ferro.db "PRAGMA wal_checkpoint(TRUNCATE);"
   ```

3. **Create backup**
   ```bash
   cp /var/lib/ferro/ferro.db /var/lib/ferro/ferro.db.bak.$(date +%s)
   ```

4. **Attempt recovery**
   ```bash
   sqlite3 /var/lib/ferro/ferro.db ".dump" | sqlite3 /var/lib/ferro/ferro_recovered.db
   ```

5. **Replace and restart**
   ```bash
   mv /var/lib/ferro/ferro_recovered.db /var/lib/ferro/ferro.db
   systemctl start ferro
   ```

## Migration Issues

- If a migration fails, check the schema version:
  ```bash
  sqlite3 /var/lib/ferro/ferro.db "PRAGMA user_version;"
  ```
- Roll back a failed migration by restoring from backup, not by re-running migrations.

## Escalation

- If `integrity_check` reports errors that `.dump` cannot recover, restore from the most recent offsite backup.
- File an issue with the full output of `PRAGMA integrity_check`.
