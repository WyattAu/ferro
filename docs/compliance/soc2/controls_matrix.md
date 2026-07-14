# SOC 2 Controls Matrix

## Access Control

| Control ID | Control | Evidence | Owner | Frequency |
|------------|---------|----------|-------|-----------|
| AC-01 | MFA enabled | MFA logs | Security | Monthly |
| AC-02 | RBAC implemented | Role definitions | Security | Quarterly |
| AC-03 | Access reviews | Review logs | Security | Quarterly |
| AC-04 | Password policy | Policy doc | Security | Annual |

## Change Management

| Control ID | Control | Evidence | Owner | Frequency |
|------------|---------|----------|-------|-----------|
| CM-01 | Code review | PR logs | Engineering | Per PR |
| CM-02 | Testing | Test reports | Engineering | Per PR |
| CM-03 | Deployment approval | Approval logs | Engineering | Per deploy |
| CM-04 | Rollback plan | Runbooks | Engineering | Per deploy |

## Incident Response

| Control ID | Control | Evidence | Owner | Frequency |
|------------|---------|----------|-------|-----------|
| IR-01 | IR plan | Plan doc | Security | Annual |
| IR-02 | IR training | Training logs | Security | Annual |
| IR-03 | Incident tracking | Jira tickets | Security | Per incident |
| IR-04 | Post-mortem | Post-mortem docs | Security | Per incident |

## Data Protection

| Control ID | Control | Evidence | Owner | Frequency |
|------------|---------|----------|-------|-----------|
| DP-01 | Encryption at rest | Config docs | Security | Quarterly |
| DP-02 | Encryption in transit | TLS config | Security | Quarterly |
| DP-03 | Data classification | Classification doc | Security | Annual |
| DP-04 | Data retention | Retention policy | Legal | Annual |