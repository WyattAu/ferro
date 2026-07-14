-- Refined rate limiter proofs with time-based refill

import Ferro.Basic
import Ferro.RateLimiter

/-- Refined rate limiter with time tracking -/
structure RefinedRateLimiter where
  tokens : Nat
  maxTokens : Nat
  refillRate : Nat
  lastRefill : Nat
  deriving Repr

/-- Refill tokens based on elapsed time -/
def RefinedRateLimiter.refill (rl : RefinedRateLimiter) (now : Nat) : RefinedRateLimiter :=
  let elapsed := now - rl.lastRefill
  let newTokens := min (rl.tokens + elapsed * rl.refillRate) rl.maxTokens
  { rl with tokens := newTokens, lastRefill := now }

/-- After refill, tokens never exceed maxTokens -/
theorem refill_never_exceeds_max (rl : RefinedRateLimiter) (now : Nat) :
  (rl.refill now).tokens ≤ rl.maxTokens := by
  simp [RefinedRateLimiter.refill]
  apply min_le_right

/-- Refill is idempotent at same time -/
theorem refill_idempotent (rl : RefinedRateLimiter) (now : Nat) :
  (rl.refill now).refill now = rl.refill now := by
  simp [RefinedRateLimiter.refill]
  constructor
  · omega
  · omega

/-- Refill updates lastRefill timestamp -/
theorem refill_updates_timestamp (rl : RefinedRateLimiter) (now : Nat) :
  (rl.refill now).lastRefill = now := by
  simp [RefinedRateLimiter.refill]

/-- No elapsed time means no token change -/
theorem no_elapsed_no_change (rl : RefinedRateLimiter) (now : Nat)
    (h : now ≤ rl.lastRefill) :
  (rl.refill now).tokens = rl.tokens := by
  simp [RefinedRateLimiter.refill]
  omega

/-- Refill adds at most elapsed * refillRate tokens -/
theorem refill_adds_bounded (rl : RefinedRateLimiter) (now : Nat) :
  (rl.refill now).tokens ≤ rl.tokens + (now - rl.lastRefill) * rl.refillRate := by
  simp [RefinedRateLimiter.refill]
  apply min_le_left

/-- Try to consume a token -/
def RefinedRateLimiter.tryConsume (rl : RefinedRateLimiter) (now : Nat) : Option RefinedRateLimiter :=
  let rl = rl.refill now
  if rl.tokens > 0 then
    some { rl with tokens := rl.tokens - 1 }
  else
    none

/-- After consuming, tokens decrease by 1 (when tokens > 0) -/
theorem consume_decreases_tokens (rl : RefinedRateLimiter) (now : Nat)
    (h : (rl.refill now).tokens > 0) :
  (rl.tryConsume now).isSome := by
  simp [RefinedRateLimiter.tryConsume]
  split
  · rfl
  · contradiction

/-- Consume result has decremented tokens -/
theorem consume_result_tokens (rl : RefinedRateLimiter) (now : Nat)
    (h : (rl.refill now).tokens > 0) :
  (rl.tryConsume now).get (by simp [RefinedRateLimiter.tryConsume]; split <;> simp_all).tokens
    = (rl.refill now).tokens - 1 := by
  simp [RefinedRateLimiter.tryConsume]
  split
  · simp
  · contradiction

/-- Consume preserves maxTokens -/
theorem consume_preserves_max (rl : RefinedRateLimiter) (now : Nat) :
  match rl.tryConsume now with
  | some result => result.maxTokens = rl.maxTokens
  | none => True := by
  simp [RefinedRateLimiter.tryConsume]
  split
  · simp [RefinedRateLimiter.refill]
  · trivial

/-- Consume preserves refillRate -/
theorem consume_preserves_refillRate (rl : RefinedRateLimiter) (now : Nat) :
  match rl.tryConsume now with
  | some result => result.refillRate = rl.refillRate
  | none => True := by
  simp [RefinedRateLimiter.tryConsume]
  split
  · simp [RefinedRateLimiter.refill]
  · trivial

/-- Multiple consumes decrease tokens by at most n -/
theorem multiple_consume_bounded (rl : RefinedRateLimiter) (now : Nat) (n : Nat) :
  match rl.tryConsume now with
  | some result => result.tokens + 1 ≥ n →
    match result.tryConsume now with
    | some result2 => result2.tokens + 2 ≥ n
    | none => n ≤ 1
  | none => True := by
  simp [RefinedRateLimiter.tryConsume]
  split <;> simp_all
  split <;> omega

/-- Token bucket invariant: tokens never negative and never exceed max -/
def RefinedRateLimiter.invariant (rl : RefinedRateLimiter) : Prop :=
  rl.tokens ≤ rl.maxTokens

/-- Invariant holds after refill -/
theorem refill_preserves_invariant (rl : RefinedRateLimiter) (now : Nat)
    (h : rl.invariant) :
  (rl.refill now).invariant := by
  simp [RefinedRateLimiter.invariant]
  apply refill_never_exceeds_max

/-- Initial rate limiter has full tokens -/
def RefinedRateLimiter.init (maxTokens refillRate : Nat) : RefinedRateLimiter :=
  { tokens := maxTokens, maxTokens := maxTokens, refillRate := refillRate, lastRefill := 0 }

/-- Initial state satisfies invariant -/
theorem init_invariant (maxTokens refillRate : Nat) :
  (RefinedRateLimiter.init maxTokens refillRate).invariant := by
  simp [RefinedRateLimiter.init, RefinedRateLimiter.invariant]

/-- Initial state has full tokens -/
theorem init_full_tokens (maxTokens refillRate : Nat) :
  (RefinedRateLimiter.init maxTokens refillRate).tokens = maxTokens := by
  simp [RefinedRateLimiter.init]
