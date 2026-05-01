# Ferro Monitoring Stack

Prometheus + Grafana monitoring for Ferro with pre-built dashboards and alerting rules.

## Quick Start

```bash
docker compose -f deploy/docker-compose.yml \
  -f deploy/monitoring/docker-compose.monitoring.yml \
  up -d
```

Grafana will be available at `http://localhost:3000` (default credentials: `admin` / `admin`).

## Architecture

```
┌──────────┐     /metrics    ┌────────────┐
│  Ferro   │ ───────────────▶│ Prometheus │
│  :8080   │                 │   :9090    │
└──────────┘                 └─────┬──────┘
                                   │ rules
                                   ▼
┌──────────────┐           ┌──────────────┐
│    Grafana   │◀──────────│ Alertmanager │
│    :3000     │  alerts   │    :9093     │
└──────────────┘           └──────────────┘

┌──────────────┐
│ Node Exporter│
│    :9100     │
└──────────────┘
```

## Importing Dashboards

1. Open Grafana at `http://localhost:3000`
2. Navigate to **Dashboards** > **Import**
3. Upload the JSON files from `deploy/monitoring/dashboards/`:
   - `ferro-overview.json` — Server overview (HTTP, resources, storage, federation)
   - `ferro-webdav.json` — WebDAV/CalDAV/CardDAV operations and performance
4. Select the `${DS_PROMETHEUS}` datasource when prompted

Dashboards are also auto-provisioned if you use the provided Docker Compose config (they mount into `/etc/grafana/provisioning/dashboards/`).

## Available Metrics

### Standard (Prometheus default exporter)

| Metric | Description |
|--------|-------------|
| `up` | Target health (1 = up, 0 = down) |
| `process_cpu_seconds_total` | Total CPU time |
| `process_resident_memory_bytes` | Resident memory (RSS) |
| `process_open_fds` | Open file descriptors |
| `process_max_fds` | Maximum file descriptors |
| `process_start_time_seconds` | Process start time |

### HTTP

| Metric | Description |
|--------|-------------|
| `http_requests_total` | Total HTTP requests (labels: `method`, `status`) |
| `http_request_duration_seconds_bucket` | Request latency histogram |
| `http_request_duration_seconds_sum` | Request latency sum |
| `http_request_duration_seconds_count` | Request latency count |

### Application

| Metric | Description |
|--------|-------------|
| `ferro_active_connections` | Current active connections |
| `ferro_storage_health_status` | Storage backend health (1 = healthy) |
| `ferro_storage_files_total` | Total files in storage |
| `ferro_storage_bytes_total` | Total bytes in storage |
| `ferro_storage_operations_total` | Storage operations (label: `operation`) |
| `ferro_federation_inbox_total` | Federation inbox deliveries |
| `ferro_federation_delivery_total` | Federation outgoing deliveries |
| `ferro_federation_errors_total` | Federation errors |

### WebDAV / CalDAV / CardDAV

| Metric | Description |
|--------|-------------|
| `ferro_webdav_sync_token_usage_total` | Sync-Token usage count |
| `ferro_caldav_report_total` | CalDAV REPORT requests |
| `ferro_caldav_operations_total` | Calendar operations (label: `operation`) |
| `ferro_carddav_report_total` | CardDAV REPORT requests |
| `ferro_carddav_operations_total` | Address book operations (label: `operation`) |

## Available Alerts

### Infrastructure (`alerts/infrastructure.yml`)

| Alert | Severity | Condition |
|-------|----------|-----------|
| FerroInstanceDown | critical | Instance unreachable for 1 minute |
| FerroHighMemoryUsage | warning | Memory usage > 85% for 5 minutes |
| FerroHighCPUUsage | warning | CPU usage > 80% for 5 minutes |
| FerroHighFileDescriptorUsage | warning | FD usage > 80% for 5 minutes |
| FerroDiskSpaceLow | warning | Disk space < 15% on /data for 5 minutes |

### Application (`alerts/application.yml`)

| Alert | Severity | Condition |
|-------|----------|-----------|
| FerroHighErrorRate | warning | 5xx rate > 5% for 5 minutes |
| FerroHighLatency | warning | P95 latency > 5s for 10 minutes |
| FerroRequestRateAnomaly | info | Rate deviates > 200% from 7-day average for 30 minutes |
| FerroStorageHealthDegraded | critical | Storage health check failing for 2 minutes |

## Configuring Alerting Channels

Edit `alertmanager.yml` to add notification integrations:

### Slack

```yaml
receivers:
  - name: 'critical'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#alerts-critical'
        send_resolved: true
```

### Email

```yaml
receivers:
  - name: 'critical'
    email_configs:
      - to: 'oncall@example.com'
        from: 'alertmanager@example.com'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alertmanager@example.com'
        auth_password_file: '/etc/alertmanager/email-password'
```

### PagerDuty

```yaml
receivers:
  - name: 'critical'
    pagerduty_configs:
      - service_key: '<your-pagerduty-service-key>'
        severity: '{{ .GroupLabels.severity }}'
```

## File Structure

```
deploy/monitoring/
├── prometheus.yml          # Prometheus scrape config & rules
├── alertmanager.yml        # Alert routing & receivers
├── alerts/
│   ├── infrastructure.yml  # Infrastructure alerts
│   └── application.yml     # Application alerts
├── dashboards/
│   ├── ferro-overview.json # Overview dashboard
│   └── ferro-webdav.json   # WebDAV dashboard
└── README.md
```
