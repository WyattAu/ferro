-- Rate limiter property proofs

/-- Token bucket state with time-based refill -/
structure TokenBucket where
  tokens : Nat
  maxTokens : Nat
  refillRate : Nat
  lastRefill : Nat
  deriving Repr

/-- Token bucket invariant: tokens ≤ maxTokens -/
def TokenBucket.invariant (tb : TokenBucket) : Prop :=
  tb.tokens ≤ tb.maxTokens

/-- Token bucket initialization -/
def TokenBucket.init (maxTokens refillRate : Nat) : TokenBucket where
  tokens := maxTokens
  maxTokens := maxTokens
  refillRate := refillRate
  lastRefill := 0

/-- Refill tokens based on elapsed time -/
def TokenBucket.refill (tb : TokenBucket) (now : Nat) : TokenBucket :=
  { tb with
    tokens := min (tb.tokens + (now - tb.lastRefill) * tb.refillRate) tb.maxTokens,
    lastRefill := now }

/-- Initial tokens equal maxTokens -/
theorem init_tokens_equal_max (maxTokens refillRate : Nat) :
  (TokenBucket.init maxTokens refillRate).tokens = maxTokens := by
  rfl

/-- Initial lastRefill is zero -/
theorem init_last_refill_zero (maxTokens refillRate : Nat) :
  (TokenBucket.init maxTokens refillRate).lastRefill = 0 := by
  rfl

/-- Refill never exceeds maxTokens -/
theorem refill_never_exceeds_max (tb : TokenBucket) (now : Nat) :
  (TokenBucket.refill tb now).tokens ≤ tb.maxTokens := by
  simp [TokenBucket.refill]
  exact Nat.min_le_right _ _

/-- No elapsed time means no token change when invariant holds -/
theorem no_elapsed_no_change (tb : TokenBucket) (now : Nat)
    (h : now ≤ tb.lastRefill) (h_inv : tb.invariant) :
  (TokenBucket.refill tb now).tokens = tb.tokens := by
  simp [TokenBucket.refill, TokenBucket.invariant] at *
  have := Nat.sub_eq_zero_of_le h
  rw [this, Nat.zero_mul, Nat.add_zero]
  exact Nat.min_eq_left h_inv

/-- Refill adds at most elapsed * refillRate tokens -/
theorem refill_adds_bounded (tb : TokenBucket) (now : Nat) :
  (TokenBucket.refill tb now).tokens ≤ tb.tokens + (now - tb.lastRefill) * tb.refillRate := by
  simp [TokenBucket.refill]
  exact Nat.min_le_left _ _

/-- Invariant holds after init -/
theorem init_invariant (maxTokens refillRate : Nat) :
  (TokenBucket.init maxTokens refillRate).invariant := by
  simp [TokenBucket.init, TokenBucket.invariant]

/-- Invariant preserved by refill -/
theorem refill_preserves_invariant (tb : TokenBucket) (now : Nat)
    (_h : tb.invariant) :
  (TokenBucket.refill tb now).invariant := by
  simp [TokenBucket.invariant]
  apply refill_never_exceeds_max

/-- Rate limit is enforced: cannot exceed maxTokens after refill -/
theorem rate_limit_enforced (tb : TokenBucket) (now : Nat) :
  (TokenBucket.refill tb now).tokens ≤ tb.maxTokens := by
  apply refill_never_exceeds_max

/-- Refill updates lastRefill timestamp -/
theorem refill_updates_timestamp (tb : TokenBucket) (now : Nat) :
  (TokenBucket.refill tb now).lastRefill = now := by
  simp [TokenBucket.refill]

/-- Refill preserves maxTokens field -/
theorem refill_preserves_maxTokens (tb : TokenBucket) (now : Nat) :
  (TokenBucket.refill tb now).maxTokens = tb.maxTokens := by
  simp [TokenBucket.refill]

/-- Refill preserves refillRate field -/
theorem refill_preserves_refillRate (tb : TokenBucket) (now : Nat) :
  (TokenBucket.refill tb now).refillRate = tb.refillRate := by
  simp [TokenBucket.refill]

/-- Token bucket is deterministic for same inputs -/
theorem refill_deterministic (tb : TokenBucket) (now : Nat) :
  TokenBucket.refill tb now = TokenBucket.refill tb now := by
  rfl
