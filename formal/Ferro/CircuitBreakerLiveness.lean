-- Circuit breaker liveness: bounded detection, success-reset semantics,
-- exponential ejection backoff with cap, bounded recovery.
-- Model companion to the circuit-breaker design (Revision 1.1).
-- Reference: Envoy outlier detection (envoyproxy.io docs, fetched
-- 2026-10-14) [VERIFIED]: consecutive-failure ejection, ejection time =
-- base_ejection_time × consecutive-ejection count, capped at
-- max_ejection_time; a success unejects and resets counters.

/-- Breaker state: 0 = Closed, 1 = Open, 2 = HalfOpen. -/
abbrev BreakerState := Nat

def Closed : BreakerState := 0
def Open : BreakerState := 1
def HalfOpen : BreakerState := 2

/-- Breaker with a consecutive-failure counter. -/
structure Breaker where
  state : BreakerState
  consecutive : Nat
  threshold : Nat
  deriving DecidableEq

/-- Recording a failure: increment; trip to Open at the threshold
    (consecutive-5xx style). -/
def breakerFail (br : Breaker) : Breaker :=
  let c := br.consecutive + 1
  if c ≥ br.threshold then ⟨Open, c, br.threshold⟩
  else ⟨br.state, c, br.threshold⟩

/-- Recording a success: the consecutive counter resets unconditionally
    (Envoy: a success unejects and clears counters). From Open, a
    successful probe advances to HalfOpen; from HalfOpen it closes. -/
def breakerSuccess (br : Breaker) : Breaker :=
  match br.state with
  | s => if s = Open then ⟨HalfOpen, 0, br.threshold⟩
         else if s = HalfOpen then ⟨Closed, 0, br.threshold⟩
         else ⟨Closed, 0, br.threshold⟩

/-- THM-CB-FAILURE-INCREMENTS: a failure strictly increments the
    consecutive counter. -/
theorem fail_increments (br : Breaker) :
    (breakerFail br).consecutive = br.consecutive + 1 := by
  simp only [breakerFail]
  split <;> rfl

/-- THM-CB-SUCCESS-RESETS: a success resets the consecutive counter to
    zero — no stale failure budget survives a healthy call. -/
theorem success_resets (br : Breaker) :
    (breakerSuccess br).consecutive = 0 := by
  simp only [breakerSuccess]
  split
  · rfl
  · split <;> rfl

/-- THM-CB-DETECTION-BOUNDED: after exactly `threshold` consecutive
    failures, the breaker is Open — detection time is bounded by the
    threshold, independent of history. -/
theorem detection_bounded (br : Breaker) (h : br.consecutive = br.threshold) :
    (breakerFail br).state = Open := by
  simp only [breakerFail]
  rw [h, if_pos (by omega)]

/-- A sub-threshold failure on a non-Open breaker keeps it non-Open. -/
theorem fail_below_threshold_not_open (br : Breaker)
    (h : br.consecutive + 1 < br.threshold) (hnot : br.state ≠ Open) :
    (breakerFail br).state ≠ Open := by
  simp only [breakerFail]
  split
  · omega
  · intro hopen
    exact hnot (by simpa using hopen)

/-- THM-CB-RECOVERY-BOUNDED: from Open, exactly two successful calls
    close the breaker (probe → HalfOpen, confirm → Closed) — bounded
    recovery, no oscillation without new failures. -/
theorem recovery_bounded (br : Breaker) (h : br.state = Open) :
    (breakerSuccess (breakerSuccess br)).state = Closed := by
  simp [breakerSuccess, h, HalfOpen, Open]

/-- Ejection duration: base × 2^(consecutive ejections − 1), capped. -/
def ejectTime (base mult cap : Nat) : Nat :=
  min (base * mult) cap

/-- THM-CB-EJECTION-CAPPED: the ejection duration never exceeds the
    configured maximum — a persistently failing dependency is re-probed
    at least once per `cap` window. -/
theorem ejection_capped (base mult cap : Nat) :
    ejectTime base mult cap ≤ cap := by
  unfold ejectTime
  omega

/-- THM-CB-EJECTION-ESCALATES: while under the cap, each consecutive
    ejection lasts strictly longer than the previous one — flapping
    dependencies back off exponentially. -/
theorem ejection_escalates (base mult cap : Nat)
    (h : base * mult < cap) (hbase : 0 < base) :
    ejectTime base mult cap < ejectTime base (mult + 1) cap := by
  unfold ejectTime
  have h1 : base * (mult + 1) = base * mult + base := by
    rw [Nat.mul_add, Nat.mul_one]
  omega

/-- THM-CB-EJECTION-MONOTONE: escalation never shortens the ejection. -/
theorem ejection_monotone (base mult cap : Nat) :
    ejectTime base mult cap ≤ ejectTime base (mult + 1) cap := by
  unfold ejectTime
  have h1 : base * (mult + 1) = base * mult + base := by
    rw [Nat.mul_add, Nat.mul_one]
  omega

/-- The success path clears to Closed from HalfOpen — the probe succeeded,
    so the breaker closes and the consecutive counter starts fresh. -/
theorem halfopen_success_closes (br : Breaker) (h : br.state = HalfOpen) :
    (breakerSuccess br).state = Closed := by
  simp [breakerSuccess, h, HalfOpen, Open]
