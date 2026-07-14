# Scenario 1: Database Outage

## Scenario Description

At 2:00 AM, monitoring alerts indicate the primary database cluster is unresponsive. All API requests are returning 500 errors. Customers are reporting issues accessing their data.

## Timeline

| Time | Event |
|------|-------|
| 2:00 AM | Alert triggered: Database connection failures |
| 2:05 AM | On-call engineer acknowledges alert |
| 2:10 AM | Investigation begins |
| 2:15 AM | Root cause identified: Database pod crashed |
| 2:20 AM | Mitigation: Restart database pod |
| 2:25 AM | Database restored, monitoring recovery |
| 2:30 AM | Incident resolved |

## Discussion Points

### Detection
- How did we detect the issue?
- Was the alert clear and actionable?
- Could we have detected it earlier?

### Response
- Who responded first?
- Was the escalation appropriate?
- Did we communicate effectively?

### Recovery
- Was the recovery plan followed?
- What worked well?
- What could be improved?

## Lessons Learned

- [To be filled during exercise]
