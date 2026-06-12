# Production Deployment Guide

Comprehensive guide for deploying Ferro in a production environment with TLS, monitoring, backups, and security hardening.

---

## Prerequisites

### Software

- **Docker Engine** 24.0+ with Docker Compose v2 plugin
- **Git** for cloning the repository
- A domain name with DNS pointing to your server (for HTTPS)

### Hardware (minimum)

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Disk | 20 GB | 100+ GB SSD |
| Network | 100 Mbps | 1 Gbps |

### Network

- Ports **80** and **443** open inbound (HTTP/HTTPS)
- DNS `A` record pointing your domain to the server IP
- Outbound access to `ghcr.io` (container images) and Let's Encrypt (TLS)

---

## Quick Start (5 minutes)

```bash
# 1. Clone the repository
git clone https://github.com/WyattAu/ferro.git
cd ferro/deploy

# 2. Create your environment file
cp .env.example .env

# 3. Edit .env with your values (domain, passwords, etc.)
nano .env

# 4. Start the production stack
docker compose -f docker-compose.yml -f docker-compose.pg.yml up -d

# 5. Verify
curl -f http://localhost:8080/healthz
```

For the **full production stack** (PostgreSQL + Redis + Caddy + monitoring):

```bash
cp .env.example .env
# Edit .env (set POSTGRES_PASSWORD, GRAFANA_ADMIN_PASSWORD, DOMAIN, etc.)
docker compose -f docker-compose.production.yml up -d
```

---

## Production Stack Architecture

```
                    ┌─────────────────────────────────────────────┐
                    │                  Internet                    │
                    └──────────────────┬──────────────────────────┘
                                       │
                              ┌────────▼────────┐
                              │   Caddy (TLS)    │
                              │  :80 / :443      │
                              │  Auto-HTTPS      │
                              │  Rate Limiting   │
                              │  Security Headers│
                              └────────┬────────┘
                                       │
                              ┌────────▼────────┐
                              │   Ferro Server   │
                              │     :8080        │
                              │  WebDAV/REST/WS  │
                              │  /healthz        │
                              │  /metrics        │
                              └──┬─────────┬───┘
                                 │         │
              ┌──────────────────┘         └──────────────────┐
              │                                               │
     ┌────────▼────────┐                           ┌──────────▼──────┐
     │   PostgreSQL 16  │                           │    Redis 7      │
     │     :5432        │                           │    :6379        │
     │  Metadata + DAV  │                           │  Cache + Queue  │
     └─────────────────┘                           └─────────────────┘
              │                                               │
              └──────────────────┬────────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     Monitoring Stack     │
                    │  Prometheus  :9090       │
                    │  Grafana     :3000       │
                    │  Alertmanager:9093       │
                    └─────────────────────────┘
```

**Network topology:**

| Network | Purpose | Access |
|---------|---------|--------|
| `frontend` | Caddy ↔ Ferro | Public-facing |
| `backend` | Ferro ↔ PostgreSQL, Redis | Internal only |
| `monitoring` | Prometheus, Grafana, Alertmanager | Internal only |

---

## Configuration

### Environment Variables

All configuration is controlled via the `.env` file. Copy `.env.example` to `.env` and edit.

| Variable | Default | Description |
|----------|---------|-------------|
| **Ferro** | | |
| `FERRO_PORT` | `8080` | Host port for Ferro (mapped to container :8080) |
| **PostgreSQL** | | |
| `POSTGRES_PASSWORD` | `changeme` | **Change this.** PostgreSQL password for the `ferro` user |
| **Caddy** | | |
| `DOMAIN` | `localhost` | Your domain name (e.g., `ferro.example.com`) |
| `HTTP_PORT` | `80` | HTTP port (Caddy) |
| `HTTPS_PORT` | `443` | HTTPS port (Caddy) |
| **Grafana** | | |
| `GRAFANA_PORT` | `3000` | Grafana web UI port |
| `GRAFANA_ADMIN_USER` | `admin` | Grafana admin username |
| `GRAFANA_ADMIN_PASSWORD` | `admin` | **Change this.** Grafana admin password |
| `GRAFANA_ROOT_URL` | `http://localhost:3000` | Grafana public URL |
| **Prometheus** | | |
| `PROMETHEUS_RETENTION` | `30d` | How long to keep metrics data |

