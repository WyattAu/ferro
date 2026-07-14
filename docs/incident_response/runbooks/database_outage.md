# Database Outage Runbook

## Severity: Level 1 (Critical)

## Symptoms
- Database connection errors
- API returning 500 errors
- High latency on database queries

## Diagnosis

1. Check database status:
```bash
kubectl get pods -n ferro-database
kubectl logs -n ferro-database -l app=postgres
```

2. Check connection pool:
```bash
curl -s http://localhost:9090/metrics | grep pg_stat_activity
```

3. Check disk space:
```bash
kubectl exec -it postgres-0 -n ferro-database -- df -h
```

## Mitigation

### If pod is down:
```bash
kubectl restart deployment postgres -n ferro-database
```

### If disk is full:
```bash
kubectl exec -it postgres-0 -n ferro-database -- vacuumdb -U ferro ferro
```

### If connection pool exhausted:
```bash
# Restart application to reset connection pool
kubectl rollout restart deployment/ferro-server -n ferro
```

## Recovery

1. Verify database is healthy
2. Verify application can connect
3. Verify API responses are correct
4. Monitor for 15 minutes

## Prevention

- Set up disk space alerts
- Configure connection pool monitoring
- Implement automated vacuuming
