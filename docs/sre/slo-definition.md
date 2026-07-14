# Ferro SLO Definition and Error Budget

**Document:** SLO Definition  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Service Level Objectives

### SLO-01: Request Availability

| Attribute | Value |
|-----------|-------|
| Target | 99.9% |
| Measurement | Successful HTTP responses / Total HTTP requests |
| Window | Rolling 30 days |
| Exclusions | Planned maintenance (max 4h/month) |
| Error Budget | 43.2 minutes downtime per month |

**Definition:** A request is successful if it returns any HTTP 2xx or 3xx status code. Requests returning 5xx are failures.

### SLO-02: Request Latency

| Attribute | Value |
|-----------|-------|
| Target | p99 < 500ms, p95 < 200ms, p50 < 20ms |
| Measurement | Time from request receipt to response completion |
| Window | Rolling 30 days |
| Exclusions | Large file uploads (>10MB), first request after cold start |

**Definition:** Latency measured at the HTTP layer, excluding network transit time.

### SLO-03: Data Durability

| Attribute | Value |
|-----------|-------|
| Target | 99.999% |
| Measurement | Data loss events / Total data operations |
| Window | Rolling 365 days |
| Error Budget | 5.26 minutes of data loss per year |

**Definition:** Once a file upload returns 200 OK and is persisted to the configured storage backend, the data must be recoverable within the retention period.

### SLO-04: API Error Rate

| Attribute | Value |
|-----------|-------|
| Target | < 0.1% |
| Measurement | 5xx responses / Total API requests |
| Window | Rolling 7 days |
| Error Budget | 10.08 minutes of 5xx per month |

**Definition:** Server-side errors (5xx) indicate bugs or resource exhaustion. Client errors (4xx) are not counted as SLO violations.

---

## Error Budget Policy

### Budget Remaining > 50%
- Normal deployment velocity
- Feature work permitted
- Refactoring permitted

### Budget Remaining 20-50%
- Increase review rigor for reliability-impacting changes
- Require rollback plan for all deployments
- No experimental features

### Budget Remaining < 20%
- Freeze on non-critical changes
- All changes require incident commander approval
- Focus exclusively on reliability improvements
- Post-mortem required for any error budget consumption

### Budget Exhausted
- Code freeze on all non-critical paths
- Mandatory reliability sprint
- All incidents require formal post-mortem
- Stakeholder notification

---

## Monitoring Configuration

### Prometheus Alerting Rules

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| ErrorBudgetLow | error_budget_remaining < 20% | Warning | Notify on-call |
| ErrorBudgetCritical | error_budget_remaining < 5% | Critical | Page on-call |
| HighErrorRate | rate(http_5xx_total[5m]) / rate(http_requests_total[5m]) > 0.01 | Critical | Auto-rollback |
| HighLatencyP99 | histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 0.5 | Warning | Investigate |
| HighLatencyP95 | histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 0.2 | Warning | Investigate |
| DataDurabilityRisk | fsync_failure_rate > 0.001 | Critical | Page on-call |

### Grafana Dashboard

- **SLO Dashboard:** `docs/quality/slo_dashboard.json` (to be created)
- **Panels:** Error budget burn rate, current budget remaining, latency percentiles, error rate trend

---

## Incident Severity Levels

| Level | Description | Response Time | Escalation |
|-------|-------------|---------------|------------|
| P0 | Data loss or complete service outage | Immediate | CEO + CTO |
| P1 | Service degraded, >10% users affected | 15 minutes | On-call engineer |
| P2 | Service degraded, <10% users affected | 1 hour | On-call engineer |
| P3 | Minor issue, no user impact | Next business day | Engineering lead |

---

## Escalation Matrix

| Severity | Initial Responder | Escalation (15min) | Escalation (30min) | Escalation (1h) |
|----------|-------------------|--------------------|--------------------|-----------------|
| P0 | On-call engineer | Engineering lead | CTO | CEO |
| P1 | On-call engineer | Engineering lead | CTO | - |
| P2 | On-call engineer | Engineering lead | - | - |
| P3 | Assigned engineer | Engineering lead | - | - |

---

## Related Documents

- `docs/runbooks/` - Operational runbooks
- `docs/reliability/incident_management/` - Incident response procedures
- `docs/compliance/soc2/` - SOC 2 compliance documentation
- `monitoring/prometheus/alerts.yml` - Prometheus alerting rules
