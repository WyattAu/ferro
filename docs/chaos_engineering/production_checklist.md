# Chaos Engineering Production Checklist

## Pre-Experiment

- [ ] Backup database
- [ ] Notify team
- [ ] Set up monitoring
- [ ] Prepare rollback plan

## During Experiment

- [ ] Monitor system metrics
- [ ] Log all errors
- [ ] Track recovery time
- [ ] Document observations

## Post-Experiment

- [ ] Verify data integrity
- [ ] Check system health
- [ ] Review logs
- [ ] Update documentation

## Experiment Schedule

| Experiment | Frequency | Duration | Owner |
|------------|-----------|----------|-------|
| Network Partition | Weekly | 1 hour | SRE |
| Disk Failure | Monthly | 2 hours | SRE |
| Memory Pressure | Monthly | 1 hour | SRE |
| CPU Saturation | Weekly | 30 min | SRE |

## Success Criteria

- [ ] System remains available
- [ ] No data loss
- [ ] Recovery within SLA
- [ ] Errors are logged
- [ ] Alerts are triggered
