# Runbook: Security Incident Response

## Severity Levels

| Level | Description | Response Time |
|-------|-------------|---------------|
| P1 | Active data breach, credential leak | Immediate |
| P2 | Unauthorized access, privilege escalation | < 1 hour |
| P3 | Suspicious activity, failed auth attempts | < 4 hours |

## 1. Containment

1. **Revoke compromised credentials immediately**
   ```bash
   # Revoke all API keys for the affected user
   ferro-admin api-keys revoke --user <username> --all

   # Rotate JWT signing secret
   openssl rand -base64 48 > /tmp/new_jwt_secret
   # Update ferro.toml [auth.jwt_secret] and restart
   ```

2. **Isolate the system (if active breach)**
   ```bash
   # Block external access at the firewall
   iptables -A INPUT -p tcp --dport <ferro_port> -j DROP

   # Or remove from load balancer
   aws elb deregister-instances-from-load-balancer --instances <instance-id>
   ```

3. **Preserve evidence**
   ```bash
   # Snapshot logs before rotation
   tar czf /tmp/ferro-incident-logs-$(date +%s).tar.gz /var/log/ferro/

   # Preserve database state
   cp /var/lib/ferro/ferro.db /var/lib/ferro/ferro.db.incident-$(date +%s)
   ```

## 2. Investigation

1. **Review audit logs**
   ```bash
   ferro-admin audit-log query --since "24 hours ago" --user <username>
   ```

2. **Check for unauthorized API key usage**
   ```bash
   ferro-admin api-keys list --user <username>
   ```

3. **Review recent authentication events**
   ```bash
   grep -i "failed\|unauthorized\|token" /var/log/ferro/auth.log
   ```

4. **Check for data exfiltration indicators**
   ```bash
   # Large download volumes
   grep "GET\|HEAD" /var/log/ferro/access.log | awk '{print $7}' | sort | uniq -c | sort -rn | head
   ```

## 3. Notification

- **P1 incidents**: Notify CISO, legal, and affected users within 72 hours (GDPR) or as required by law.
- **P2 incidents**: Notify security team and affected users within 72 hours.
- **P3 incidents**: Document in incident tracker, no external notification required.

## 4. Recovery

1. **Rotate all secrets** in `ferro.toml`:
   - `auth.jwt_secret`
   - `storage.s3.secret_key` (if exposed)
   - Any other credentials in environment variables

2. **Force password reset** for affected users
   ```bash
   ferro-admin users reset-password --user <username>
   ```

3. **Review and tighten RBAC policies**
   ```bash
   ferro-admin policies list
   # Remove overly permissive policies
   ```

4. **Deploy updated code** if a vulnerability was patched.

5. **Resume normal operations** and remove firewall/load balancer restrictions.

## 5. Post-Incident

- Write a post-incident report within 48 hours.
- Update security policies and access controls.
- Schedule a follow-up review to verify mitigations.
