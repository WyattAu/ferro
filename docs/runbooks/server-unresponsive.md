# Runbook: Server Unresponsive

## Symptoms

- HTTP requests time out or return 503/504
- Load balancer health checks fail
- `/healthz` endpoint unreachable
- Clients report connection refused

## Diagnosis

1. **Check process status**
   ```bash
   systemctl status ferro
   # or
   ps aux | grep ferro
   ```

2. **Check logs for panics or OOM kills**
   ```bash
   journalctl -u ferro --since "10 minutes ago" --no-pager | tail -100
   dmesg | grep -i "oom\|killed"
   ```

3. **Check port binding**
   ```bash
   ss -tlnp | grep ferro
   # or
   netstat -tlnp | grep <ferro_port>
   ```

4. **Check resource limits**
   ```bash
   ulimit -a
   cat /proc/<pid>/limits
   ```

5. **Check disk space**
   ```bash
   df -h /var/lib/ferro
   df -h /tmp
   ```

## Resolution

- **Process crashed**: Restart with `systemctl restart ferro`. Check logs for root cause.
- **OOM killed**: Increase memory limits or reduce concurrency. See runbook `memory-exhaustion.md`.
- **Port conflict**: Kill the conflicting process or update `ferro.toml` with a different port.
- **Disk full**: Clear old logs/WAL files, then restart.
- **Deadlock**: Collect a core dump (`gcore <pid>`) and inspect thread state.

## Escalation

- If restarts fail repeatedly, escalate to on-call engineering.
- If the issue is persistent under load, collect a perf profile and open an incident.
