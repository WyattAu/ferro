# Ferro Ecosystem Deployment

## Packages

| Package | Binary | Purpose |
|---|---|---|
| ferro-server | `ferro-server` | Core storage server with WebDAV, API, and metrics endpoints |
| ferro-web | static files (nginx) | User-facing file browser |
| ferro-admin | static files (nginx) | Admin panel for system management |
| ferro-cli | `ferro-cli` | Admin CLI tool (included in server image) |
| ferro-fuse | `ferro-fuse` | FUSE filesystem mount (run outside Docker) |
| ferro-observability | endpoints in ferro-server | `/metrics` and `/healthz` endpoints for monitoring |

## Quick Start

### Full Ecosystem (Grafana + Loki + Prometheus)

```bash
cd deploy
docker compose -f docker-compose.ecosystem.yml up -d
```

### Full Ecosystem (VictoriaMetrics + VictoriaLogs)

```bash
cd deploy
docker compose -f docker-compose.ecosystem-vm.yml up -d
```

### Minimal (Server + Web UI only)

```bash
cd deploy
docker compose -f docker-compose.minimal.yml up -d
```

## Switching Between Monitoring Stacks

Both ecosystem files include the same core services (ferro-server, ferro-web, ferro-admin). The difference is the monitoring backend:

| Stack | Compose File | Metrics | Logs | Dashboard |
|---|---|---|---|---|
| Grafana + Loki | `docker-compose.ecosystem.yml` | Prometheus | Loki | Grafana |
| VictoriaMetrics | `docker-compose.ecosystem-vm.yml` | VictoriaMetrics + vmagent | VictoriaLogs | Grafana |

To switch stacks, stop one and start the other:

```bash
docker compose -f docker-compose.ecosystem.yml down
docker compose -f docker-compose.ecosystem-vm.yml up -d
```

Both stacks share the same `prometheus-ecosystem.yml` scrape config, so vmagent can scrape the same targets.

## Compose File Reference

| File | Services | Use Case |
|---|---|---|
| `docker-compose.yml` | ferro-server only | Base deployment |
| `docker-compose.minimal.yml` | ferro-server + ferro-web | Lightweight file browser |
| `docker-compose.ecosystem.yml` | All packages + Grafana/Loki/Prometheus | Full self-hosted cloud |
| `docker-compose.ecosystem-vm.yml` | All packages + VictoriaMetrics/VictoriaLogs | Full stack, Victoria backend |
| `docker-compose.full.yml` | Base + PostgreSQL + Redis | Extended storage backends |
| `docker-compose.pg.yml` | PostgreSQL addon | Production metadata DB |
| `docker-compose.redis.yml` | Redis addon | Distributed locks |

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `FERRO_ADMIN_PASSWORD` | `changeme` | Admin password for ferro-server |
| `FERRO_ADMIN_USER` | `admin` | Admin username for ferro-server |
| `GRAFANA_PASSWORD` | `admin` | Grafana admin password |
| `RUST_LOG` | `info` | Rust log level for ferro-server |

Set them before starting:

```bash
export FERRO_ADMIN_PASSWORD=your-secure-password
export GRAFANA_PASSWORD=your-grafana-password
docker compose -f docker-compose.ecosystem.yml up -d
```

## Port Mapping

| Port | Service | Description |
|---|---|---|
| 80 | ferro-web | User-facing file browser |
| 8080 | ferro-server | Core API, WebDAV, and metrics |
| 8081 | ferro-admin | Admin panel |
| 3000 | grafana | Monitoring dashboards |
| 9090 | prometheus | Prometheus UI (ecosystem only) |
| 8428 | victoria-metrics | VictoriaMetrics UI (VM ecosystem only) |
| 9428 | victoria-logs | VictoriaLogs API (VM ecosystem only) |
| 3100 | loki | Loki log API (Grafana/Loki ecosystem only) |

## Volumes

| Volume | Used By | Content |
|---|---|---|
| `ferro-data` | ferro-server | Persistent file storage and metadata |
| `loki-data` | loki | Log storage (Grafana/Loki stack) |
| `prometheus-data` | prometheus | Metrics storage (Grafana/Loki stack) |
| `grafana-data` | grafana | Dashboard configurations |
| `vm-data` | victoria-metrics | Metrics storage (VM stack) |
| `vl-data` | victoria-logs | Log storage (VM stack) |

## Individual Package Deployment

### ferro-server only

```bash
docker compose up -d
```

### ferro-server + web UI

```bash
docker compose -f docker-compose.minimal.yml up -d
```

### Monitoring only (standalone)

```bash
cd monitoring
docker compose -f docker-compose.grafana-loki.yml up -d
# or
docker compose -f docker-compose.victoria.yml up -d
```

### ferro-fuse

ferro-fuse requires host-level access to the FUSE device. Run it outside Docker:

```bash
./ferro-fuse --server http://localhost:8080 --mount /mnt/ferro
```

### ferro-cli

The CLI is included in the server image. Execute commands inside the container:

```bash
docker exec ferro-server /app/ferro-cli --help
```

Or build and run locally:

```bash
cargo build --release --bin ferro-cli
./target/release/ferro-cli --server http://localhost:8080
```
