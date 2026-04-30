# K3s Deployment

Single-command deployment on K3s:

```bash
kubectl apply -f deploy/k3s/ferro.yaml
```

## With Traefik (K3s default)

K3s ships with Traefik. The Ingress is pre-configured for `ferro.local`.
Add to your `/etc/hosts`: `127.0.0.1 ferro.local`

## With PostgreSQL

```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
helm install postgres bitnami/postgresql --set auth.postgresPassword=ferro --namespace ferro
# Then set FERRO_DATABASE_URL env in the Deployment
```

## Cleanup

```bash
kubectl delete -f deploy/k3s/ferro.yaml
```