### Database Configuration (PostgreSQL)

The production stack uses **PostgreSQL 16 Alpine** with the following defaults:

| Setting | Value |
|---------|-------|
| Database | `ferro` |
| User | `ferro` |
| Password | `${POSTGRES_PASSWORD}` |
| Data volume | `postgres-data:/var/lib/postgresql/data` |
| Health check | `pg_isready -U ferro` |
| Memory limit | 512 MB (production) / 256 MB (overlay) |

PostgreSQL is used for metadata, DAV sync tokens, and audit logs. Data files are stored on the local filesystem by default.

### Redis Configuration

Redis 7 Alpine provides caching and session storage:

| Setting | Value |
|---------|-------|
| URL | `redis://redis:6379` |
| Persistence | AOF (`--appendonly yes`) |
| Max memory | 256 MB |
| Eviction | `allkeys-lru` |
| Health check | `redis-cli ping` |

### Storage Configuration

| Backend | Configuration | Best For |
|---------|---------------|----------|
| Local filesystem | Default (`/data` volume) | Single node, <100 users |
| PostgreSQL | `FERRO_DATABASE_URL` env var | Metadata + DAV sync |
| S3 | `FERRO_S3_*` env vars | Cloud-native, scalable |
| GCS | `FERRO_GCS_*` env vars | Google Cloud |
| Azure Blob | `FERRO_AZURE_*` env vars | Azure Cloud |

For production, use **PostgreSQL + local filesystem** or **PostgreSQL + S3** for maximum reliability.

### Authentication Options

| Method | Configuration | Use Case |
|--------|---------------|----------|
| Basic Auth | `admin_user` / `admin_password` in config | Simple single-user |
| API Keys | Generate via admin API | Programmatic access |
| Federation | `--federation-secret` for ActivityPub | Cross-server federation |

### CORS Settings

If using a web UI, configure allowed origins:

```bash
# In your ferro.toml or environment
FERRO_CORS_ALLOWED_ORIGINS=https://your-domain.com,https://app.your-domain.com
```

---

## HTTPS Setup

### Caddy Auto-TLS (Recommended)

Caddy automatically provisions and renews **Let's Encrypt** certificates. Set `DOMAIN` in `.env`:

```bash
DOMAIN=ferro.example.com
```

The Caddyfile at `deploy/Caddyfile` configures:

- Reverse proxy to Ferro on `:8080`
- Automatic HTTPS with Let's Encrypt
- Gzip compression
- Security headers (HSTS, X-Frame-Options, CSP, etc.)

### Custom Certificate Support

Replace the Caddyfile with custom TLS configuration:

```text
your-domain.com {
    tls /path/to/cert.pem /path/to/key.pem
    reverse_proxy ferro:8080
}
```

Or use an internal CA:

```text
your-domain.com {
    tls internal
    reverse_proxy ferro:8080
}
```

### Reverse Proxy Configuration

Caddy sets these security headers on all responses:

| Header | Value |
|--------|-------|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains; preload` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |

### WebSocket Proxying

The Caddyfile includes a `handle_path /ws*` block for WebSocket upgrades. Ensure your client connects to `wss://your-domain.com/ws*`.

---

## Monitoring

The production stack includes a full monitoring suite.

### Prometheus Metrics

Ferro exposes a `/metrics` endpoint in Prometheus format. Key metrics:

| Category | Metrics |
|----------|---------|
| **HTTP** | `http_requests_total`, `http_request_duration_seconds_bucket`, `http_request_size_bytes_sum` |
| **Application** | `ferro_active_connections`, `ferro_storage_files_total`, `ferro_storage_bytes_total` |
| **Federation** | `ferro_federation_inbox_total`, `ferro_federation_delivery_total`, `ferro_federation_errors_total` |
| **WebDAV/CalDAV** | `ferro_webdav_sync_token_usage_total`, `ferro_caldav_report_total`, `ferro_carddav_operations_total` |
| **CRDT** | `ferro_crdt_sync_operations_total`, `ferro_crdt_sync_sessions_active` |
| **Auth** | `ferro_auth_failures_total`, `ferro_auth_attempts_total`, `ferro_active_sessions` |
| **System** | `process_cpu_seconds_total`, `process_resident_memory_bytes`, `process_open_fds` |

Prometheus scrape config (`deploy/monitoring/prometheus.yml`) scrapes Ferro every 15 seconds.

