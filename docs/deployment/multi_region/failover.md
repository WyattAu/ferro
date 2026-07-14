# Failover Procedures

## Automatic Failover

### Detection

| Parameter | Value |
|-----------|-------|
| Health check interval | 30 seconds |
| Failure threshold | 3 consecutive failures |
| Check timeout | 10 seconds |
| Total detection time | ~90 seconds |

Health checks verify:
- HTTP endpoint responds on `/healthz`
- Response status is 200
- Response time <5 seconds
- PostgreSQL is accepting connections

### Process

```
t=0s    Health check fails
t=30s   Second health check fails
t=60s   Third health check fails — region marked unhealthy
t=60s   Load balancer begins traffic rerouting
t=90s   All traffic moved to healthy regions
t=120s  Database promotion complete in secondary region
t=180s  Full service restored
```

1. Health check failure detected
2. Region marked unhealthy after 3 consecutive failures
3. Load balancer removes region from routing pool
4. Traffic rerouted to nearest healthy region (<30 seconds)
5. If primary database was in failed region: replica promoted
6. Application servers in healthy regions scale up
7. Monitoring alerts triggered (PagerDuty/Slack)
8. Incident timeline recorded

### Post-Failover Verification

```bash
# Verify all regions healthy
for region in us-east-1 us-west-2 eu-west-1 ap-northeast-1; do
  curl -sf https://${region}.ferro.internal/healthz && echo "${region}: OK" || echo "${region}: FAILED"
done

# Verify replication status
kubectl exec -it postgres-0 -- psql -c \
  "SELECT client_addr, state, sent_lsn, replay_lsn,
   (sent_lsn - replay_lsn) AS replication_lag
   FROM pg_stat_replication;"

# Verify replication lag is within bounds
kubectl exec -it postgres-0 -- psql -c \
  "SELECT CASE
     WHEN extract(epoch FROM replay_lag) > 30 THEN 'CRITICAL'
     WHEN extract(epoch FROM replay_lag) > 5 THEN 'WARNING'
     ELSE 'OK'
   END AS status,
   extract(epoch FROM replay_lag) AS lag_seconds
   FROM pg_stat_replication;"
```

## Manual Failover

### Planned Maintenance

**Pre-maintenance checklist:**

- [ ] Schedule maintenance window (off-peak preferred)
- [ ] Notify stakeholders via #ops channel
- [ ] Verify target region is healthy
- [ ] Verify replication lag <1 second
- [ ] Run pre-failover consistency check
- [ ] Take database backup

**Execution steps:**

```bash
# 1. Drain traffic from region
kubectl annotate service ferro ingress.kubernetes.io/force-ssl-redirect=false
# OR update DNS weight to 0

# 2. Wait for in-flight requests to complete
sleep 60

# 3. Verify no active connections
kubectl exec -it postgres-0 -- psql -c \
  "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';"

# 4. Perform maintenance

# 5. Restore traffic
# Update DNS weight back to normal

# 6. Verify health
curl -sf https://region.ferro.internal/healthz
```

### Emergency Maintenance

1. Assess impact and determine urgency
2. Notify stakeholders immediately
3. Execute automatic failover (remove region from LB)
4. Perform emergency maintenance
5. Verify修复后 region is healthy
6. Gradually restore traffic (10% → 25% → 50% → 100%)
7. Monitor for 30 minutes post-restore

## Failover Testing

### Monthly Failover Tests

**Test 1: Single Region Failure**

```bash
# Simulate: Block all traffic to region
iptables -A OUTPUT -d <region-subnet> -j DROP

# Monitor: Verify failover completes in <60 seconds
# Verify: All requests served by healthy regions
# Verify: No data loss

# Restore: Remove iptables rule
iptables -D OUTPUT -d <region-subnet> -j DROP
```

**Test 2: Database Primary Failure**

```bash
# Simulate: Stop PostgreSQL primary
kubectl delete pod postgres-0

# Monitor: Verify replica promoted within 60 seconds
# Verify: Application reconnects and serves requests
# Verify: No data loss (check last transaction)

# Restore: Rejoin old primary as replica
```

**Test 3: Network Partition**

```bash
# Simulate: Block cross-region traffic
# Verify: Each region operates independently
# Verify: No split-brain issues
# Verify: Conflict resolution works correctly

# Restore: Reconnect regions
# Verify: Replication catches up
# Verify: Data consistency across regions
```

### Quarterly Disaster Recovery Tests

Full disaster recovery test including:
1. Simulate primary region total loss
2. Promote secondary to primary
3. Verify all services operational
4. Measure recovery time objective (RTO) and recovery point objective (RPO)
5. Document results and gaps

### Annual Full-Scale Tests

Complete multi-region failover exercise:
- All regions involved
- All services tested
- All data paths verified
- Full post-mortem and improvement plan

## Rollback Procedures

### Application Rollback

```bash
# Roll back to previous version
kubectl rollout undo deployment/ferro-server -n ferro

# Verify rollback
kubectl rollout status deployment/ferro-server -n ferro

# Check version
kubectl get deployment ferro-server -n ferro -o jsonpath='{.spec.template.spec.containers[0].image}'
```

### Database Rollback

```bash
# Restore from point-in-time backup
pg_restore -h <host> -U ferro -d ferro \
  --target-time="2026-07-10 14:30:00" \
  /backups/ferro-latest.dump

# Verify data consistency
kubectl exec -it postgres-0 -- psql -c \
  "SELECT count(*) FROM users; SELECT count(*) FROM files;"
```

### Infrastructure Rollback

```bash
# Roll back Terraform changes
cd deploy/terraform
terraform apply -var-file=previous.tfvars

# Roll back Kubernetes manifests
kubectl apply -k deploy/kubernetes/base/previous/
```

## Runbook: Region Failure Response

| Step | Action | Owner | SLA |
|------|--------|-------|-----|
| 1 | Acknowledge alert | On-call | 5 min |
| 2 | Verify failover completed | On-call | 10 min |
| 3 | Check data consistency | DBA | 15 min |
| 4 | Notify stakeholders | Incident lead | 15 min |
| 5 | Root cause investigation | Engineering | 2 hours |
| 6 | Post-incident review | Team | 24 hours |
| 7 | Remediation plan | Engineering | 48 hours |
