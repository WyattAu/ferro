# Data Loss Runbook

## Severity: Level 1 (Critical)

## Symptoms
- Missing data reports
- Database inconsistencies
- Backup failures
- Transaction rollbacks

## Diagnosis

1. Check database integrity:
```bash
kubectl exec -it postgres-0 -n ferro-database -- psql -U ferro -c "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public';"
```

2. Check backup status:
```bash
kubectl logs -n ferro -l app=backup --tail=100
```

3. Check for recent drops/deletes:
```bash
kubectl exec -it postgres-0 -n ferro-database -- psql -U ferro -c "SELECT * FROM pg_stat_user_tables ORDER BY n_tup_del DESC;"
```

## Mitigation

### If accidental deletion:
```bash
# Stop all writes to prevent further damage
kubectl scale deployment/ferro-api -n ferro --replicas=0

# Restore from backup
kubectl exec -it postgres-0 -n ferro-database -- pg_restore -U ferro -d ferro /backups/latest.dump
```

### If corruption:
```bash
# Switch to read-only mode
kubectl set env deployment/ferro-api -n ferro DATABASE_READ_ONLY=true

# Attempt repair
kubectl exec -it postgres-0 -n ferro-database -- psql -U ferro -c "REINDEX DATABASE ferro;"
```

## Recovery

1. Verify data integrity
2. Verify application can read/write data
3. Verify backups are working
4. Monitor for 24 hours

## Prevention

- Implement soft deletes
- Set up point-in-time recovery
- Regular backup testing
- Database audit logging
