# Scenario 3: Data Loss

## Scenario Description

At 3:00 PM, a developer accidentally deletes a production database table while running a migration. The table contains 100,000 customer records.

## Timeline

| Time | Event |
|------|-------|
| 3:00 PM | Accidental deletion occurs |
| 3:05 PM | Error detected by monitoring |
| 3:10 PM | Incident declared |
| 3:15 PM | Backup restoration begins |
| 3:30 PM | Restoration complete |
| 3:35 PM | Data integrity verified |

## Discussion Points

### Detection
- How did we detect the data loss?
- Was the detection immediate?
- Could we have prevented it?

### Response
- Did we follow the data loss runbook?
- Was the backup recent enough?
- Did we communicate effectively?

### Recovery
- How long did recovery take?
- Was any data lost?
- What improvements are needed?

## Lessons Learned

- [To be filled during exercise]
