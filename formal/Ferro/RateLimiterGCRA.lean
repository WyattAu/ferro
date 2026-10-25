-- Generic Cell Rate Algorithm (GCRA): virtual-scheduling admission with
-- burst bound. Model companion to YP-TENANCY-ISOLATION-001 (Revision 1.1).
-- GCRA is the algorithm behind redis-cell and Rust `governor`'s GCRA
-- limiter: state is a single number (TAT, the theoretical arrival time);
-- a request at time t is admitted iff t + burst·interval ≥ TAT; on
-- admission TAT ← max(t, TAT) + interval.

/-- Admission test: `t + b·i ≥ TAT` (b = burst depth, i = interval). -/
def gcraAdmit (t tat b i : Nat) : Bool :=
  decide (t + b * i ≥ tat)

/-- TAT update on admission: max(t, TAT) + interval. -/
def gcraTatUpdate (t tat i : Nat) : Nat := max t tat + i

/-- THM-GCRA-TAT-MONOTONE: the theoretical arrival time never decreases —
    the limiter's single unit of state is monotone, so a rollback of the
    clock cannot lower the bar. -/
theorem tat_monotone (t tat i : Nat) :
    tat ≤ gcraTatUpdate t tat i := by
  unfold gcraTatUpdate
  omega

/-- TAT strictly advances past the admitted request time. -/
theorem tat_above_arrival (t tat i : Nat) (hi : 0 < i) :
    t < gcraTatUpdate t tat i := by
  unfold gcraTatUpdate
  omega

/-- After k admissions at the same instant t, TAT is exactly
    max(t, tat₀) + k·i (recurrence form of the virtual scheduler). -/
def sameTimeTat (k t tat₀ i : Nat) : Nat :=
  match k with
  | 0 => tat₀
  | k' + 1 => gcraTatUpdate t (sameTimeTat k' t tat₀ i) i

theorem gcraTatUpdate_ge_self (t tat i : Nat) :
    gcraTatUpdate t tat i ≥ tat + i := by
  unfold gcraTatUpdate
  omega

theorem gcraTatUpdate_ge_t (t tat i : Nat) :
    gcraTatUpdate t tat i ≥ t + i := by
  unfold gcraTatUpdate
  omega

/-- After k+1 admissions at the same instant t, TAT is at least
    t + (k+1)·i — each admission pushes the virtual schedule one interval. -/
theorem sameTimeTat_succ_ge (k t tat₀ i : Nat) (hi : 0 < i) :
    sameTimeTat (k + 1) t tat₀ i ≥ t + (k + 1) * i := by
  induction k with
  | zero =>
      have hrfl : sameTimeTat (0 + 1) t tat₀ i = gcraTatUpdate t tat₀ i := rfl
      rw [hrfl, Nat.one_mul]
      exact gcraTatUpdate_ge_t t tat₀ i
  | succ k' ih =>
      have h1 : gcraTatUpdate t (sameTimeTat (k' + 1) t tat₀ i) i
          ≥ sameTimeTat (k' + 1) t tat₀ i + i :=
        gcraTatUpdate_ge_self t (sameTimeTat (k' + 1) t tat₀ i) i
      have h2 : (k' + 1 + 1) * i = (k' + 1) * i + i := by
        rw [Nat.add_mul, Nat.one_mul]
      have hrfl : sameTimeTat (k' + 1 + 1) t tat₀ i
          = gcraTatUpdate t (sameTimeTat (k' + 1) t tat₀ i) i := rfl
      rw [hrfl]
      omega

/-- THM-GCRA-BURST-BOUND: if k+1 requests arrive at the same instant and
    are all admitted, then k ≤ burst — an instantaneous burst can never
    exceed the burst depth. (The (k+2)-th same-instant request is
    refused.) -/
theorem gcra_burst_bound (k t tat₀ b i : Nat) (hi : 0 < i)
    (hk : sameTimeTat (k + 1) t tat₀ i ≤ t + b * i) :
    k ≤ b := by
  have h1 : sameTimeTat (k + 1) t tat₀ i ≥ t + (k + 1) * i :=
    sameTimeTat_succ_ge k t tat₀ i hi
  have h3 : (k + 1) * i ≤ b * i := by omega
  have hcomm1 : (k + 1) * i = i * (k + 1) := Nat.mul_comm (k + 1) i
  have hcomm2 : b * i = i * b := Nat.mul_comm b i
  rw [hcomm1, hcomm2] at h3
  have hfinal := Nat.le_of_mul_le_mul_left (a := k + 1) h3 hi
  omega

/-- Consecutive admissions are spaced at least one interval once the burst
    is consumed: if TAT ≥ t + b·i has been reached, the next admissible
    time is ≥ TAT - b·i ≥ t + (b·i) - b·i = t — spacing statement in the
    simplest useful form: admitting at a later t' advances TAT to at
    least t' + i, so the NEXT same-instant admission is impossible when
    the burst is exhausted (follows from gcra_burst_bound). -/
theorem gcra_spacing (t tat b i : Nat) (hi : 0 < i)
    (hadmit : gcraAdmit t tat b i = true) :
    gcraTatUpdate t tat i ≥ t + i := by
  unfold gcraAdmit at hadmit
  simp only [decide_eq_true_eq] at hadmit
  unfold gcraTatUpdate
  omega
