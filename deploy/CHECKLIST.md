# Ferro Production Deployment Checklist

## Prerequisites

- [ ] Docker Engine 24+ and Docker Compose v2 installed
- [ ] `.env` file created from `.env.example` and configured
- [ ] `POSTGRES_PASSWORD` set to a strong, unique value (not `changeme`)
- [ ] `GRAFANA_ADMIN_PASSWORD` set to a strong, unique value (not `changeme`)
- [ ] Domain name DNS A record points to the deployment host

## TLS & Networking

- [ ] Domain name configured in `.env` (DOMAIN variable)
- [ ] Caddy auto-TLS enabled (remove `localhost` for production domain)
- [ ] Ports 80/443 open on firewall/security group
- [ ] `HTTP_PORT` and `HTTPS_PORT` set correctly in `.env`

## Database

- [ ] PostgreSQL credentials match between `.env` and `FERRO_DATABASE_URL`
- [ ] Sufficient disk space for `postgres-data` volume
- [ ] PostgreSQL data directory on fast storage (SSD recommended)

## Monitoring Stack

- [ ] `deploy/monitoring/prometheus.yml` targets are correct
- [ ] `deploy/monitoring/alertmanager.yml` webhook URLs are reachable
- [ ] `deploy/grafana/provisioning/datasources/` contains Prometheus + Loki datasource configs
- [ ] `deploy/grafana/provisioning/dashboards/` contains dashboard provider config
- [ ] Grafana admin credentials changed from defaults
- [ ] Alert rules in `deploy/monitoring/alerts/ferro-alerts.yml` reviewed

## Backup & Recovery

- [ ] PostgreSQL backup schedule configured (e.g., pg_dump cron or volume snapshot)
- [ ] Backup storage location configured (S3, NFS, etc.)
- [ ] Backup retention policy defined
- [ ] Recovery procedure documented and tested

## Pre-Deploy Validation

- [ ] `docker compose -f docker-compose.yml config` succeeds
- [ ] `docker compose -f docker-compose.yml -f docker-compose.pg.yml config` succeeds
- [ ] `docker compose -f docker-compose.yml -f docker-compose.production.yml config` succeeds
- [ ] All images pulled: `docker compose pull`
- [ ] No port conflicts on host (80, 443, 3000, 8080)

## Deploy

```bash
cd deploy
cp .env.example .env   # then edit with real values
docker compose -f docker-compose.yml -f docker-compose.production.yml up -d
```

## Post-Deploy Verification

- [ ] All containers healthy: `docker compose ps`
- [ ] Ferro health endpoint responds: `curl http://localhost:8080/healthz`
- [ ] Grafana accessible at `https://<domain>:3000`
- [ ] Prometheus targets all UP: `http://localhost:9090/targets`
- [ ] Loki ready: `curl http://localhost:3100/ready`
- [ ] No container restart loops: `docker compose logs --tail=50`
