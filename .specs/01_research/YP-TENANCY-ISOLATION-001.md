# YP-TENANCY-ISOLATION-001: Multi-Tenant Path Isolation, Quotas & Workload Isolation

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-TENANCY-ISOLATION-001                   |
| Domain       | Multi-Tenancy / Isolation                  |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/multi-tenant/src/{tenant,isolation,quota}.rs`, `crates/rate-limiter`, `crates/wasm-host` |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| `enforce_path_prefix` **falls back to the RAW path when `canonicalize()` fails** — fail-open on broken symlinks/race windows | `crates/multi-tenant/src/isolation.rs:105-108` |
| Path scheme inconsistent: `isolation.rs` expects `{base}/tenants/{org}/{tenant}` (`:102`); `TenantPathResolver` produces `{base}/tenants/{tenant}` (`:145`) | same |
| `resolve` rejects any path *containing* `..` but does not canonicalize after join | `:137` |
| Tenant store is in-memory DashMap with O(n) slug scan — restart loses tenants unless persisted elsewhere | `tenant.rs:50-79` |
| Quotas are in-memory `AtomicU64` deltas — restart ⇒ drift; no admission hook found in the upload path | `quota.rs:20-60` |
| Plugins run via Wasm (`wasm-host`); consistent-hash ring exists but single host | `crates/wasm-host/`, `crates/consistent-hash/` |

**Security bug (not a design smell):** the canonicalize fail-open is exploitable today if a tenant-writable tree can induce a `canonicalize` failure (deep paths > PATH_MAX, racy symlinks). Fix before writing any formal tenant-containment claim — current code violates `THM-PREFIX-CONTAINMENT`'s precondition.

---

## YP-2: Executive Summary

### Problem Statement

Ferro's tenancy is logical only. For 50 users on one TrueNAS host the risks are concrete: a fail-open path check (security), a one-refactor-away path-scheme mismatch, restart-drifted quotas, and noisy neighbors (one tenant's mass re-index/OCR/backup starving interactive sync; SQLite's single writer is the contended resource).

### Decisions (three cheap mechanisms, no cluster scheduling)

1. **Correct the isolation kernel first (security):** canonicalize *then* compare; **fail closed** on canonicalize error; unify layout to `{base}/tenants/{tenant_id}` (org is a policy fact, not a path segment); after join, re-verify canonical prefix; on Linux open tenant files with **openat2 `RESOLVE_BENEATH`** in worker paths (kills symlink TOCTOU outright). Persist `TenantStore` to SQLite.
2. **Durable quotas:** per-tenant byte deltas persisted in the same transaction as uploads/deletes (natural with the YP-EVENT-DURABILITY-001 outbox); nightly reconciliation scan; enforce at **admission** — reject when `used + in_flight > quota`; overshoot bounded by max aggregate admitted-but-unfinished writes.
3. **Noisy-neighbor control without cgroups (yet):** per-tenant token buckets (existing `rate-limiter`) at the API edge; batch jobs (backup/OCR/index) on a **separate bounded worker pool** with per-tenant round-robin and **interactive-pause** (pause batch chunks when interactive p99 exceeds target — Heracles' latency-target idea applied in-process); systemd unit `CPUWeight`/`IOWeight` + `MemoryHigh` as the coarse outer fence. Revisit per-tenant cgroups only if Ferro forks per-tenant workers (it shouldn't at this scale).
4. **Plugins:** keep the Wasm boundary — no network preopen by default; WASI fs preopens **scoped to the calling tenant's root** (openat2-scoped handles, not paths — plugins doing their own path resolution inherit TOCTOU); fuel/epoch metering with per-tenant budgets.
5. **Postgres path (future):** RLS (`tenant_id = current_setting('ferro.tenant')`) as defense-in-depth behind Cedar — restrictive policies AND-combined, per Postgres §5.9, mindful of its documented sub-SELECT race caveat.

### Guarantees

- No access resolves outside the caller's tenant subtree (with openat2, without symlink-race assumptions).
- Quota overshoot bounded by admitted-in-flight writes; exact after reconciliation.
- Interactive p99 within target + pause quantum, independent of other tenants' load.

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| Fail-open / fail-closed | Behavior on error: allow (unsafe) / deny (safe). Path isolation must fail closed. |
| openat2 `RESOLVE_BENEATH` | Linux syscall flag resolving paths strictly beneath a directory fd; blocks `..` and symlink escapes at the kernel layer. |
| TOCTOU | Time-of-check-to-time-of-use race (classic symlink attack window). |
| Token bucket | Rate limiter: capacity `b`, refill `r`; smooths bursts. |
| Interactive-pause | Batch scheduler yields when interactive latency target breached (Heracles). |
| Fuel/epoch metering | Wasmtime execution budget: deterministic instruction counting with async interruption. |
| RLS | PostgreSQL row-level security; permissive policies OR, restrictive policies AND. |

---

## YP-4: Theoretical Foundation

### THM-PREFIX-CONTAINMENT

If every file-system access by tenant `t` is resolved via resolver `R` that (i) canonicalizes the final path, (ii) **fails closed** on canonicalization error, and (iii) requires `canonical ∈ prefix(t)`, then no access resolves outside `t`'s subtree — given no symlink race. **With openat2 `RESOLVE_BENEATH`, the lemma holds without that assumption.** ⚠ Current code violates (ii) (`isolation.rs:105-108`).

### THM-QUOTA-OVERSHOOT-BOUND

With transactional delta accounting and admission test `used + in_flight ≤ quota`, overshoot per tenant is bounded by the maximum aggregate size of admitted-but-unfinished writes `ε ≤ B_max` (configurable, e.g. 2× max object size), and converges exactly after reconciliation.

### THM-NOISY-NEIGHBOR-BOUND

With per-tenant token buckets (rate `r_i`, burst `b_i`) at the edge and a separate batch pool with per-tenant round-robin and interactive-pause (target `L`), interactive p99 for every tenant stays ≤ `L + c` (c = pause quantum), independent of any other tenant's offered load — the single-host analog of PerfIso's colocation claim.

### THM-PLUGIN-CONTAINMENT

A Wasm plugin instance with linear-memory isolation, no ambient WASI authority, preopens restricted to the tenant root (openat2-scoped), and fuel budget `F` cannot (a) observe memory outside its instance, (b) open paths outside the preopen, or (c) execute more than `F` units per invocation (PLDI'17 model + capability discipline).

### THM-TENANT-POLICY-SOUNDNESS

If all object reads pass through decision function `D` built on validated Cedar policies (see YP-AUTHZ-POLICY-001) AND membership enforcement from `TenantGuard`, then `∀ p, o`: `D(p, o) = allow → o.tenant ∈ tenants(p)` — the policy layer cannot express, and the guard layer cannot permit, a cross-tenant read.

---

## YP-5: Algorithm Specification

### ALG-RESOLVE-001: Fail-Closed Tenant Resolution

```
fn resolve(base, tenant, rel):
    p := join(base, "tenants", tenant_id, rel)     # unified layout, no org segment
    if rel contains "..": reject                    # fast path; kernel check is authoritative
    canon := canonicalize(p) or return Err(FailClosed)   # NEVER fall back to raw
    require canon starts_with canonicalize(tenant_root(tenant))
    return openat2(root_fd, rel, RESOLVE_BENEATH)        # workers; kernel-enforced
