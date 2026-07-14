#!/bin/bash
set -euo pipefail

echo "Setting up monitoring..."

# Create monitoring namespace
kubectl create namespace monitoring

# Deploy Prometheus
kubectl apply -f monitoring/prometheus/ -n monitoring

# Deploy Grafana
kubectl apply -f monitoring/grafana/ -n monitoring

# Deploy Alertmanager
kubectl apply -f monitoring/alertmanager/ -n monitoring

echo "Monitoring setup complete."