### Grafana Dashboards

Grafana is available at `http://localhost:3000` (default credentials: `admin` / `admin`).

Import dashboards from `deploy/monitoring/dashboards/`:

| Dashboard | File | Description |
|-----------|------|-------------|
| Server Overview | `ferro-overview.json` | HTTP, resources, storage, federation |
| WebDAV | `ferro-webdav.json` | WebDAV/CalDAV/CardDAV operations |
| Admin Dashboard | `admin-dashboard.json` | Combined admin view |
| Logs Overview | `grafana-loki/logs-overview.json` | Log volume, errors, full-text search |
| Audit Log | `grafana-loki/audit-log.json` | Audit events, action breakdown |

### Alert Rules

Pre-configured alerts in `deploy/monitoring/alerts/`:

#### Infrastructure (`infrastructure.yml`)

| Alert | Severity | Condition |
|-------|----------|-----------|
| FerroInstanceDown | critical | Instance unreachable >1 min |
| FerroHighMemoryUsage | warning | Memory >85% for 5 min |
| FerroHighCPUUsage | warning | CPU >80% for 5 min |
| FerroHighFileDescriptorUsage | warning | FD usage >80% for 5 min |
| FerroDiskSpaceLow | warning | Disk <15% for 5 min |

#### Application (`application.yml`)

| Alert | Severity | Condition |
|-------|----------|-----------|
| FerroHighErrorRate | warning | 5xx rate >5% for 5 min |
| FerroHighLatency | warning | P95 latency >5s for 10 min |
| FerroRequestRateAnomaly | info | Rate deviates >200% from 7-day avg |
| FerroStorageHealthDegraded | critical | Storage health failing >2 min |

### Alertmanager Configuration

Alertmanager routes alerts to receivers defined in `deploy/monitoring/alertmanager.yml`:

| Receiver | Severity | Default Target |
|----------|----------|----------------|
| `default` | info | Ferro webhook (`/api/webhooks/alertmanager`) |
| `critical` | critical | Ferro webhook (1h repeat) |
| `warnings` | warning | Ferro webhook |

