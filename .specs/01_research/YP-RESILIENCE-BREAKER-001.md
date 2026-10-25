# YP-RESILIENCE-BREAKER-001: Adaptive Circuit Breaking, Ejection Backoff & Recovery Bounds

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-RESILIENCE-BREAKER-001                  |
| Domain       | Resilience / Failure Detection             |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration R2-8) |
| Date         | 2026-10-14                                 |
| Crate Ref    | `formal/Ferro/CircuitBreaker*.lean`, wasm-worker + outbound HTTP paths |
| Supersedes   | (none; first YP for the breaker — previously only `formal/` models existed) |

---

## YP-1: Ground Truth (Current Implementation)

| Fact | Location |
|---|---|
| 3-state breaker (Closed → Open → HalfOpen), fixed failure-count threshold | `formal/Ferro/CircuitBreaker.lean` |
| Time-based refill modelled but no ejection backoff | `formal/Ferro/CircuitBreakerRefined.lean` |
| No consecutive-failure reset semantics, no capped exponential backoff, no recovery bound | repo-wide |

**Consequences:** a flapping dependency causes 1:1 ejection/re-probe churn (no escalation); a breaker stuck Open has no bounded re-probe guarantee; success does not provably clear the failure budget.

---

## YP-2: Executive Summary

### Problem Statement

Ferro's wasm workers and outbound federation/webhook calls need a breaker that (a) detects persistent failure within a bounded number of calls, (b) backs off exponentially so a flapping dependency is probed less and less often, (c) recovers in bounded time, and (d) never fully disables a dependency class (fleet cap).

### Design Decision (the headline)

> **Adopt Envoy-style outlier semantics on the existing 3-state machine: consecutive-failure tripping, success-resets-counter, ejection duration = base × consecutive-ejections capped at a maximum, and a fleet-wide max-ejection-percentage.**

### Guarantees

- **Bounded detection:** after exactly `threshold` consecutive failures the breaker is Open.
- **Bounded recovery:** from Open, two successful calls close the breaker (probe → HalfOpen → Closed).
- **Capped escalation:** ejection duration ≤ `max_ejection_time` — re-probe at least once per cap window.
- **Escalation under the cap:** consecutive ejections strictly increase in duration.

---

## YP-3: Nomenclature

| Term | Definition |
|------|------------|
| Consecutive counter | Failures since last success; a success clears it to 0. |
| Threshold | Consecutive failures that trip Open (consecutive-5xx analogue). |
| Ejection duration | `min(base × consecutive_ejections, cap)` — Envoy `base_ejection_time` × count, capped by `max_ejection_time`. |
| HalfOpen probe | The single trial call allowed while Open. |

---

## YP-4: Theoretical Foundation

### THM-CB-FAILURE-INCREMENTS
A failure strictly increments the consecutive counter. [Proved — Lean `fail_increments`]

### THM-CB-SUCCESS-RESETS
A success resets the consecutive counter to zero. [Proved — Lean `success_resets`]

### THM-CB-DETECTION-BOUNDED
After exactly `threshold` consecutive failures the breaker is Open — detection latency ≤ threshold calls regardless of history. [Proved — Lean `detection_bounded`]

### THM-CB-RECOVERY-BOUNDED
From Open, two successes close the breaker. Recovery is bounded; no oscillation without new failures. [Proved — Lean `recovery_bounded`, `halfopen_success_closes`]

### THM-CB-EJECTION-CAPPED
`ejectTime = min(base × mult, cap) ≤ cap` — a dead dependency is re-probed at least once per cap window; liveness of the probe path. [Proved — Lean `ejection_capped`]

### THM-CB-EJECTION-ESCALATES / THM-CB-EJECTION-MONOTONE
While `base × mult < cap`, consecutive ejections strictly lengthen; never shorten. [Proved — Lean `ejection_escalates`, `ejection_monotone`]

---

## YP-5: Algorithm Specification

### ALG-BREAKER-ADAPTIVE-001

```
on failure:  consecutive += 1
             if consecutive ≥ threshold: state ← Open, ejectFor(min(base × ejectCount, cap))
on success:  consecutive ← 0
             Open      → HalfOpen (probe allowed)
             HalfOpen  → Closed
             Closed    → Closed
fleet rule:  never eject more than max_ejection_percent of hosts in a class
             (Envoy max_ejection_percent analogue; panic threshold bypasses)
```

### Parameters

| Parameter | Value | Rationale |
|---|---|---|
| threshold | 5 consecutive | Envoy consecutive_5xx default is 5 |
| base_ejection | 30 s | Envoy default |
| cap | 300 s | Envoy max_ejection_time default |
| max_ejection_percent | 10 % | Envoy default; keeps the class alive |

---

## YP-6: Test Vector Specification

Trip at threshold (5th consecutive failure); success resets budget mid-streak; ejection durations 30/60/120/240/300/300…; recovery probe → confirm; fleet cap respected.

## YP-7: Domain Constraints

- DC-CB-001: a breaker MUST NOT enter Open without `threshold` consecutive failures (no heuristic tripping on the formal path).
- DC-CB-002: ejection duration MUST be capped; unbounded backoff is forbidden.
- DC-CB-003: a success MUST clear the consecutive counter.
- DC-CB-004: the fleet MUST keep ≥ (100 − max_ejection_percent)% of a class ejectable-capacity.

## YP-8: Bibliography

1. Envoy Proxy. "Outlier detection," *Architecture Overview* — envoyproxy.io docs (fetched 2026-10-14): consecutive-5xx, ejection-time escalation with cap, success uneject, max_ejection_percent. [VERIFIED]
2. Netflix. "Concurrency Limits & adaptive limiting" (reference for future adaptive thresholds). [UNCERTAIN — not fetched]
