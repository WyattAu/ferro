# Ferro Deployment Guide

Ferro can be deployed on multiple platforms. All configurations are modular — enable only the components you need.

## Quick Start

### Docker Compose (single-node, minimal)

```bash
cd deploy
docker compose up -d
```

### Docker Compose with PostgreSQL

```bash
POSTGRES_PASSWORD=your-password docker compose -f docker-compose.yml -f docker-compose.pg.yml up -d
```

### Docker Compose with PostgreSQL + Redis

```bash
POSTGRES_PASSWORD=your-password docker compose -f docker-compose.yml -f docker-compose.pg.yml -f docker-compose.redis.yml up -d
```

## Platform-Specific Guides

| Platform | Directory | Description |
|---|---|---|
| Docker Compose | `deploy/` | Single-node, layered overlays |
| Podman | `deploy/podman/` | Rootless containers with systemd |
| Firecracker | `deploy/firecracker/` | MicroVM with ~125ms boot |
| K3s | `deploy/k3s/` | Lightweight Kubernetes, single manifest |
| Kubernetes | `deploy/kubernetes/` | Production K8s with Kustomize |
| Helm | `deploy/helm/` | Helm chart for K8s |
| Terraform | `deploy/terraform/` | Infrastructure as Code |

## Docker Compose Layers

The Docker Compose configs use a layered overlay pattern:

- `docker-compose.yml` — Base: Ferro only
- `docker-compose.pg.yml` — Overlay: Adds PostgreSQL
- `docker-compose.redis.yml` — Overlay: Adds Redis
- `docker-compose.full.yml` — Documents full stack composition

Combine layers with `-f` flags. Environment variables control configuration:

| Variable | Default | Description |
|---|---|---|
| `FERRO_PORT` | `8080` | Host port mapping |
| `POSTGRES_PASSWORD` | `ferro` | PostgreSQL password |

## Podman

Rootless containers with SELinux support. See [`deploy/podman/README.md`](podman/README.md).

```bash
cd deploy/podman
podman-compose -f podman-compose.yml up -d
```

## Firecracker MicroVM

VM-level isolation with minimal attack surface. See [`deploy/firecracker/README.md`](firecracker/README.md).

```bash
cd deploy/firecracker
chmod +x start-vm.sh
sudo ./start-vm.sh
```

## K3s

Single manifest deployment using K3s built-in Traefik. See [`deploy/k3s/README.md`](k3s/README.md).

```bash
kubectl apply -f deploy/k3s/ferro.yaml
```

## Kubernetes (Production)

Full Kustomize-based deployment with network policies, PDBs, and ingress. See [`deploy/kubernetes/`](kubernetes/).

```bash
kubectl apply -k deploy/kubernetes/base
```

## Helm

Deploy via Helm chart with full configuration options. See [`deploy/helm/`](helm/).

```bash
helm install ferro deploy/helm/ferro
```

## Terraform

Infrastructure as Code for Kubernetes or K3s. See [`deploy/terraform/`](terraform/).

```bash
cd deploy/terraform
terraform init
terraform plan -var="admin_password=your-password"
terraform apply -var="admin_password=your-password"
```

### Terraform K3s Module

```bash
cd deploy/terraform/k3s
terraform init
terraform apply
```

## Security

All deployments follow security best practices:
- Non-root containers where supported
- `no-new-privileges` security option
- `cap-drop: ALL` with minimal capabilities added back
- Resource limits on all containers
- Health checks on all services
- No secrets in configuration files — use environment variables
