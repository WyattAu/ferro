# Multi-Region Deployment Guide

## Prerequisites

- AWS account with multi-region access (us-east-1, us-west-2, eu-west-1, ap-northeast-1)
- Kubernetes cluster in each region (EKS or self-managed)
- Terraform >= 1.5
- Helm >= 3.12
- kubectl configured for all clusters
- S3 bucket per region with cross-region replication enabled
- Domain configured with CloudFlare or Route 53

## Directory Structure

```
deploy/
├── terraform/
│   ├── multi-region/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   ├── outputs.tf
│   │   ├── regions.tfvars
│   │   └── modules/
│   │       ├── eks/
│   │       ├── rds/
│   │       └── s3/
├── kubernetes/
│   ├── base/
│   └── overlays/
│       ├── us-east-1/
│       ├── us-west-2/
│       ├── eu-west-1/
│       └── ap-northeast-1/
└── helm/
    └── ferro/
        └── values-multi-region.yaml
```

## Deployment Steps

### Step 1: Infrastructure Provisioning

```bash
# Initialize Terraform
cd deploy/terraform/multi-region
terraform init

# Plan infrastructure
terraform plan -var-file=regions.tfvars

# Apply infrastructure
terraform apply -var-file=regions.tfvars

# Verify infrastructure
terraform output
```

### Step 2: Kubernetes Cluster Setup

```bash
# Configure kubectl for each region
aws eks update-kubeconfig --name ferro-us-east-1 --region us-east-1
aws eks update-kubeconfig --name ferro-us-west-2 --region us-west-2
aws eks update-kubeconfig --name ferro-eu-west-1 --region eu-west-1
aws eks update-kubeconfig --name ferro-ap-northeast-1 --region ap-northeast-1

# Verify clusters
for context in $(kubectl config get-contexts -o name); do
  echo "=== $context ==="
  kubectl --context=$context get nodes
done
```

### Step 3: Database Deployment

**Deploy PostgreSQL primary (US East):**

```bash
# Deploy primary
kubectl --context=us-east-1 apply -f k8s/database/primary.yaml

# Wait for primary ready
kubectl --context=us-east-1 rollout status statefulset/postgres -n ferro

# Deploy replicas in other regions
kubectl --context=us-west-2 apply -f k8s/database/replica-us-west.yaml
kubectl --context=eu-west-1 apply -f k8s/database/replica-eu-west.yaml
kubectl --context=ap-northeast-1 apply -f k8s/database/replica-ap-northeast.yaml

# Verify replication
kubectl --context=us-east-1 exec -it postgres-0 -n ferro -- \
  psql -c "SELECT * FROM pg_stat_replication;"
```

### Step 4: Storage Replication Setup

```bash
# Create S3 buckets with cross-region replication
aws s3api create-bucket \
  --bucket ferro-data-us-east-1 \
  --region us-east-1

aws s3api create-bucket \
  --bucket ferro-data-us-west-2 \
  --region us-west-2

# Configure replication rules
aws s3api put-bucket-replication \
  --bucket ferro-data-us-east-1 \
  --replication-configuration file://replication-config.json
```

### Step 5: Application Deployment

**Deploy to primary region first:**

```bash
# Deploy to US East (primary)
kubectl --context=us-east-1 apply -k deploy/kubernetes/overlays/us-east-1/

# Verify deployment
kubectl --context=us-east-1 rollout status deployment/ferro-server -n ferro

# Verify health
curl -sf https://us-east.ferro.com/healthz
```

**Deploy to secondary regions:**

```bash
# Deploy to US West
kubectl --context=us-west-2 apply -k deploy/kubernetes/overlays/us-west-2/
kubectl --context=us-west-2 rollout status deployment/ferro-server -n ferro

# Deploy to EU West
kubectl --context=eu-west-1 apply -k deploy/kubernetes/overlays/eu-west-1/
kubectl --context=eu-west-1 rollout status deployment/ferro-server -n ferro

# Deploy to Asia Pacific (standby)
kubectl --context=ap-northeast-1 apply -k deploy/kubernetes/overlays/ap-northeast-1/
kubectl --context=ap-northeast-1 rollout status deployment/ferro-server -n ferro
```

### Step 6: Load Balancer Configuration

```bash
# Update CloudFlare DNS for geo-routing
# Or configure Route 53 health checks and failover

# Verify routing
for region in us-east us-west eu-west ap-northeast; do
  echo "=== $region ==="
  curl -sf https://${region}.ferro.com/healthz && echo " OK" || echo " FAILED"
done
```

