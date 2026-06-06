# Blue-Green Deployment Guide

Deploy Ferro with zero downtime using blue-green deployment strategy.

## Concept

Blue-green deployment maintains two identical production environments:

- **Blue:** Currently serving live traffic
- **Green:** New version being deployed and tested

Traffic switches atomically from blue to green after validation.

## Docker Compose Setup

```yaml
# docker-compose.bluegreen.yml
services:
  ferro-blue:
    image: ghcr.io/wyattau/ferro:current
    environment:
      - FERRO_DATA_DIR=/data
      - FERRO_ADMIN_USER=admin
      - FERRO_ADMIN_PASSWORD=${BLUE_PASSWORD}
    volumes:
      - ferro-data:/data
    expose:
      - "8080"
    restart: unless-stopped

  ferro-green:
    image: ghcr.io/wyattau/ferro:next
    environment:
      - FERRO_DATA_DIR=/data
      - FERRO_ADMIN_USER=admin
      - FERRO_ADMIN_PASSWORD=${GREEN_PASSWORD}
    volumes:
      - ferro-data:/data
    expose:
      - "8081"
    restart: unless-stopped

  caddy:
    image: caddy:2-alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
    restart: unless-stopped

volumes:
  ferro-data:
    driver: local
  caddy-data:
  caddy-config:
```

## Caddyfile Configuration

```
# Active: blue, Standby: green
ferro.example.com {
    reverse_proxy ferro-blue:8080
}
```

## Deployment Steps

### 1. Deploy Green Version

```bash
# Pull new image
docker pull ghcr.io/wyattau/ferro:next

# Start green environment
docker compose -f docker-compose.bluegreen.yml up -d ferro-green
```

### 2. Validate Green

```bash
# Run smoke tests against green
curl -sf http://localhost:8081/.well-known/ferro | jq .

# Run E2E tests
BASE_URL=http://localhost:8081 npx playwright test

# Monitor for errors
docker logs ferro-green --tail 50
```

### 3. Switch Traffic

```bash
# Update Caddyfile to point to green
sed -i 's/ferro-blue:8080/ferro-green:8081/' Caddyfile

# Reload Caddy (zero-downtime)
docker kill --signal=USR1 caddy

# Verify
curl -sf https://ferro.example.com/.well-known/ferro | jq .
```

### 4. Decommission Blue

```bash
# Stop old environment
docker compose -f docker-compose.bluegreen.yml stop ferro-blue

# Or keep as rollback target
docker compose -f docker-compose.bluegreen.yml pause ferro-blue
```

## Rollback

If issues are detected after switching:

```bash
# Revert Caddyfile
sed -i 's/ferro-green:8081/ferro-blue:8080/' Caddyfile

# Reload Caddy
docker kill --signal=USR1 caddy
```

## Database Migration Considerations

When performing database migrations during blue-green deployment:

1. **Backward-compatible migrations only** -- both blue and green must work with the same schema
2. **Deploy migration first** -- run migrations before switching traffic
3. **Test with both versions** -- verify both blue and green work with the new schema

```bash
# Run migration on the shared database
docker compose exec ferro-green ferro-server --migrate-from sqlite
```

## Kubernetes Setup

For Kubernetes, use separate Deployments:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ferro-blue
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ferro
      version: blue
  template:
    metadata:
      labels:
        app: ferro
        version: blue
    spec:
      containers:
        - name: ferro
          image: ghcr.io/wyattau/ferro:current
          ports:
            - containerPort: 8080
```

Switch traffic using a Service selector update:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: ferro
spec:
  selector:
    app: ferro
    version: green  # Change from blue to green
  ports:
    - port: 80
      targetPort: 8080
```
