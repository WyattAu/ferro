# Security Breach Runbook

## Severity: Level 1 (Critical)

## Symptoms
- Unauthorized access detected
- Data exfiltration alert
- Suspicious activity logs
- Failed authentication attempts

## Diagnosis

1. Check security logs:
```bash
kubectl logs -n ferro -l app=security --tail=200
```

2. Check access logs:
```bash
kubectl logs -n ferro -l app=nginx --tail=200 | grep "401\|403"
```

3. Check database audit logs:
```bash
kubectl exec -it postgres-0 -n ferro-database -- psql -U ferro -c "SELECT * FROM audit_log WHERE timestamp > NOW() - INTERVAL '1 hour';"
```

## Mitigation

### If unauthorized access:
```bash
# Revoke all active sessions
kubectl exec -it postgres-0 -n ferro-database -- psql -U ferro -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'ferro';"

# Rotate secrets
kubectl delete secret -n ferro --all
# Re-deploy with new secrets
kubectl rollout restart deployment/ferro-server -n ferro
```

### If data exfiltration:
```bash
# Block suspicious IPs
kubectl exec -it nginx-0 -n ferro -- iptables -A INPUT -s <suspicious-ip> -j DROP

# Enable enhanced logging
kubectl set env deployment/ferro-api -n ferro LOG_LEVEL=debug
```

## Recovery

1. Verify security controls are restored
2. Verify no ongoing unauthorized access
3. Notify affected users if data was exposed
4. Document all findings

## Prevention

- Enable audit logging
- Implement MFA
- Set up intrusion detection
- Regular security reviews
