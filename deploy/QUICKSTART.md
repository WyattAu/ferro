# Ferro Production Stack - Quick Start

## Prerequisites

- Docker Engine 24.0+ (or Docker Desktop 4.25+)
- Docker Compose v2.20+
- 4 GB RAM minimum (6 GB recommended with monitoring)
- Ports 80, 443, 3000, 9090 available

## 1. Clone and Configure

```bash
git clone https://github.com/WyattAu/ferro.git
cd ferro/deploy
cp .env.example .env
```

Edit `.env` with your values:

```bash
# Required
POSTGRES_PASSWORD=<your-secure-password>
GRAFANA_ADMIN_PASSWORD=<your-secure-password>

# Optional (for custom domain)
DOMAIN=ferro.example.com
GRAFANA_ROOT_URL=https://ferro.example.com:3000
```

## 2. Start the Stack

```bash
docker compose -f docker-compose.yml -f docker-compose.production.yml up -d
```

This starts:
- **ferro** - Main application
- **postgres** - PostgreSQL database
- **redis** - Caching layer
- **caddy** - Reverse proxy with TLS
- **prometheus** - Metrics collection
- **grafana** - Dashboards (port 3000)
- **loki** - Log aggregation
- **alertmanager** - Alert routing

## 3. Verify It's Running

```bash
# Check all services are healthy
docker compose -f docker-compose.yml -f docker-compose.production.yml ps

# Watch logs
docker compose -f docker-compose.yml -f docker-compose.production.yml logs -f ferro

# Test health endpoint
curl http://localhost:8080/healthz
```

## 4. Access the Web UI

| Service  | URL                    | Credentials              |
|----------|------------------------|--------------------------|
| Ferro    | http://localhost       | -                        |
| Grafana  | http://localhost:3000  | admin / (your password)  |
| Prometheus | http://localhost:9090 | -                        |

For a custom domain, set `DOMAIN` in `.env` and update DNS. Caddy will auto-provision TLS via Let's Encrypt.

## 5. Common Issues

### Container fails to start

```bash
# Check logs for the specific service
docker compose -f docker-compose.yml -f docker-compose.production.yml logs <service>
```

### Port conflicts

```bash
# Find what's using port 80
lsof -i :80
# Change HTTP_PORT in .env to a free port (e.g., 8080)
```

### Database connection refused

PostgreSQL needs ~10 seconds to initialize. If ferro starts too fast, restart it:

```bash
docker compose -f docker-compose.yml -f docker-compose.production.yml restart ferro
```

### Out of memory

Reduce monitoring overhead or increase limits:

```bash
# Disable monitoring services if not needed
docker compose -f docker-compose.yml -f docker-compose.production.yml up -d \
  ferro postgres redis caddy
```

### TLS certificate errors (Let's Encrypt)

- Ensure port 80 is reachable from the internet
- Ensure your DNS A record points to this server
- Caddy retries automatically; wait 5 minutes
- For testing, use `DOMAIN=localhost` (Caddy uses self-signed certs)

## Teardown

```bash
# Stop all services
docker compose -f docker-compose.yml -f docker-compose.production.yml down

# Stop and remove volumes (destroys data)
docker compose -f docker-compose.yml -f docker-compose.production.yml down -v
```