### Step 7: Monitoring Setup

```bash
# Deploy Prometheus + Grafana per region
kubectl --context=us-east-1 apply -f k8s/monitoring/prometheus.yaml
kubectl --context=us-east-1 apply -f k8s/monitoring/grafana.yaml

# Deploy cross-region dashboards
kubectl --context=us-east-1 apply -f k8s/monitoring/cross-region-dashboard.yaml

# Configure alerts
kubectl --context=us-east-1 apply -f k8s/alerts/replication-lag.yaml
kubectl --context=us-east-1 apply -f k8s/alerts/failover-detected.yaml
```

## Verification

### Health Checks

```bash
# Check all regions
for region in us-east-1 us-west-2 eu-west-1 ap-northeast-1; do
  echo "=== $region ==="
  kubectl --context=$region get pods -n ferro
  kubectl --context=$region get ingress -n ferro
done
```

### Replication Status

```bash
# Check database replication lag
kubectl --context=us-east-1 exec -it postgres-0 -n ferro -- \
  psql -c "SELECT client_addr, state, replay_lag FROM pg_stat_replication;"

# Check S3 replication metrics
aws s3api get-bucket-replication --bucket ferro-data-us-east-1
```

### Load Testing

```bash
# Run load test across regions
k6 run \
  --vus 100 \
  --duration 5m \
  --out influxdb=http://monitoring.ferro.internal:8086/k6 \
  benchmarks/k6/multi_region_load_test.js
```

## Blue-Green Deployment

### Deploy New Version

```bash
# 1. Deploy to secondary region (US West) first
kubectl --context=us-west-2 set image deployment/ferro-server \
  ferro-server=ghcr.io/wyattau/ferro:v3.2.0 -n ferro

# 2. Verify health
kubectl --context=us-west-2 rollout status deployment/ferro-server -n ferro
curl -sf https://us-west.ferro.com/healthz

# 3. Monitor for 15 minutes
# Check error rates, latency, logs

# 4. If healthy, deploy to primary (US East)
kubectl --context=us-east-1 set image deployment/ferro-server \
  ferro-server=ghcr.io/wyattau/ferro:v3.2.0 -n ferro

# 5. Deploy to remaining regions
for ctx in eu-west-1 ap-northeast-1; do
  kubectl --context=$ctx set image deployment/ferro-server \
    ferro-server=ghcr.io/wyattau/ferro:v3.2.0 -n ferro
done
```

### Rollback

```bash
# Rollback all regions
for ctx in us-east-1 us-west-2 eu-west-1 ap-northeast-1; do
  kubectl --context=$ctx rollout undo deployment/ferro-server -n ferro
done
```

## Canary Deployment

### Gradual Traffic Shift

```bash
# 1. Deploy canary (10% traffic)
kubectl apply -f k8s/canary/ferro-canary.yaml

# 2. Monitor for 30 minutes
# Check error rates, latency

# 3. Increase to 25%
kubectl patch deployment ferro-canary -n ferro -p \
  '{"spec":{"replicas":2}}'

# 4. Increase to 50%
kubectl patch deployment ferro-canary -n ferro -p \
  '{"spec":{"replicas":5}}'

# 5. Full deployment (100%)
kubectl --context=us-east-1 set image deployment/ferro-server \
  ferro-server=ghcr.io/wyattau/ferro:v3.2.0 -n ferro
```

## Rollback Procedures

### Application Rollback

```bash
# Rollback to previous version
kubectl --context=us-east-1 rollout undo deployment/ferro-server -n ferro

# Rollback to specific version
kubectl --context=us-east-1 rollout undo deployment/ferro-server \
  --to-revision=5 -n ferro
```

### Infrastructure Rollback

```bash
# Rollback Terraform
cd deploy/terraform/multi-region
terraform plan -var-file=rollback.tfvars
terraform apply -var-file=rollback.tfvars
```

### Database Rollback

```bash
# Restore from backup
pg_restore -h <primary-host> -U ferro -d ferro \
  --target-time="2026-07-10 14:30:00" \
  /backups/ferro-latest.dump
```

## Operational Runbooks

| Scenario | Runbook |
|----------|---------|
| Region failure | [failover.md](failover.md) |
| Replication lag | [data_replication.md](data_replication.md#monitoring-replication) |
| Database failover | [../runbooks/database_migration.md](../runbooks/database_migration.md) |
| Incident response | [../runbooks/incident_response.md](../runbooks/incident_response.md) |
| Backup recovery | [../runbooks/backup_recovery.md](../runbooks/backup_recovery.md) |
