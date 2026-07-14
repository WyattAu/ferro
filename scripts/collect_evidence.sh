#!/bin/bash
set -euo pipefail

EVIDENCE_DIR="docs/compliance/soc2/evidence/collected"
mkdir -p "$EVIDENCE_DIR"

echo "Collecting SOC 2 evidence..."

# Access Control Evidence
echo "Collecting access control evidence..."
cat > "$EVIDENCE_DIR/access_control_$(date +%Y%m%d).md" << EOF
# Access Control Evidence - $(date +%Y-%m-%d)

## MFA Status
$(grep -r "mfa" --include="*.toml" --include="*.yaml" . 2>/dev/null | head -20 || echo "No MFA config found")

## RBAC Configuration
$(grep -r "role" --include="*.toml" --include="*.yaml" . 2>/dev/null | head -20 || echo "No RBAC config found")

## User Management
$(grep -r "user" --include="*.toml" --include="*.yaml" . 2>/dev/null | head -20 || echo "No user config found")
EOF

# Change Management Evidence
echo "Collecting change management evidence..."
cat > "$EVIDENCE_DIR/change_management_$(date +%Y%m%d).md" << EOF
# Change Management Evidence - $(date +%Y-%m-%d)

## Git History (Last 30 days)
$(git log --oneline --since="30 days ago" | head -50)

## Pull Requests
$(gh pr list --state merged --limit 20 2>/dev/null || echo "GitHub CLI not available")

## CI/CD Pipeline
$(cat .github/workflows/*.yml 2>/dev/null | head -100 || echo "No CI config found")
EOF

# Security Evidence
echo "Collecting security evidence..."
cat > "$EVIDENCE_DIR/security_$(date +%Y%m%d).md" << EOF
# Security Evidence - $(date +%Y-%m-%d)

## Vulnerability Scan
$(cargo audit 2>/dev/null | head -50 || echo "cargo-audit not available")

## Dependency Check
$(cargo deny check 2>/dev/null | head -50 || echo "cargo-deny not available")

## Secret Scanning
$(grep -r "password\|secret\|token\|key" --include="*.rs" --include="*.toml" . 2>/dev/null | grep -v "test\|example\|doc" | head -20 || echo "No secrets found")
EOF

# Monitoring Evidence
echo "Collecting monitoring evidence..."
cat > "$EVIDENCE_DIR/monitoring_$(date +%Y%m%d).md" << EOF
# Monitoring Evidence - $(date +%Y-%m-%d)

## Test Coverage
$(cargo llvm-cov report 2>/dev/null | head -30 || echo "Coverage not available")

## Lint Results
$(cargo clippy -- -D warnings 2>/dev/null | tail -20 || echo "Clippy not available")

## Performance Metrics
$(cat benchmarks/results/*.json 2>/dev/null | head -50 || echo "No benchmark results found")
EOF

echo "Evidence collected in $EVIDENCE_DIR"
