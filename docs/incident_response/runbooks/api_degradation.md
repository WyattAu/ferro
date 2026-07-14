# API Degradation Runbook

## Severity: Level 2 (High)

## Symptoms
- Increased API latency
- Intermittent 502/503 errors
- Slow response times

## Diagnosis

1. Check API server status:
```bash
kubectl get pods -n ferro -l app=api
kubectl top pods -n ferro
```

2. Check application logs:
```bash
kubectl logs -n ferro -l app=api --tail=100
```

3. Check resource usage:
```bash
kubectl top nodes
kubectl describe node <node-name>
```

## Mitigation

### If pod is OOMKilled:
```bash
kubectl rollout restart deployment/ferro-api -n ferro
```

### If high CPU:
```bash
# Scale up replicas
kubectl scale deployment/ferro-api -n ferro --replicas=5
```

### If downstream service issue:
```bash
# Check service dependencies
curl -s http://localhost:9090/metrics | grep upstream_request_duration
```

## Recovery

1. Verify API latency is normal
2. Verify error rates are low
3. Verify downstream services are healthy
4. Monitor for 30 minutes

## Prevention

- Set up latency alerts
- Configure auto-scaling
- Implement circuit breakers
