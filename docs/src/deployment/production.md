# Production Deployment Guide

Step-by-step guide for deploying Ferro in a production environment.

## Prerequisites

- A server or VM with Linux (amd64 or arm64)
- At least 512MB RAM (1GB recommended for >100 concurrent users)
- Persistent storage volume for data directory
- TLS certificate (via Let's Encrypt, Caddy, or reverse proxy)
- Domain name (for federation and WebDAV clients)

## 1. Choose Your Storage Backend

| Backend | Best For | Scalability | Persistence |
|---------|----------|-------------|-------------|
| Local filesystem (`--data-dir`) | Single node, small deployments | Vertical only | Filesystem |
| PostgreSQL (`postgresql://`) | Multi-user, >100 concurrent | Horizontal read replicas | Database + filesystem |
| S3 (`s3://`) | Cloud-native, scalable | Unlimited | Object store |
| GCS (`gs://`) | Google Cloud | Unlimited | Object store |
| Azure Blob (`az://`) | Azure Cloud | Unlimited | Object store |

For production, we recommend **PostgreSQL + local filesystem** or **S3** for maximum reliability.

## 2. Deploy with Docker (Recommended)

### 2.1 Create Configuration

```bash
mkdir -p /opt/ferro && cd /opt/ferro

# Create config file
cat > ferro.toml << 'EOF'
[server]
bind = "0.0.0.0"
port = 8080
admin_user = "admin"
admin_password = "CHANGE-ME-USE-SECRETS"

[storage]
backend = "local"
data_dir = "/data"

[security]
rate_limit_rpm = 300
max_upload_bytes = 1073741824

[logging]
level = "info"
format = "json"
EOF
```

### 2.2 Docker Compose with TLS (Caddy)

```yaml
# docker-compose.yml
services:
  ferro:
    image: ghcr.io/wyattau/ferro:latest
    container_name: ferro
    restart: unless-stopped
    volumes:
      - ferro-data:/data
      - ./ferro.toml:/etc/ferro/ferro.toml:ro
    environment:
      FERRO_CONFIG: /etc/ferro/ferro.toml
    ports:
      - "127.0.0.1:8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

  caddy:
    image: caddy:2
    container_name: caddy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - caddy-data:/data
      - caddy-config:/config
      - ./Caddyfile:/etc/caddy/Caddyfile:ro

volumes:
  ferro-data:
  caddy-data:
  caddy-config:
```

```text
# Caddyfile
your-domain.com {
    reverse_proxy ferro:8080
}
```

### 2.3 Start Services

```bash
docker compose up -d
docker compose logs -f ferro  # verify startup
curl -f http://localhost:8080/healthz  # should return OK
```

## 3. Deploy on Kubernetes

See the [Kubernetes deployment guide](./kubernetes.md) for K3s single-node and Kustomize production manifests.

Key resources:
- Readiness: `/readyz` (verifies storage + DB + search)
- Liveness: `/healthz` (basic process health)
- Startup: `/startupz` (longer timeout for initial load)
- Metrics: `/metrics` (Prometheus format)

## 4. Deploy Bare Metal

```bash
# Download binary
curl -L https://github.com/WyattAu/ferro/releases/latest/download/ferro-server -o /usr/local/bin/ferro-server
chmod +x /usr/local/bin/ferro-server

# Create data directory
mkdir -p /var/lib/ferro

# Create systemd service
cat > /etc/systemd/system/ferro.service << 'EOF'
[Unit]
Description=Ferro File Server
After=network.target

[Service]
Type=simple
User=ferro
Group=ferro
ExecStart=/usr/local/bin/ferro-server \
  --data-dir /var/lib/ferro \
  --admin-user admin \
  --admin-password ${FERRO_ADMIN_PASSWORD} \
  --port 8080
Restart=on-failure
RestartSec=5
Environment=FERRO_LOG_FORMAT=json
EnvironmentFile=/etc/ferro/env

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now ferro
```

## 5. Production Checklist

### Security

- [ ] Change default admin password
- [ ] Enable TLS (Caddy, nginx, or direct via `--tls-cert` / `--tls-key`)
- [ ] Set `--admin-password` via environment variable or secrets manager, not CLI
- [ ] Configure rate limiting (`--rate-limit-rpm`)
- [ ] Set upload size limits (`--max-upload-bytes`)
- [ ] Review and customize Content-Security-Policy headers
- [ ] Configure CORS origins if using web UI
- [ ] Enable federation secret if using ActivityPub (`--federation-secret`)

### Reliability

- [ ] Use persistent storage volume (not container ephemeral storage)
- [ ] Configure health checks (`/healthz`, `/readyz`, `/startupz`)
- [ ] Set restart policy (`unless-stopped` or systemd restart)
- [ ] Enable WAL mode for SQLite (default when using `--data-dir`)
- [ ] Set up database backups (see `/api/admin/backup` endpoint)
- [ ] Configure log aggregation (JSON format via `FERRO_LOG_FORMAT=json`)

### Monitoring

- [ ] Configure Prometheus to scrape `/metrics`
- [ ] Import Grafana dashboard from `docs/src/deployment/grafana-dashboard.json`
- [ ] Set up alerts for:
  - `ferro_up` == 0 (server down)
  - Error rate > 5% on any endpoint
  - Storage backend latency p99 > 500ms
  - Cache hit rate < 50%

### Performance

- [ ] Use PostgreSQL for >100 concurrent users
- [ ] Tune `--max-upload-bytes` for your use case
- [ ] Enable read cache (default when using `--data-dir`)
- [ ] Monitor WASM worker fuel consumption via `/metrics`

## 6. Verification

After deployment, verify everything works:

```bash
# Health check
curl -f https://your-domain.com/healthz

# Readiness check (verifies all subsystems)
curl -f https://your-domain.com/readyz

# Authentication
curl -u admin:your-password https://your-domain.com/api/storage/stats

# File upload
curl -X PUT https://your-domain.com/test.txt \
  -u admin:your-password \
  -H "Content-Type: text/plain" \
  -d "Hello, production!"

# File download
curl https://your-domain.com/test.txt -u admin:your-password

# Metrics
curl https://your-domain.com/metrics -u admin:your-password | head -20

# Security headers
curl -I https://your-domain.com/healthz | grep -i strict-transport
```

## 7. Backup and Recovery

### Automated Backups

```bash
# Trigger backup via API
curl -X POST https://your-domain.com/api/admin/backup \
  -u admin:your-password

# Schedule with cron (daily at 2am)
0 2 * * * curl -s -X POST https://localhost:8080/api/admin/backup -u admin:password > /dev/null
```

### Disaster Recovery

1. Stop the Ferro server
2. Restore data directory from backup
3. Restore SQLite database (if using local storage)
4. Restart the server
5. Verify via `/readyz`

## 8. Scaling

| Users | Recommended Setup |
|-------|-------------------|
| 1-10 | Single node, local storage, SQLite |
| 10-100 | Single node, local storage, PostgreSQL |
| 100-1000 | Multiple nodes behind load balancer, shared PostgreSQL + S3 |
| 1000+ | Kubernetes with horizontal pod autoscaling, PostgreSQL HA, S3 |

For multi-node deployments, use a shared storage backend (S3, GCS, or Azure Blob) and external PostgreSQL. Each node runs stateless Ferro with `--storage-url s3://bucket` and `--metadata-db postgresql://...`.
