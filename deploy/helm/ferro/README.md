# Ferro Helm Chart

Self-hosted file server with WebDAV, S3-compatible API, federation, and WASM workers.

## Quick Start

```bash
helm install ferro ./deploy/helm/ferro \
  --set ferro.adminPassword="$(openssl rand -base64 24)" \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=ferro.example.com
```

## Configuration

### Required Settings

| Parameter | Description |
|-----------|-------------|
| `ferro.adminPassword` | Admin password (REQUIRED, use secrets in production) |

### Storage

| Parameter | Default | Description |
|-----------|---------|-------------|
| `ferro.storage.backend` | `local` | Storage backend: local, s3, gcs, azure |
| `ferro.storage.dataDir` | `/data` | Local data directory |
| `ferro.storage.url` | `""` | Backend URL (e.g., s3://bucket/prefix) |
| `persistence.enabled` | `true` | Enable persistent volume |
| `persistence.size` | `10Gi` | PV size |
| `persistence.storageClass` | `""` | Storage class (default: cluster default) |

### Networking

| Parameter | Default | Description |
|-----------|---------|-------------|
| `service.type` | `ClusterIP` | Service type |
| `service.port` | `8080` | Service port |
| `ingress.enabled` | `false` | Enable ingress |
| `ingress.className` | `""` | Ingress class |

### Features

| Parameter | Default | Description |
|-----------|---------|-------------|
| `ferro.wasmEnabled` | `false` | Enable WASM workers |
| `ferro.maxUploadBytes` | `1073741824` | Max upload size (1GB) |
| `ferro.maxFileVersions` | `10` | Max file versions |
| `ferro.rateLimitRpm` | `300` | Rate limit (requests/minute) |
| `ferro.federationSecret` | `""` | Federation secret (empty = disabled) |
| `ferro.metadataDb` | `""` | PostgreSQL URL for metadata |

### Monitoring

| Parameter | Default | Description |
|-----------|---------|-------------|
| `serviceMonitor.enabled` | `false` | Enable Prometheus ServiceMonitor |
| `serviceMonitor.path` | `/metrics` | Metrics endpoint path |
| `serviceMonitor.interval` | `30s` | Scrape interval |

### OIDC

| Parameter | Default | Description |
|-----------|---------|-------------|
| `ferro.oidc.enabled` | `false` | Enable OIDC authentication |
| `ferro.oidc.issuerUrl` | `""` | OIDC issuer URL |
| `ferro.oidc.clientId` | `""` | OIDC client ID |
| `ferro.oidc.clientSecret` | `""` | OIDC client secret |

## Examples

### With S3 storage and ingress

```bash
helm install ferro ./deploy/helm/ferro \
  --set ferro.adminPassword="my-secret-password" \
  --set ferro.storage.url="s3://my-bucket/files" \
  --set ferro.metadataDb="postgresql://user:pass@db:5432/ferro" \
  --set ingress.enabled=true \
  --set ingress.className=nginx \
  --set "ingress.hosts[0].host=files.example.com" \
  --set "ingress.tls[0].secretName=ferro-tls" \
  --set "ingress.tls[0].hosts[0]=files.example.com"
```

### With Prometheus monitoring

```bash
helm install ferro ./deploy/helm/ferro \
  --set ferro.adminPassword="my-secret-password" \
  --set serviceMonitor.enabled=true \
  --set serviceMonitor.labels.release=prometheus
```
