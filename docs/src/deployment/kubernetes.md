# Kubernetes Deployment

Ferro provides two Kubernetes deployment options: a lightweight single-manifest setup via K3s and a full Kustomize-based production deployment.

## K3s (Lightweight)

### Deploy

```bash
kubectl apply -f deploy/k3s/ferro.yaml
```

### With Traefik (K3s default)

K3s ships with Traefik. Add to `/etc/hosts`:

```
127.0.0.1 ferro.local
```

The Ingress is pre-configured for `ferro.local`.

### With PostgreSQL

```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
helm install postgres bitnami/postgresql \
  --set auth.postgresPassword=ferro \
  --namespace ferro
```

Then set `FERRO_DATABASE_URL` in the Deployment.

### Cleanup

```bash
kubectl delete -f deploy/k3s/ferro.yaml
```

## Production (Kustomize)

### Deploy base

```bash
kubectl apply -k deploy/kubernetes/base
```

### What's included

The production deployment includes:

| Resource | Description |
|----------|-------------|
| `Namespace` | Dedicated `ferro` namespace |
| `Deployment` | Ferro pods with resource limits |
| `Service` | ClusterIP service |
| `Ingress` | Ingress with configurable class |
| `PVC` | Persistent volume claim for data |
| `Secret` | Admin credentials |
| `ConfigMap` | Server configuration |
| `PDB` | Pod disruption budget |
| `NetworkPolicy` | Network policies (deny, DNS, external, ingress) |

### Network Policies

The base deployment includes restrictive network policies:

- `networkpolicy-deny.yaml` -- Deny all ingress/egress by default
- `networkpolicy-dns.yaml` -- Allow DNS egress
- `networkpolicy-external.yaml` -- Allow external egress (for S3, OIDC, etc.)
- `networkpolicy-ingress.yaml` -- Allow ingress on service ports

## Helm

### Install

```bash
helm install ferro deploy/helm/ferro
```

### Custom values

```bash
helm install ferro deploy/helm/ferro \
  --set replicaCount=2 \
  --set persistence.size=10Gi \
  --set auth.adminUser=admin \
  --set auth.adminPassword=changeme \
  --set ingress.enabled=true \
  --set ingress.className=nginx
```

### Helm chart values

| Value | Default | Description |
|-------|---------|-------------|
| `replicaCount` | `1` | Number of replicas |
| `persistence.enabled` | `true` | Enable persistent volume |
| `persistence.size` | `5Gi` | Volume size |
| `ingress.enabled` | `false` | Enable Ingress |
| `ingress.className` | `""` | Ingress class name |
| `networkPolicy.enabled` | `true` | Enable network policies |
| `auth.adminUser` | `admin` | Admin username |
| `auth.adminPassword` | `""` | Admin password |
| `image.repository` | `ghcr.io/wyattau/ferro` | Container image |
| `image.tag` | `latest` | Image tag |

## Tips

- Use PDBs to ensure availability during rolling updates
- Network policies provide defense-in-depth
- Set `FERRO_LOG_FORMAT=json` for structured logging
- Use the Helm chart for complex deployments with custom values