```

### ALG-QUOTA-001: Durable Admission Control

```
upload: if used(t) + size > quota(t): reject 413
        delta := +size in same tx as CAS put
delete: delta := -size in same tx
nightly: reconcile used(t) against storage scan; log drift
in-flight: admitted-but-unfinished tracked; counted in admission test
```

### ALG-BATCHPOOL-001: Interactive-Priority Batch Scheduling

```
pools: interactive (unbounded-ish, per-user buckets) | batch (N workers, round-robin tenants)
batch chunk loop: if interactive_p99 > L: yield(sleep c)     # Heracles-style pause
systemd: CPUWeight=80 IOWeight=80 MemoryHigh=<70%>            # coarse outer fence
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_durability.toml` — sections `tenancy_*`.

Critical vectors: symlink-race attempt → denied (fail-closed); `..` traversal (raw and canonical forms); org-less layout migration matrix; quota admission at boundary; overshoot ≤ B_max under concurrent admits; restart → quota persisted (no drift); batch starvation test (interactive p99 ≤ L + c under synthetic batch flood); plugin preopen escape attempt → denied; plugin fuel exhaustion.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_durability.toml`.

Summary: path isolation MUST fail closed (`DC-TENANT-001`); one canonical path layout, org is not a path segment (`DC-TENANT-002`); workers MUST use `RESOLVE_BENEATH` on Linux (`DC-TENANT-003`); quota counters MUST be durable (`DC-TENANT-004`); batch workloads MUST run on a separate pool with interactive-pause (`DC-TENANT-005`); plugin preopens MUST be tenant-rooted and fuel-metered (`DC-TENANT-006`).

