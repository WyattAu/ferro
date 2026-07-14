# Runbook: Security Incident Handling

## Overview

This runbook provides procedures for handling security incidents, including data breaches, unauthorized access, credential leaks, and vulnerability exploitation.

## Severity Levels

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Active data breach, credential leak | Immediate |
| P2 | Unauthorized access, privilege escalation | < 1 hour |
| P3 | Suspicious activity, failed auth attempts | < 4 hours |
| P4 | Security policy violations | < 24 hours |

## Prerequisites

- [ ] Security team contact list available
- [ ] Incident response team assembled
- [ ] Forensic tools available
- [ ] Legal/compliance team notified (for P1/P2)
- [ ] Communication plan ready

## Incident Classification

### P1 - Critical
- Active data breach
- Credential leak (API keys, passwords, tokens)
- Ransomware or malware
- All users' data at risk

### P2 - High
- Unauthorized access to admin functions
- Privilege escalation
- Vulnerability actively exploited
- Sensitive data exposed

### P3 - Medium
- Suspicious login attempts
- Failed authentication patterns
- Policy violations
- Non-critical vulnerabilities

### P4 - Low
- Security misconfigurations
- Minor policy violations
- Informational security events

## Response Procedure

### 1. Containment (Immediate)

#### Revoke Compromised Credentials

```bash
# Revoke all API keys for affected user
ferro-admin api-keys revoke --user <username> --all

# Rotate JWT signing secret
NEW_JWT_SECRET=$(openssl rand -base64 48)
echo "$NEW_JWT_SECRET" > /tmp/new_jwt_secret

# Update configuration
sed -i "s/jwt_secret = .*/jwt_secret = \"$NEW_JWT_SECRET\"/" /etc/ferro/ferro.toml

# Restart to apply new secret
systemctl restart ferro
```

#### Isolate the System (Active Breach)

```bash
# Block external access at firewall
iptables -A INPUT -p tcp --dport <ferro_port> -j DROP

# Or remove from load balancer (AWS example)
aws elb deregister-instances-from-load-balancer \
  --instances <instance-id>

# Or scale down to zero (Kubernetes)
kubectl scale deployment ferro --replicas=0
```

#### Preserve Evidence

```bash
# Snapshot logs before any changes
tar czf /tmp/ferro-incident-logs-$(date +%s).tar.gz /var/log/ferro/

# Preserve database state
cp /var/lib/ferro/ferro.db /var/lib/ferro/ferro.db.incident-$(date +%s)

# Preserve memory dump (if needed)
gcore -o /tmp/ferro-dump $(pgrep ferro)

# Preserve network connections
ss -tlnp > /tmp/ferro-connections-$(date +%s).txt
```

### 2. Investigation

#### Review Audit Logs

```bash
# Query audit logs for affected user
ferro-admin audit-log query --since "24 hours ago" --user <username>

# Check for unauthorized API key usage
ferro-admin api-keys list --user <username>

# Review recent authentication events
grep -i "failed\|unauthorized\|token" /var/log/ferro/auth.log

# Check for data exfiltration indicators
grep "GET\|HEAD" /var/log/ferro/access.log | \
  awk '{print $7}' | sort | uniq -c | sort -rn | head
```

#### Analyze Attack Vector

```bash
# Check for SQL injection attempts
grep -i "union\|select\|drop\|insert\|update" /var/log/ferro/*.log

# Check for path traversal attempts
grep -i "\.\./\|etc/passwd\|shadow" /var/log/ferro/*.log

# Check for XSS attempts
grep -i "<script\|javascript\|onerror" /var/log/ferro/*.log

# Review IP addresses
awk '{print $1}' /var/log/ferro/access.log | sort | uniq -c | sort -rn | head
```

### 3. Notification

| Severity | Notification Requirements |
|----------|---------------------------|
| P1 | CISO, legal, affected users within 72 hours (GDPR) |
| P2 | Security team, affected users within 72 hours |
| P3 | Document in incident tracker |
| P4 | Security team review |

### 4. Recovery

#### Rotate All Secrets

```bash
# Generate new secrets
NEW_JWT_SECRET=$(openssl rand -base64 48)
NEW_API_SECRET=$(openssl rand -base64 32)

# Update ferro.toml
cat > /tmp/ferro-secrets.txt << EOF
[auth]
jwt_secret = "$NEW_JWT_SECRET"

[storage]
s3_secret_key = "$NEW_API_SECRET"
EOF

# Apply changes
systemctl restart ferro
```

#### Force Password Reset

```bash
# Reset password for affected users
ferro-admin users reset-password --user <username>

# Or force password expiry for all users
ferro-admin users expire-passwords --all
```

#### Review and Tighten RBAC

```bash
# List current policies
ferro-admin policies list

# Remove overly permissive policies
ferro-admin policies delete --name <policy-name>

# Verify minimal required permissions
ferro-admin roles list --verbose
```

### 5. Post-Incident

- [ ] Write post-incident report within 48 hours
- [ ] Update security policies and access controls
- [ ] Schedule follow-up review to verify mitigations
- [ ] Conduct security training if needed
- [ ] Update monitoring rules
- [ ] Review and update this runbook

## Verification Checklist

- [ ] Compromised credentials revoked
- [ ] All secrets rotated
- [ ] Affected users notified
- [ ] System isolated (if active breach)
- [ ] Evidence preserved
- [ ] Attack vector identified
- [ ] Vulnerability patched
- [ ] Monitoring enhanced
- [ ] Post-incident review scheduled

## Legal and Compliance

### GDPR Requirements
- Report to supervisory authority within 72 hours
- Notify affected users without undue delay
- Document all actions taken

### HIPAA Requirements
- Report to HHS within 60 days
- Notify affected individuals
- Document breach assessment

### Other Regulations
- Check applicable regulations for your jurisdiction
- Consult legal team for notification requirements

## Escalation

- If data breach is confirmed, escalate to legal and compliance immediately.
- If active attack is ongoing, escalate to CISO and consider law enforcement.
- If vulnerability is critical, consider responsible disclosure process.

## Contact Information

| Role | Contact | Availability |
|------|---------|--------------|
| Security Lead | @security-lead | 24/7 for P1/P2 |
| On-Call Engineer | @oncall | 24/7 |
| Legal/Compliance | @legal | Business hours |
| CISO | @ciso | P1 escalation |
| Engineering Lead | @eng-lead | Business hours |
| External Security Firm | @security-firm | Contract basis |
