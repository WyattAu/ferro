-- Formal verification of token bucket rate limiter
-- Models the core algorithm from crates/rate-limiter/src/bucket.rs

import Ferro.Basic

/-- Token bucket state -/
structure BucketState where
  tokens : Nat
  maxTokens : Nat
  deriving Repr, BEq

/-- Rate limiter configuration -/
structure RateLimiter where
  maxTokens : Nat
  refillRate : Nat
  deriving Repr

/-- Initial bucket state has full tokens -/
def RateLimiter.initBucket (rl : RateLimiter) : BucketState where
  tokens := rl.maxTokens
  maxTokens := rl.maxTokens

/-- Refill tokens up to max -/
def BucketState.refill (state : BucketState) (refillRate : Nat) : BucketState :=
  { state with tokens := min (state.tokens + refillRate) state.maxTokens }

/-- Try to consume a token from the bucket -/
def BucketState.tryConsume (state : BucketState) : Option BucketState :=
  if state.tokens > 0 then
    some { state with tokens := state.tokens - 1 }
  else
    none

/-- Consume a fixed cost from the bucket -/
def BucketState.consume (state : BucketState) (cost : Nat) : BucketState :=
  { state with tokens := state.tokens - min cost state.tokens }

/-- Initial state has full tokens -/
theorem init_has_full_tokens (rl : RateLimiter) :
  (rl.initBucket).tokens = rl.maxTokens := by
  simp [RateLimiter.initBucket]

/-- Initial state tokens equal maxTokens -/
theorem init_tokens_eq_max (rl : RateLimiter) :
  (rl.initBucket).tokens = (rl.initBucket).maxTokens := by
  simp [RateLimiter.initBucket]

/-- Refill never exceeds maxTokens -/
theorem refill_never_exceeds_max (state : BucketState) (refillRate : Nat) :
  (state.refill refillRate).tokens ≤ state.maxTokens := by
  simp [BucketState.refill]
  apply min_le_right

/-- Refill is non-decreasing when below max -/
theorem refill_non_decreasing (state : BucketState) (refillRate : Nat)
    (h : state.tokens < state.maxTokens) :
  state.tokens ≤ (state.refill refillRate).tokens := by
  simp [BucketState.refill]
  apply le_min
  · omega
  · exact le_refl state.tokens

/-- Consume when tokens > 0 produces Some with decremented tokens -/
theorem consume_decreases_tokens (state : BucketState) (h : state.tokens > 0) :
  (state.tryConsume).isSome = true := by
  simp [BucketState.tryConsume]
  simp [h]

/-- Consume when tokens = 0 produces None -/
theorem consume_fails_when_empty (state : BucketState) (h : state.tokens = 0) :
  state.tryConsume = none := by
  simp [BucketState.tryConsume]
  simp [h]

/-- After successful consume, tokens decrease by 1 -/
theorem consume_result_decreases (state : BucketState) (h : state.tokens > 0) :
  (state.tryConsume.get (by simp [BucketState.tryConsume]; simp [h])).tokens
    = state.tokens - 1 := by
  simp [BucketState.tryConsume]
  simp [h]

/-- Consume with cost = 0 is identity -/
theorem consume_zero_is_identity (state : BucketState) :
  (state.consume 0).tokens = state.tokens := by
  simp [BucketState.consume]

/-- Consume with cost ≥ tokens empties bucket to 0 -/
theorem consume_overflow_empties (state : BucketState) (cost : Nat) (h : cost ≥ state.tokens) :
  (state.consume cost).tokens = 0 := by
  simp [BucketState.consume]
  omega

/-- Fixed cost consume is bounded by tokens -/
theorem consume_bounded (state : BucketState) (cost : Nat) :
  (state.consume cost).tokens ≤ state.tokens := by
  simp [BucketState.consume]

/-- Init bucket: tokens always non-negative (trivially true for Nat) -/
theorem init_tokens_nonneg (rl : RateLimiter) :
  (rl.initBucket).tokens ≥ 0 := by
  omega

/-- Refill preserves non-negativity -/
theorem refill_preserves_nonneg (state : BucketState) (refillRate : Nat) :
  (state.refill refillRate).tokens ≥ 0 := by
  omega
