# ADR-001: Error Budget Policy

**Status:** Accepted
**Date:** 2026-07-12
**Deciders:** Wyatt (Sole developer)

## Context

Ferro is a production WebDAV/file-server with 58 crates, 2500+ tests, and real users syncing data. As a solo-developer project shipping frequent releases, there is a natural tension between shipping new features quickly and maintaining reliability. Without a structured error budget, reliability work gets perpetually deferred in favor of features, or conversely, a single incident can trigger an overcorrection that halts all feature development.

The error budget provides a data-driven mechanism to balance velocity and reliability: when the budget is healthy, ship aggressively; when it is depleted, pause feature work and focus on stability.

## Decision

### Error Budget Thresholds

Define error budgets based on a rolling 30-day window measured from structured request logs (`crates/observability`):

| SLI | Target | Budget (30-day) |
|-----|--------|-----------------|
| Availability (non-5xx responses / total responses) | 99.9% | 43 minutes of 5xx downtime |
| Data durability (no data loss on committed writes) | 99.999% | 0.43 minutes of data loss window |
| Request latency (P99 for WebDAV GET/PUT) | < 200ms local, < 500ms remote | 1% of requests may exceed |
| Sync consistency (CalDAV/CardDAV) | 99.9% eventual consistency | 0.1% stale reads |

### Consumption Policy

| Budget Remaining | Action |
|------------------|--------|
| > 50% | **Green**: Ship features, refactor, take calculated risks |
| 25-50% | **Yellow**: New features require a brief reliability justification; prioritize bug fixes |
| 10-25% | **Orange**: Feature freeze for reliability sprint; all PRs must fix a bug or improve monitoring |
| < 10% | **Red**: Full feature freeze; P0 focus on stability; daily triage of all 5xx errors |
| 0% (budget exhausted) | **Hard stop**: No releases until budget recovers to >25%; all-hands-on-deck stability |

### Freeze Triggers

A **reliability freeze** is automatically triggered when:

1. **Error budget drops below 10%** -- measured weekly on Sundays via CI script
2. **Any single incident consumes >25% of the budget** -- e.g., a 12-minute outage
3. **Data loss event** -- any confirmed data loss, regardless of budget status
4. **Two consecutive P0/P1 bugs** in a single release cycle
5. **Security incident** -- any unauthorized access or data exfiltration

### Measurement

- SLIs are computed from the existing `X-Request-ID` traced structured logs
- A `scripts/error-budget-report.sh` script computes current budget status and outputs JSON
- CI runs this weekly and posts results to a GitHub issue (or Discord webhook if configured)
- Manual override: the developer can declare a freeze at any time

### Recovery

- Budget recovers naturally as the 30-day rolling window advances past bad days
- Aggressive recovery: after a freeze, the next release must include measurable reliability improvements (e.g., added retry logic, circuit breaker tuning, improved health checks)

## Consequences

### Positive
- Provides a clear, objective trigger for when to stop shipping features and focus on reliability
- Prevents both over-engineering (never shipping) and under-engineering (shipping broken code)
- Creates a historical record of reliability posture over time
- Low overhead -- leverages existing observability infrastructure

### Negative
- Solo developer has no team to hold accountable; the budget is self-enforced and can be ignored under pressure
- 30-day rolling window may mask short but severe outages if followed by clean periods
- Initial SLI instrumentation may not cover all failure modes

### Risks
- Without enforcement, the budget becomes ceremonial documentation rather than a real constraint
- Overly conservative thresholds could slow feature delivery on a pre-1.0 project
- Measurement gaps: some failure modes (silent data corruption, eventual consistency violations) are hard to detect programmatically

## Alternatives Considered

### No Formal Budget
- **Description:** Continue ad-hoc reliability decisions based on developer judgment
- **Pros:** Zero overhead, maximum flexibility
- **Cons:** No objective criteria for prioritizing reliability; reactive rather than proactive
- **Why Rejected:** Solo developer context means the "gut feeling" approach will always favor features over stability

### SRE-Style Multi-Team Budget
- **Description:** Full Google SRE error budget with burn-rate alerts, multi-window analysis
- **Pros:** Industry best practice, proven at scale
- **Cons:** Massive overkill for a solo-developer project; requires infrastructure (alerting, dashboards) that doesn't exist
- **Why Rejected:** Complexity far exceeds the benefit for a single developer

### Per-Crate Error Budgets
- **Description:** Separate error budgets for each of the 58 crates
- **Pros:** Granular visibility, can freeze individual subsystems
- **Cons:** 58 budgets to track, impossible to maintain with one person, data doesn't exist at that granularity
- **Why Rejected:** Unsustainable overhead for solo developer

## Related ADRs
- [ADR-001](ADR-001-server-crate-decomposition.md) -- Server Crate Decomposition (decomposition affects where failures manifest)

## References
- Google SRE Book: Managing Risk with Error Budgets
- Ferro observability crate: `crates/observability/`
- Existing health check: `GET /api/health` (JSON with subsystem status)