To add Slack, email, or PagerDuty notifications, edit `alertmanager.yml` (see [monitoring README](../../deploy/monitoring/README.md#configuring-alerting-channels)).

### Log Aggregation

Logs are written in JSON format (`FERRO_LOG_FORMAT=json`). For centralized logging:

**Option A: Grafana + Loki**

```bash
docker compose -f deploy/monitoring/docker-compose.grafana-loki.yml up -d
```

**Option B: VictoriaMetrics + VictoriaLogs** (lower resource usage)

```bash
docker compose -f deploy/monitoring/docker-compose.victoria.yml up -d
```

---

## Backup & Recovery

### Automated Backup Schedule

```bash
# Schedule daily backups via cron (runs at 2am UTC)
0 2 * * * curl -s -X POST https://your-domain.com/api/admin/backup -u admin:${FERRO_PASSWORD} > /dev/null 2>&1

# Weekly full backup with timestamp
0 3 * * 0 curl -s -X POST https://your-domain.com/api/admin/backup -u admin:${FERRO_PASSWORD} && \
  docker exec ferro-postgres pg_dump -U ferro ferro > /backups/postgres-$(date +\%Y\%m\%d).sql
```

### Manual Backup Procedure

**Step 1: Trigger Ferro backup via API**

```bash
curl -X POST https://your-domain.com/api/admin/backup \
  -u admin:your-password
```

**Step 2: Backup PostgreSQL**

```bash
docker exec ferro-postgres pg_dump -U ferro ferro > backup-$(date +%Y%m%d-%H%M%S).sql
```

**Step 3: Backup data volume**

```bash
docker run --rm -v ferro-data:/data -v $(pwd):/backup alpine \
  tar czf /backup/ferro-data-$(date +%Y%m%d-%H%M%S).tar.gz -C /data .
```

### Disaster Recovery

1. Stop the Ferro server: `docker compose stop ferro`
2. Restore the PostgreSQL dump:
   ```bash
   docker exec -i ferro-postgres psql -U ferro ferro < backup.sql
   ```
3. Restore the data volume:
   ```bash
   docker run --rm -v ferro-data:/data -v $(pwd):/backup alpine \
     tar xzf /backup/ferro-data-YYYYMMDD-HHMMSS.tar.gz -C /data
   ```
4. Restart: `docker compose up -d`
5. Verify: `curl -f https://your-domain.com/readyz`

### Point-in-Time Recovery

With PostgreSQL WAL archiving enabled:

1. Ensure `archive_mode = on` and `archive_command` is configured in PostgreSQL
2. Restore the base backup
3. Replay WAL segments to the desired timestamp
4. Verify with `pg_waldump`

For local filesystem storage, maintain timestamped snapshots of the data volume.

---

## Scaling

### Scaling Tiers

| Users | Setup | Resources |
|-------|-------|-----------|
| 1-10 | Single node, local storage, SQLite | 2 CPU, 2 GB RAM |
| 10-100 | Single node, PostgreSQL, Redis | 2 CPU, 4 GB RAM |
| 100-1000 | Multiple nodes, shared PostgreSQL + S3 | 4 CPU, 8 GB RAM per node |
| 1000+ | Kubernetes, PostgreSQL HA, S3, CDN | Auto-scaled |

### Horizontal Scaling with Raft Consensus

Ferro supports multi-node deployments using CRDT-based sync:

```bash
# Node 1 (primary)
ferro-server --raft-addr node1:7000 --raft-id node1

# Node 2 (replica)
ferro-server --raft-addr node2:7000 --raft-id node2 --raft-join node1:7000
```

All nodes share the same storage backend (S3/GCS/Azure Blob) for consistency.

### PostgreSQL Connection Pooling

For high-traffic deployments, add PgBouncer:

```yaml
# Add to docker-compose.production.yml
pgbouncer:
  image: edoburu/pgbouncer:latest
  environment:
    DATABASE_URL: postgres://ferro:${POSTGRES_PASSWORD}@postgres:5432/ferro
    MAX_CLIENT_CONN: 1000
    DEFAULT_POOL_SIZE: 50
  ports:
    - "6432:6432"
```

Then point `FERRO_DATABASE_URL` to `postgres://ferro:password@pgbouncer:6432/ferro`.

### Redis Cluster Mode

For multi-node deployments, use Redis Cluster:

```yaml
redis:
  command: >
    redis-server
    --cluster-enabled yes
    --cluster-config-file nodes.conf
    --cluster-node-timeout 5000
    --appendonly yes
    --maxmemory 512mb
    --maxmemory-policy allkeys-lru
```

### CDN for Static Assets

Place a CDN (Cloudflare, CloudFront, etc.) in front of your domain:

1. Set `CNAME` to point to your server
2. Enable proxy/CDN in your DNS provider
3. Configure SSL/TLS mode to "Full (Strict)"
4. Set cache rules for static file extensions (`.css`, `.js`, `.png`, etc.)

---

## Security Hardening

### Firewall Rules

```bash
# UFW (Ubuntu/Debian)
ufw allow 22/tcp    # SSH
ufw allow 80/tcp    # HTTP (redirects to HTTPS)
ufw allow 443/tcp   # HTTPS
ufw deny 8080/tcp   # Block direct Ferro access
ufw deny 3000/tcp   # Block direct Grafana access
ufw deny 9090/tcp   # Block direct Prometheus access
ufw enable

# iptables (alternative)
iptables -A INPUT -p tcp --dport 22 -j ACCEPT
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
iptables -A INPUT -p tcp --dport 8080 -j DROP
iptables -A INPUT -p tcp --dport 3000 -j DROP
iptables -A INPUT -p tcp --dport 9090 -j DROP
```

### Rate Limiting

Caddy supports rate limiting via the `rate_limit` directive:

```text
your-domain.com {
    rate_limit {
        zone api {
            key {remote_host}
            events 100
            window 1m
        }
    }
    reverse_proxy ferro:8080
}
```

### CORS Configuration

Restrict CORS to your domain only:

```bash
FERRO_CORS_ALLOWED_ORIGINS=https://ferro.example.com
FERRO_CORS_ALLOW_CREDENTIALS=true
```

### Content Security Policy

The default Caddyfile includes basic security headers. For a stricter CSP:

```text
header {
    Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' wss://your-domain.com"
}
```

### Vulnerability Scanning

```bash
# Scan Ferro container image
trivy image ghcr.io/wyattau/ferro:latest

# Scan running containers
trivy container ferro

# Scan Docker Compose stack
trivy config deploy/docker-compose.production.yml
```

### Additional Hardening

- Run containers as non-root (Ferro image supports this)
- Use `--no-new-privileges` security option
- Drop all capabilities and add back only required ones
- Set resource limits on all containers (already configured)
- Enable audit logging via Prometheus metrics
- Rotate PostgreSQL and Redis passwords regularly
- Use Docker secrets or a vault for sensitive values

---

## Troubleshooting

### Common Issues

| Problem | Cause | Solution |
|---------|-------|----------|
| `Connection refused` on :8080 | Ferro not started | Check `docker compose logs ferro` |
| `502 Bad Gateway` | Caddy can't reach Ferro | Ensure Ferro healthcheck passes: `docker inspect ferro` |
| ` certificate issuer` errors | DNS not pointing to server | Verify `dig your-domain.com` returns your server IP |
| `FATAL: password authentication failed` | Wrong POSTGRES_PASSWORD | Ensure `.env` password matches Ferro's `FERRO_DATABASE_URL` |
| `OOMKilled` | Container memory exceeded | Increase memory limits in compose file |
| Slow WebDAV sync | Missing Redis cache | Add Redis overlay: `-f docker-compose.redis.yml` |
| Federation errors | DNS/TLS issues | Check `ferro_federation_errors_total` metric |

### Log Locations

| Service | Logs | Command |
|---------|------|---------|
| Ferro | stdout (JSON) | `docker compose logs -f ferro` |
| PostgreSQL | stdout | `docker compose logs -f postgres` |
| Redis | stdout | `docker compose logs -f redis` |
| Caddy | stdout (JSON) | `docker compose logs -f caddy` |
| Prometheus | stdout | `docker compose logs -f prometheus` |
| Grafana | stdout | `docker compose logs -f grafana` |

All containers are configured with `json-file` log driver with 10 MB max size and 3 file rotation.

### Health Check Endpoints

```bash
# Basic process health
curl -f http://localhost:8080/healthz

# Readiness (verifies storage + DB + search)
curl -f http://localhost:8080/readyz

# Startup (longer timeout for initial load)
curl -f http://localhost:8080/startupz

# Prometheus metrics
curl http://localhost:8080/metrics

# Container health status
docker inspect --format='{{.State.Health.Status}}' ferro
```

### Performance Debugging

**Check resource usage:**

```bash
docker stats --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\t{{.BlockIO}}"
```

**Inspect Ferro metrics:**

```bash
# Active connections
curl -s http://localhost:8080/metrics | grep ferro_active_connections

# Request rate
curl -s http://localhost:8080/metrics | grep http_requests_total

# Storage health
curl -s http://localhost:8080/metrics | grep ferro_storage_health_status

# Latency histogram
curl -s http://localhost:8080/metrics | grep http_request_duration_seconds_bucket
```

**Grafana dashboards:**

1. Open `http://localhost:3000`
2. Navigate to **Dashboards** > **Browse**
3. Open **Ferro Server Overview** for HTTP, resources, and storage metrics
4. Open **Ferro WebDAV** for DAV-specific performance

### Database Debugging

```bash
# Connect to PostgreSQL
docker exec -it ferro-postgres psql -U ferro ferro

# Check active connections
SELECT count(*) FROM pg_stat_activity WHERE datname = 'ferro';

# Check table sizes
SELECT relname, pg_size_pretty(pg_total_relation_size(relid))
FROM pg_catalog.pg_statio_user_tables
ORDER BY pg_total_relation_size(relid) DESC;
```

---

## Production Checklist

Before going live, verify:

- [ ] `POSTGRES_PASSWORD` changed from default
- [ ] `GRAFANA_ADMIN_PASSWORD` changed from default
- [ ] `DOMAIN` set to your actual domain
- [ ] DNS `A` record points to server IP
- [ ] Ports 80/443 open in firewall
- [ ] TLS working: `curl -I https://your-domain.com/healthz` shows `Strict-Transport-Security`
- [ ] Ferro healthy: `curl -f https://your-domain.com/readyz`
- [ ] Backups scheduled
- [ ] Monitoring dashboards imported
- [ ] Alert notifications configured (Slack/email/PagerDuty)
- [ ] Rate limiting enabled
- [ ] CORS restricted to your domain
- [ ] Container image scanned for vulnerabilities
