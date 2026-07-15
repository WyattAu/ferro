# ALC_CMC: Configuration Management

## Assurance Family Requirement

The developer shall use disciplined development procedures and configuration management to ensure consistency of the TOE and its security functions.

**EAL Level:** EAL 3+ (ALC_CMC.3)

## Evidence Artifacts

### 1. CI Pipeline (Automated Testing)

**File:** `.github/workflows/ci.yml`

| Check | Enforcement | Failure Action |
|-------|-------------|----------------|
| `cargo fmt --all -- --check` | Required | Blocks merge |
| `cargo clippy --workspace --all-targets -- -D warnings` | Required | Blocks merge |
| MSRV check (Rust 1.92) | Required | Blocks merge |
| `cargo test --locked --all` | Required | Blocks merge |
| `cargo deny check` | Required | Blocks merge |
| Trivy scan (`exit-code: 1` on CRITICAL/HIGH) | Required | Blocks merge |

**Feature Matrix (18 configurations):**
- 6 individual feature tests: `pg`, `redis`, `ldap`, `s3`, `gcs`, `azure`
- 12 feature combination tests
- PostgreSQL integration test with service container

### 2. Security Audit (cargo-deny)

**File:** `deny.toml`

| Check | Configuration |
|-------|---------------|
| Advisories | `yanked = "deny"`, documented ignores with rationale |
| Licenses | Allow-list of 14 approved licenses |
| Bans | `multiple-versions = "warn"`, `wildcards = "deny"` |
| Sources | `unknown-registry = "deny"`, `unknown-git = "deny"` |
| Targets | 5 platforms: x86_64/aarch64 Linux, x86_64/aarch64 macOS, Windows |

**Ignored Advisories (with documented rationale):**
- RUSTSEC-2025-0141: bincode unmaintained (transitive via fuse3, no server impact)
- RUSTSEC-2024-0436: paste unmaintained (transitive via leptos, web only)
- RUSTSEC-2024-0384: instant unmaintained (transitive via reed-solomon, distributed only)
- RUSTSEC-2026-0173: proc-macro-error2 (transitive via leptos, web only)

### 3. Git Configuration

| Mechanism | File | Purpose |
|-----------|------|---------|
| Branch protection | GitHub settings | Requires PR + review |
| Commit signing | GPG/SSH | Verified commits |
| Dependabot | `.github/dependabot.yml` | Automated dependency updates |
| Auto-merge | `.github/workflows/dependabot-auto-merge.yml` | Dependabot PR automation |

### 4. Pre-commit Hooks

**Path:** `.githooks/pre-commit` — **NOT YET CREATED**

Planned hooks (from CONTRIBUTING.md):
- `cargo fmt` — Code formatting
- `cargo clippy` — Linting
- `cargo test` — Unit tests

### 5. Version Control

| Aspect | Implementation |
|--------|---------------|
| Repository | Git (GitHub) |
| Branching | Main + develop branches |
| PR process | Automated checks + 1 review required |
| Lock file | `Cargo.lock` committed for reproducibility |
| Toolchain pinning | `rust-toolchain.toml` (Rust 1.95.0) |

### 6. Code Quality Gates

**File:** `.github/workflows/quality.yml`, `.github/workflows/quality-gate.yml`

| Gate | Description |
|------|-------------|
| Format | `cargo fmt --check` |
| Lint | `cargo clippy -D warnings` |
| Test | `cargo test --all` |
| Audit | `cargo deny check` |
| Security | Trivy scan |
| MSRV | Rust 1.92 compatibility |

### 7. Extended Checks

**File:** `.github/workflows/extended-checks.yml`

Additional checks beyond basic CI:
- Formal verification (Lean4)
- Sanitizers (ASAN, MSAN)
- Fuzzing regression tests
- Performance benchmarks

## Gaps

| Gap | Priority | Notes |
|-----|----------|-------|
| CM plan document | Medium | Formal configuration management plan needed |
| Change management procedures | Medium | Documented change process needed |
| Pre-commit hooks | Low | `.githooks/pre-commit` not yet created |

## Verification Instructions

```bash
# Verify CI passes
gh run list --branch main --limit 5

# Verify cargo-deny passes
cargo deny check advisories bans licenses sources

# Verify format consistency
cargo fmt --all -- --check

# Verify no clippy warnings
cargo clippy --workspace --all-targets -- -D warnings

# Verify Cargo.lock is current
cargo update --locked --dry-run

# Verify toolchain pinning
rustup show active-toolchain
# Should show 1.95.0
```

## References

- `.github/workflows/ci.yml` — Main CI pipeline
- `.github/workflows/quality-gate.yml` — Quality gate
- `.github/workflows/extended-checks.yml` — Extended checks
- `deny.toml` — cargo-deny configuration
- `rust-toolchain.toml` — Pinned toolchain
- `Cargo.lock` — Locked dependencies
