#!/bin/bash
set -euo pipefail

ENVIRONMENT=${1:-staging}
NAMESPACE=${ENVIRONMENT}

echo "Rolling back deployment in ${ENVIRONMENT}..."

kubectl rollout undo deployment/ferro-server -n "${NAMESPACE}"
kubectl rollout status deployment/ferro-server -n "${NAMESPACE}" --timeout=300s

echo "Rollback complete."
