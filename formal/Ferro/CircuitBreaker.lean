-- Formal verification of circuit breaker state machine

inductive CircuitState where
  | closed
  | open
  | halfOpen
  deriving Repr, BEq

structure CircuitBreaker where
  state : CircuitState
  failureCount : Nat
  successCount : Nat
  threshold : Nat
  deriving Repr

/-- Initial state is closed with zero counts -/
def CircuitBreaker.init (threshold : Nat) : CircuitBreaker where
  state := .closed
  failureCount := 0
  successCount := 0
  threshold := threshold

/-- Record a failure -/
def CircuitBreaker.recordFailure (cb : CircuitBreaker) : CircuitBreaker :=
  if cb.state == .open then cb
  else if cb.failureCount + 1 ≥ cb.threshold then
    { cb with state := .open, failureCount := cb.failureCount + 1 }
  else
    { cb with failureCount := cb.failureCount + 1 }

/-- Record a success -/
def CircuitBreaker.recordSuccess (cb : CircuitBreaker) : CircuitBreaker :=
  match cb.state with
  | .open => cb
  | .closed => { cb with successCount := cb.successCount + 1, failureCount := 0 }
  | .halfOpen => { cb with state := .closed, successCount := 0, failureCount := 0 }

/-- After threshold failures, state is open -/
theorem recordFailure_opens_circuit
  (cb : CircuitBreaker) (h : cb.state ≠ .open) (h2 : cb.failureCount + 1 ≥ cb.threshold) :
  (cb.recordFailure).state = .open := by
  simp [CircuitBreaker.recordFailure]
  split
  · -- case: state == .open = true
    have : cb.state = .open := by simp_all [BEq.beq, Bool.not_eq_true]
    contradiction
  · -- case: threshold check passed
    rfl

/-- Circuit starts closed -/
theorem init_is_closed (threshold : Nat) : (CircuitBreaker.init threshold).state = .closed := by
  simp [CircuitBreaker.init]
