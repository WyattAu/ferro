# Ferro Deployment Strategy

## Environments

### Development
- **Method:** `cargo run -p ferro-server`
- **Storage:** InMemoryStorageEngine (default)
- **Database:** SQLite (optional, for metadata testing)
- **Purpose:** Local development and debugging

### Staging
- **Method:** Docker container
- **Image:** Multi-stage Dockerfile (builder + runtime)
- **Storage:** Local FS backend via object_store
- **Database:** PostgreSQL 16 (single instance)
- **Purpose:** Pre-production validation, rclone compatibility testing
- **Deployment:** `docker compose up` with postgres service dependency

### Production
- **Method:** Kubernetes (Helm chart)
- **Replicas:** 3+ (Horizontal Pod Autoscaler, min 3, max 20)
- **Storage:** Amazon S3 (or GCS/Azure Blob) via object_store
- **Database:** PostgreSQL 16 HA (Patroni or Cloud-managed RDS)
- **Ingress:** NGINX Ingress Controller with TLS termination
- **Purpose:** Production workloads
- **Rollout Strategy:** Rolling update with readiness/liveness probes

## Rollback Strategy
- **Mechanism:** Git-based rollback via `git checkout <previous-commit>` followed by redeployment
- **Kubernetes:** `helm rollback <release> <revision>` for Helm-managed deployments
- **Database Migrations:** Forward-only migrations with down migrations tested in CI
- **SLA Target:** Rollback complete within 5 minutes

## Monitoring

### Structured Logging
- **Crate:** `tracing` + `tracing-subscriber` (JSON format in production)
- **Output:** stdout (collected by container runtime / Fluentd)
- **Levels:** trace, debug, info, warn, error
- **Context:** Request ID, user principal, resource path propagated via tracing spans

### Metrics
- **Format:** Prometheus exposition format
- **Endpoint:** `/metrics` (authenticated, admin-only)
- **Key Metrics:**
  - `ferro_http_requests_total` (method, path, status)
  - `ferro_http_request_duration_seconds` (histogram)
  - `ferro_webdav_operations_total` (method, status)
  - `ferro_cas_operations_total` (operation, result)
  - `ferro_active_locks` (gauge)
  - `ferro_storage_bytes_used` (gauge)

### Health Endpoint
- **Endpoint:** `/health`
- **Checks:** Storage backend connectivity, database connectivity, lock manager status
- **Response:** `200 OK` with JSON body `{ "status": "healthy", "checks": { ... } }`
- **Failure:** `503 Service Unavailable` with failing check details

## Infrastructure Requirements

### Minimum (Staging)
- 2 CPU cores, 4 GB RAM
- 20 GB storage (objects)
- PostgreSQL 16 instance

### Recommended (Production)
- 4 CPU cores per pod, 8 GB RAM per pod
- S3 storage (unlimited, billed)
- PostgreSQL HA (3-node Patroni cluster or managed RDS)
- Redis (optional, for session caching and rate limiting)
