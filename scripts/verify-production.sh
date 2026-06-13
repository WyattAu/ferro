#!/usr/bin/env bash
# verify-production.sh - Validate the Ferro production Docker stack
set -uo pipefail

DEPLOY_DIR="$(cd "$(dirname "$0")/../deploy" && pwd)"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PASS=0
FAIL=0
WARN=0

pass()  { ((PASS++)) || true; echo -e "  ${GREEN}PASS${NC}  $1"; }
fail()  { ((FAIL++)) || true; echo -e "  ${RED}FAIL${NC}  $1"; }
warn()  { ((WARN++)) || true; echo -e "  ${YELLOW}WARN${NC}  $1"; }

echo "=== Ferro Production Stack Verification ==="
echo ""

# ── 1. Docker Compose syntax validation ──────────────────────
echo "--- Docker Compose Validation ---"

validate_compose() {
    local label="$1"; shift
    local rc=0
    timeout 10 docker compose "$@" config > /dev/null 2>&1 || rc=$?
    if [[ $rc -eq 0 ]]; then
        pass "$label"
    elif ! command -v docker &>/dev/null; then
        warn "$label (docker not available - skipped)"
    else
        fail "$label (exit code $rc)"
    fi
}

validate_compose "docker-compose.yml (base)" \
    -f "$DEPLOY_DIR/docker-compose.yml"

validate_compose "docker-compose.yml + pg overlay" \
    -f "$DEPLOY_DIR/docker-compose.yml" \
    -f "$DEPLOY_DIR/docker-compose.pg.yml"

validate_compose "docker-compose.yml + production overlay" \
    -f "$DEPLOY_DIR/docker-compose.yml" \
    -f "$DEPLOY_DIR/docker-compose.production.yml"

validate_compose "docker-compose.production.yml (standalone)" \
    -f "$DEPLOY_DIR/docker-compose.production.yml"

echo ""

# ── 2. Required files check ──────────────────────────────────
echo "--- Required Files ---"

check_file() {
    local path="$1"
    local label="${2:-$1}"
    if [[ -f "$DEPLOY_DIR/$path" ]]; then
        pass "$label"
    else
        fail "$label"
    fi
}

check_file "docker-compose.yml"
check_file "docker-compose.production.yml"
check_file "docker-compose.pg.yml"
check_file "Caddyfile"
check_file ".env.example"

echo ""

# ── 3. Dockerfiles ───────────────────────────────────────────
echo "--- Dockerfiles ---"

check_file "Dockerfile.admin"  "Dockerfile.admin"
check_file "Dockerfile.web"    "Dockerfile.web"
check_file "entrypoint.sh"     "entrypoint.sh"

echo ""

# ── 4. Monitoring configs ────────────────────────────────────
echo "--- Monitoring Configs ---"

check_file "monitoring/prometheus.yml"    "prometheus.yml"
check_file "monitoring/alertmanager.yml"  "alertmanager.yml"
check_file "monitoring/loki-config.yml"   "loki-config.yml"

echo ""

# ── 5. Alert rules ───────────────────────────────────────────
echo "--- Alert Rules ---"

check_file "monitoring/alerts/ferro-alerts.yml"    "ferro-alerts.yml"
check_file "monitoring/alerts/application.yml"      "application.yml"
check_file "monitoring/alerts/infrastructure.yml"   "infrastructure.yml"

echo ""

# ── 6. Grafana provisioning ──────────────────────────────────
echo "--- Grafana Provisioning ---"

check_file "grafana/provisioning/dashboards/default.yml"              "dashboards provider"
check_file "grafana/provisioning/datasources/prometheus.yml"          "prometheus datasource"

echo ""

# ── 7. Caddyfile syntax (basic checks) ──────────────────────
echo "--- Caddyfile Syntax ---"

if [[ -f "$DEPLOY_DIR/Caddyfile" ]]; then
    CF="$DEPLOY_DIR/Caddyfile"
    if grep -q 'reverse_proxy' "$CF"; then
        pass "Caddyfile has reverse_proxy directive"
    else
        fail "Caddyfile missing reverse_proxy directive"
    fi

    if grep -q 'header' "$CF"; then
        pass "Caddyfile has security headers"
    else
        warn "Caddyfile missing security headers"
    fi

    if grep -q 'encode' "$CF"; then
        pass "Caddyfile has encoding enabled"
    else
        warn "Caddyfile missing gzip/encoding"
    fi

    if grep -q 'log' "$CF"; then
        pass "Caddyfile has logging configured"
    else
        warn "Caddyfile missing logging"
    fi
else
    fail "Caddyfile not found"
fi

echo ""

# ── 8. Volume definitions check ──────────────────────────────
echo "--- Volume Definitions in Production Stack ---"

if grep -q 'volumes:' "$DEPLOY_DIR/docker-compose.production.yml"; then
    volumes=(
        ferro-data postgres-data redis-data
        caddy-data caddy-config loki-data
        prometheus-data grafana-data
    )
    for v in "${volumes[@]}"; do
        if grep -q "$v:" "$DEPLOY_DIR/docker-compose.production.yml"; then
            pass "Volume: $v"
        else
            fail "Volume missing: $v"
        fi
    done
else
    fail "No volumes section in docker-compose.production.yml"
fi

echo ""

# ── 9. Network definitions check ─────────────────────────────
echo "--- Network Definitions ---"

if grep -q 'networks:' "$DEPLOY_DIR/docker-compose.production.yml"; then
    for net in frontend backend monitoring; do
        if grep -q "$net:" "$DEPLOY_DIR/docker-compose.production.yml"; then
            pass "Network: $net"
        else
            fail "Network missing: $net"
        fi
    done
else
    fail "No networks section in docker-compose.production.yml"
fi

echo ""

# ── 10. Security checks ─────────────────────────────────────
echo "--- Security Checks ---"

if grep -q 'internal: true' "$DEPLOY_DIR/docker-compose.production.yml"; then
    pass "Internal networks configured"
else
    warn "No internal networks (backend/monitoring should be internal)"
fi

if grep -q 'max-size:' "$DEPLOY_DIR/docker-compose.production.yml"; then
    pass "Log rotation configured"
else
    warn "No log rotation configured"
fi

if grep -q 'resources:' "$DEPLOY_DIR/docker-compose.production.yml"; then
    pass "Resource limits configured"
else
    warn "No resource limits configured"
fi

if grep -q 'healthcheck:' "$DEPLOY_DIR/docker-compose.production.yml"; then
    pass "Healthchecks configured"
else
    warn "No healthchecks configured"
fi

echo ""

# ── Summary ──────────────────────────────────────────────────
echo "=== Summary ==="
echo -e "  ${GREEN}Passed: $PASS${NC}"
echo -e "  ${YELLOW}Warnings: $WARN${NC}"
echo -e "  ${RED}Failed: $FAIL${NC}"
echo ""

if [[ $FAIL -eq 0 ]]; then
    echo -e "${GREEN}All critical checks passed.${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed. Review above.${NC}"
    exit 1
fi
