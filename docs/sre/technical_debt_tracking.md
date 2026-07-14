# Technical Debt Automation

**Document:** Technical Debt Tracking and Management  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Overview

Automated technical debt tracking monitors TODO/FIXME comments, code complexity, dependency staleness, and other debt indicators. Debt is categorized, prioritized, and tracked over time.

---

## Debt Categories

| Category | Indicators | Priority Weight |
|----------|-----------|-----------------|
| Security | TODO/FIXME with "security", "vuln", "auth" | 4x |
| Correctness | TODO/FIXME with "bug", "fix", "wrong", "hack" | 3x |
| Performance | TODO/FIXME with "slow", "optimize", "perf" | 2x |
| Maintainability | TODO/FIXME with "refactor", "cleanup", "remove" | 1x |
| Documentation | TODO/FIXME with "doc", "comment", "explain" | 0.5x |

---

## Detection Methods

### 1. TODO/FIXME Scanning

```bash
# Scan for TODO/FIXME/HACK/XXX comments
rg -n "TODO|FIXME|HACK|XXX|SAFETY" crates/ --type rust | \
  grep -v "test" | \
  grep -v "#\[cfg(test)\]" | \
  sort > debt-todo.txt

# Count by category
rg -c "TODO.*security|FIXME.*security" crates/ --type rust
rg -c "TODO.*perf|FIXME.*perf|TODO.*slow" crates/ --type rust
```

### 2. Complexity Metrics

```bash
# Cyclomatic complexity (requires cargo-geiger or similar)
# Flag functions with complexity > 20
```

### 3. Dead Code Detection

```bash
# Unused code
cargo clippy --workspace -- -W dead_code

# Unused dependencies
cargo machete
```

### 4. Dependency Staleness

```bash
# Check for outdated dependencies
cargo outdated --workspace

# Check for unmaintained dependencies
cargo deny check advisories
```

---

## Debt Inventory

| ID | Category | Description | File | Priority | Added | Status |
|----|----------|-------------|------|----------|-------|--------|
| TD-001 | Security | Debug impl leaks secrets | crates/auth/src/users.rs | P0 | 2026-07-12 | Open |
| TD-002 | Performance | Lock tokens logged at debug | crates/server-webdav-core/src/handlers/lock.rs | P2 | 2026-07-12 | Open |
| TD-003 | Maintainability | Duplicate webhook config types | crates/webhook, crates/server-api-core, crates/server-automation | P2 | 2026-07-12 | Open |
| TD-004 | Security | Missing zeroize on secret types | Multiple crates | P0 | 2026-07-12 | Open |
| TD-005 | Maintainability | SMB password in mount_options string | crates/mount-nfs/src/smb.rs | P1 | 2026-07-12 | Open |

---

## Debt Dashboard

### Metrics Tracked

| Metric | Current | Target | Trend |
|--------|---------|--------|-------|
| TODO count | ~50 | < 20 | Stable |
| FIXME count | ~15 | < 5 | Decreasing |
| Complexity > 20 | ~8 functions | 0 | Stable |
| Unused dependencies | 0 | 0 | Clean |
| Outdated dependencies | ~5 | < 2 | Stable |
| Dead code warnings | ~3 | 0 | Decreasing |

### Weekly Report

```bash
#!/bin/bash
# scripts/technical_debt_report.sh
set -euo pipefail

echo "# Technical Debt Report - $(date +%Y-%m-%d)"
echo ""
echo "## TODO/FIXME Count"
rg -c "TODO|FIXME" crates/ --type rust | wc -l
echo ""
echo "## By Category"
echo "Security: $(rg -c 'TODO.*security|FIXME.*security' crates/ --type rust 2>/dev/null | wc -l)"
echo "Performance: $(rg -c 'TODO.*perf|FIXME.*perf|TODO.*slow' crates/ --type rust 2>/dev/null | wc -l)"
echo "Maintainability: $(rg -c 'TODO.*refactor|FIXME.*refactor' crates/ --type rust 2>/dev/null | wc -l)"
```

---

## Integration

### CI Gate (Advisory)

```yaml
# In CI - informational, not blocking
- name: Debt report
  run: |
    ./scripts/technical_debt_report.sh > debt-report.md
    cat debt-report.md
```

### Sprint Integration

Debt items are reviewed weekly:
- P0 items addressed immediately
- P1 items addressed in current sprint
- P2 items addressed in next sprint
- P3 items backlog

---

## References

- [Technical Debt Quadrant](https://martinfowler.com/bliki/TechnicalDebtQuadrant.html)
- [COSMIC debt model](https://www.iso.org/standard/35563.html)
