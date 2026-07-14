# Code Review Policy

**Effective Date:** 2026-07-07
**Review Cycle:** Continuous
**SLA:** 2 hours for initial response, 24 hours for approval

---

## Overview

This document establishes the code review process for the Ferro project, aligned with HFT-grade quality standards.

---

## Review Requirements

### Minimum Reviewers

| Change Type | Required Reviewers | Approval Required |
|-------------|:------------------:|:-----------------:|
| Core algorithms | 3 | All 3 |
| Security-critical | 3 | All 3 |
| API changes | 3 | All 3 |
| Bug fixes | 2 | All 2 |
| Documentation | 1 | 1 |
| Dependencies | 2 | All 2 |
| Configuration | 2 | All 2 |

### Reviewer Qualifications

| Area | Required Expertise |
|------|-------------------|
| Core algorithms | Formal methods, complexity analysis |
| Security | OWASP, cryptography, secure coding |
| API design | Backward compatibility, ergonomics |
| Performance | Latency analysis, profiling |
| Concurrency | Thread safety, lock-free algorithms |

---

## Review Process

### 1. Pre-Review (Automated)

Before human review, all PRs must pass:

```yaml
# Required CI checks
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo test --workspace
- cargo audit
- cargo deny check
- MIRI (unsafe code only)
- ASan/TSan (selected crates)
- Property-based tests
- Fuzz tests (regression)
```

### 2. Initial Review (SLA: 2 hours)

Reviewer must:
1. Verify CI checks pass
2. Review code changes for correctness
3. Check for security vulnerabilities
4. Verify test coverage
5. Validate documentation updates
6. Leave inline comments

### 3. Detailed Review (SLA: 24 hours)

For security-critical or core algorithm changes:
1. Verify correctness against specification
2. Check for edge cases
3. Validate performance implications
4. Review concurrency safety
5. Verify formal proofs (if applicable)
6. Sign off with explicit approval

### 4. Final Approval (SLA: 2 hours)

After all review comments are addressed:
1. Verify all discussions resolved
2. Confirm CI passes
3. Merge with appropriate method (squash, rebase, merge)

---

## Review Checklist

### Code Quality

- [ ] Code follows style guide
- [ ] No unnecessary complexity
- [ ] Functions are short (<30 lines)
- [ ] Files are <400 lines
- [ ] Nesting depth ≤3
- [ ] No dead code
- [ ] No unused imports

### Security

- [ ] No hardcoded secrets
- [ ] Input validation present
- [ ] Output encoding correct
- [ ] SQL injection prevented
- [ ] Path traversal prevented
- [ ] XSS prevented
- [ ] CSRF protection present
- [ ] Authentication/authorization correct

### Performance

- [ ] No unnecessary allocations
- [ ] No unnecessary clones
- [ ] Hot paths optimized
- [ ] Memory usage acceptable
- [ ] No lock contention
- [ ] No N+1 queries

### Concurrency

- [ ] Thread safety verified
- [ ] No data races
- [ ] No deadlocks
- [ ] Proper atomic ordering
- [ ] Lock-free where possible
- [ ] Proper error handling in spawns

### Testing

- [ ] Unit tests present
- [ ] Integration tests present
- [ ] Edge cases covered
- [ ] Error cases covered
- [ ] Property-based tests (if applicable)
- [ ] Fuzz tests (if applicable)
- [ ] Mutation score >80%

### Documentation

- [ ] API documentation complete
- [ ] Inline comments where needed
- [ ] CHANGELOG updated
- [ ] ADR created (if architectural)
- [ ] Runbook updated (if operational)

---

## SLA Definitions

### Response Time

| Priority | Initial Response | Review Complete | Merge |
|----------|:----------------:|:---------------:|:-----:|
| Critical (security, production) | 30 min | 2 hours | 4 hours |
| High (features, bug fixes) | 2 hours | 24 hours | 48 hours |
| Medium (improvements) | 4 hours | 48 hours | 1 week |
| Low (docs, cleanup) | 24 hours | 1 week | 2 weeks |

### Escalation

If SLA is breached:
1. Notify team lead
2. Reassign reviewer if needed
3. Escalate to architect for critical changes

---

## Special Cases

### Hotfixes

For production incidents:
1. Create minimal fix PR
2. Tag as `hotfix`
3. Fast-track review (1 reviewer minimum)
4. Post-incident review mandatory

### Security Vulnerabilities

For security fixes:
1. Create private PR
2. Tag as `security`
3. 3-person review required
4. Coordinate disclosure

### Breaking Changes

For API breaking changes:
1. Create RFC first
2. 3-person review required
3. Migration guide required
4. Deprecation period required

---

## Tools

### Required

- `cargo fmt` - Formatting
- `cargo clippy` - Linting
- `cargo test` - Testing
- `cargo audit` - Security
- `cargo deny` - Compliance
- `cargo miri` - Memory safety
- `cargo-fuzz` - Fuzzing
- `cargo-mutants` - Mutation testing

### Recommended

- `cargo llvm-cov` - Coverage
- `cargo semver-checks` - Compatibility
- `cargo-geiger` - Unsafe audit
- `cargo-outdated` - Dependencies

---

## Metrics

### Review Metrics

Track monthly:
- Average review time
- SLA compliance rate
- Comments per PR
- Rejection rate
- Reviewer load distribution

### Quality Metrics

Track monthly:
- Defect density
- Test coverage
- Mutation score
- Fuzzing coverage
- Security vulnerabilities found

---

## Continuous Improvement

### Monthly Review

1. Analyze review metrics
2. Identify bottlenecks
3. Update process as needed
4. Share learnings

### Quarterly Audit

1. Audit review quality
2. Verify SLA compliance
3. Update reviewer qualifications
4. Update tooling

---

## References

- Google Code Review Guidelines
- Amazon Code Review Standards
- Apple Security Review Process
- HFT Industry Best Practices
- OWASP Code Review Guide
