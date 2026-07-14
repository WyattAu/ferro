# Runbook: Incident Response

## Overview

This runbook defines the incident response process for Ferro production incidents, including classification, escalation, communication, and resolution procedures.

## Severity Levels

| Level | Description | Response Time | Update Frequency |
|-------|-------------|---------------|------------------|
| P1 | Complete service outage, data loss | Immediate | Every 30 minutes |
| P2 | Major feature degradation | < 15 minutes | Every hour |
| P3 | Minor feature issue, workaround exists | < 1 hour | Every 4 hours |
| P4 | Cosmetic issue, low impact | < 24 hours | Daily |

## Prerequisites

- [ ] Incident commander assigned
- [ ] Communication channel created (Slack/Teams)
- [ ] Monitoring dashboards accessible
- [ ] Runbooks for specific incidents available
- [ ] Contact list up to date

## Incident Classification

### P1 - Critical
- Complete service outage
- Data breach or loss
- Security vulnerability actively exploited
- All users affected

### P2 - Major
- Primary feature unavailable
- Significant performance degradation
- Partial data loss
- Multiple users affected

### P3 - Minor
- Secondary feature unavailable
- Workaround available
- Single user or limited impact
- Non-critical functionality

### P4 - Low
- Cosmetic issues
- Documentation errors
- Non-urgent improvements

## Response Procedure

### 1. Detection and Acknowledgment (0-5 minutes)

```bash
# Acknowledge alert in monitoring system
# Create incident ticket with severity level
# Notify on-call team
```

### 2. Assembly (5-15 minutes)

- Assign Incident Commander
- Create dedicated communication channel
- Begin incident timeline documentation
- Pull in relevant team members based on severity

### 3. Assessment (15-30 minutes)

```bash
# Check service health
curl -f http://localhost:8080/healthz

# Review recent logs
journalctl -u ferro --since "30 minutes ago" --no-pager | tail -200

# Check system resources
top -bn1 | head -20
df -h
free -m

# Check database
sqlite3 /var/lib/ferro/ferro.db "PRAGMA integrity_check;"
```

### 4. Mitigation (30-60 minutes)

**For Service Outage:**
```bash
# Restart service
systemctl restart ferro

# If restart fails, check dependencies
systemctl status ferro
journalctl -u ferro --since "5 minutes ago" --no-pager
```

**For Performance Issues:**
```bash
# Check connection pools
ferro-admin status

# Reduce load if needed
# Scale up or add instances
```

**For Data Issues:**
```bash
# Stop writes immediately
systemctl stop ferro

# Preserve current state for investigation
cp /var/lib/ferro/ferro.db /var/lib/ferro/ferro.db.incident-$(date +%s)
```

### 5. Resolution (1-4 hours)

- Implement permanent fix
- Verify fix in staging first if possible
- Deploy fix with monitoring
- Confirm service restoration

### 6. Recovery (4-24 hours)

- Monitor for recurrence
- Verify all features working
- Check data integrity
- Resume normal operations

## Communication Templates

### Initial Notification
```
[INCIDENT] P{X} - {Brief Description}
Status: Investigating
Impact: {Description of impact}
Next update: {Time}
```

### Status Update
```
[UPDATE] P{X} - {Brief Description}
Status: {Investigating|Identified|Monitoring|Resolved}
Progress: {What's been done}
Next update: {Time}
```

### Resolution
```
[RESOLVED] P{X} - {Brief Description}
Duration: {Total time}
Root cause: {Brief description}
Remediation: {Actions taken}
Post-mortem scheduled: {Date/Time}
```

## Post-Incident

- [ ] Write post-mortem report within 48 hours
- [ ] Identify root cause
- [ ] Document lessons learned
- [ ] Create action items for prevention
- [ ] Update runbooks if needed
- [ ] Schedule follow-up review

## Escalation Matrix

| Severity | First Responder | Escalation (15 min) | Escalation (1 hour) |
|----------|-----------------|---------------------|---------------------|
| P1 | On-Call Engineer | Engineering Lead | CTO |
| P2 | On-Call Engineer | Engineering Lead | VP Engineering |
| P3 | Support Team | On-Call Engineer | Engineering Lead |
| P4 | Support Team | Support Lead | On-Call Engineer |

## Contact Information

| Role | Contact | Availability |
|------|---------|--------------|
| On-Call Engineer | @oncall | 24/7 |
| Engineering Lead | @eng-lead | Business hours |
| Database Admin | @db-admin | Business hours |
| Security Lead | @security-lead | 24/7 for P1 |
| CTO | @cto | P1 escalation |