---

## YP-8: Bibliography

1. Delimitrou, C., Kozyrakis, C. "QoS-Aware Scheduling in Heterogeneous Datacenters with Paragon." *ACM TOCS*, Dec 2013, DOI `10.1145/2556583` (original ASPLOS 2013 — venue corrected from "SOSP 2012"). Online interference classification + QoS targets. [VERIFIED]
2. Lo, D., Cheng, L., Govindaraju, R., Ranganathan, P., Kozyrakis, C. "Heracles: Improving Resource Efficiency at Scale." *ISCA 2015*; extended *ACM TOCS* 34(2), 2016, DOI `10.1145/2882783`. Latency-target tracking + resource controls. [VERIFIED]
3. Xiao, W., Ren, S., Li, Y., Zhang, Y., Hou, P., Li, Z., Feng, Y., Lin, W., Jia, Y. "AntMan: Dynamic Scaling on GPU Clusters for Deep Learning." *USENIX OSDI '20*, pp. 533–548. **Negative result:** an "AntMan, ASPLOS 2020" about container/latency isolation does not exist. [VERIFIED]
4. Iorgulescu, C., Azimi, R., Kwon, Y., Elnikety, S., Syamala, M., Narasayya, V. R., Herodotou, H., Tomita, P., et al. "PerfIso: Performance Isolation for Commercial Latency-Sensitive Services." *USENIX ATC '18*, pp. 519–532. ~3 years in Bing across 90,000+ servers. [VERIFIED]
5. **Negative result:** "Silo: Predictable Latency in the Memory Hierarchy" (SOSP 2011) — no trace under that title/venue; do not cite until verified. [VERIFIED ABSENCE]
6. PostgreSQL Global Development Group. "Row Security Policies" (§5.9, PG 18 docs): per-role `USING`/`WITH CHECK`; permissive OR / restrictive AND; default-deny; `BYPASSRLS`; documented sub-SELECT race caveat. [VERIFIED]
7. Haas, A., Rossberg, A., Schuff, D. L., Titzer, B. L., Holman, M., Gohman, D., Wagner, L., Zakai, A., Bastien, J. F. "Bringing the Web up to Speed with WebAssembly." *PLDI 2017*, DOI `10.1145/3062341.3062363` (**not** ASPLOS). Linear memory + validation + no ambient authority. [VERIFIED]
8. Cutler, C., et al. "Cedar: a new language for expressive, fast, safe, and analyzable authorization." *OOPSLA 2024* (venue corrected from "IEEE S&P"). [VERIFIED]
9. Linux cgroups v2, namespaces(7), seccomp(2), Landlock, systemd resource-control(5) — stable primary sources; verify exact URLs before spec freeze. [UNCERTAIN]
10. Wasmtime security/sandbox docs; Dropbox Business team namespaces; Google Workspace org isolation; Gitea/Forgejo org model; Seafile per-user libraries — vendor docs, verify individually. [UNCERTAIN]
