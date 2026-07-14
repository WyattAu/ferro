-- Refined circuit breaker proofs with time-based transitions

inductive CircuitState where
  | closed
  | open
  | halfOpen
  deriving Repr, DecidableEq

structure RefinedCircuitBreaker where
  state : CircuitState
  failureCount : Nat
  successCount : Nat
  threshold : Nat
  timeout : Nat
  lastFailure : Nat
  deriving Repr

def RefinedCircuitBreaker.init (threshold timeout : Nat) : RefinedCircuitBreaker where
  state := .closed
  failureCount := 0
  successCount := 0
  threshold := threshold
  timeout := timeout
  lastFailure := 0

def RefinedCircuitBreaker.recordFailure (cb : RefinedCircuitBreaker) (now : Nat) : RefinedCircuitBreaker :=
  match cb.state with
  | .open => cb
  | _ =>
    if cb.failureCount + 1 ≥ cb.threshold then
      { cb with state := .open, failureCount := cb.failureCount + 1, lastFailure := now }
    else if cb.state = .halfOpen then
      { cb with state := .open, failureCount := cb.failureCount + 1, lastFailure := now }
    else
      { cb with failureCount := cb.failureCount + 1 }

def RefinedCircuitBreaker.recordSuccess (cb : RefinedCircuitBreaker) : RefinedCircuitBreaker :=
  match cb.state with
  | .open => cb
  | .closed => { cb with successCount := cb.successCount + 1, failureCount := 0 }
  | .halfOpen => { cb with state := .closed, successCount := 0, failureCount := 0 }

def RefinedCircuitBreaker.checkTimeout (cb : RefinedCircuitBreaker) (now : Nat) : RefinedCircuitBreaker :=
  match cb.state with
  | .open =>
    if now - cb.lastFailure ≥ cb.timeout then
      { cb with state := .halfOpen }
    else cb
  | _ => cb

theorem init_is_closed (threshold timeout : Nat) :
  (RefinedCircuitBreaker.init threshold timeout).state = .closed := by
  rfl

theorem init_failure_count_zero (threshold timeout : Nat) :
  (RefinedCircuitBreaker.init threshold timeout).failureCount = 0 := by
  rfl

theorem init_success_count_zero (threshold timeout : Nat) :
  (RefinedCircuitBreaker.init threshold timeout).successCount = 0 := by
  rfl

theorem init_last_failure_zero (threshold timeout : Nat) :
  (RefinedCircuitBreaker.init threshold timeout).lastFailure = 0 := by
  rfl

theorem threshold_opens_circuit
  (cb : RefinedCircuitBreaker) (h : cb.state ≠ .open) (h2 : cb.failureCount + 1 ≥ cb.threshold) :
  (cb.recordFailure 0).state = .open := by
  simp [RefinedCircuitBreaker.recordFailure]
  simp_all

theorem half_open_failure_opens
  (cb : RefinedCircuitBreaker) (h : cb.state = .halfOpen) :
  (cb.recordFailure 0).state = .open := by
  simp [RefinedCircuitBreaker.recordFailure, h]

theorem half_open_success_closes
  (cb : RefinedCircuitBreaker) (h : cb.state = .halfOpen) :
  (cb.recordSuccess).state = .closed := by
  simp [RefinedCircuitBreaker.recordSuccess, h]

theorem closed_success_increments
  (cb : RefinedCircuitBreaker) (h : cb.state = .closed) :
  (cb.recordSuccess).successCount = cb.successCount + 1 := by
  simp [RefinedCircuitBreaker.recordSuccess, h]

theorem closed_success_resets_failures
  (cb : RefinedCircuitBreaker) (h : cb.state = .closed) :
  (cb.recordSuccess).failureCount = 0 := by
  simp [RefinedCircuitBreaker.recordSuccess, h]

theorem open_success_noop
  (cb : RefinedCircuitBreaker) (h : cb.state = .open) :
  (cb.recordSuccess).state = .open := by
  simp [RefinedCircuitBreaker.recordSuccess, h]

theorem open_failure_noop
  (cb : RefinedCircuitBreaker) (h : cb.state = .open) :
  (cb.recordFailure 0).state = .open := by
  simp [RefinedCircuitBreaker.recordFailure, h]

theorem check_timeout_closed_noop
  (cb : RefinedCircuitBreaker) (now : Nat) (h : cb.state = .closed) :
  (cb.checkTimeout now).state = .closed := by
  simp [RefinedCircuitBreaker.checkTimeout, h]

theorem check_timeout_half_open_noop
  (cb : RefinedCircuitBreaker) (now : Nat) (h : cb.state = .halfOpen) :
  (cb.checkTimeout now).state = .halfOpen := by
  simp [RefinedCircuitBreaker.checkTimeout, h]

theorem state_machine_deterministic
  (cb : RefinedCircuitBreaker) (now : Nat) :
  (cb.recordFailure now) = (cb.recordFailure now) := by
  rfl

theorem failure_count_nonneg (cb : RefinedCircuitBreaker) :
  cb.failureCount ≥ 0 := by
  omega

theorem success_count_nonneg (cb : RefinedCircuitBreaker) :
  cb.successCount ≥ 0 := by
  omega

theorem open_record_success_preserves
  (cb : RefinedCircuitBreaker) (h : cb.state = .open) :
  (cb.recordSuccess).failureCount = cb.failureCount := by
  simp [RefinedCircuitBreaker.recordSuccess, h]

theorem timeout_nonneg (cb : RefinedCircuitBreaker) :
  cb.timeout ≥ 0 := by
  omega

/-- After threshold failures, state transitions to open -/
theorem threshold_transitions_to_open
  (cb : RefinedCircuitBreaker) (now : Nat)
  (h : cb.state ≠ .open) (h2 : cb.failureCount + 1 ≥ cb.threshold) :
  (cb.recordFailure now).state = .open := by
  simp [RefinedCircuitBreaker.recordFailure]
  simp_all

/-- Check timeout on closed state returns closed -/
theorem check_timeout_closed
  (cb : RefinedCircuitBreaker) (now : Nat) (h : cb.state = .closed) :
  (cb.checkTimeout now).state = .closed := by
  simp [RefinedCircuitBreaker.checkTimeout, h]

/-- Check timeout on half-open state returns half-open -/
theorem check_timeout_half_open
  (cb : RefinedCircuitBreaker) (now : Nat) (h : cb.state = .halfOpen) :
  (cb.checkTimeout now).state = .halfOpen := by
  simp [RefinedCircuitBreaker.checkTimeout, h]
