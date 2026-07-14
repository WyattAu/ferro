-- Cache invalidation properties
-- Models cache consistency with write-through invalidation

/-- Cache with key-value pairs using function model -/
structure InvalidationCache where
  data : Nat → Option Nat

/-- Empty cache -/
def InvalidationCache.empty : InvalidationCache where
  data := fun _ => none

/-- Write data to cache -/
def InvalidationCache.write (cache : InvalidationCache) (key val : Nat) : InvalidationCache :=
  { cache with data := fun k => if k == key then some val else cache.data k }

/-- Read from cache -/
def InvalidationCache.read (cache : InvalidationCache) (key : Nat) : Option Nat :=
  cache.data key

/-- Invalidate a key (remove from cache) -/
def InvalidationCache.invalidate (cache : InvalidationCache) (key : Nat) : InvalidationCache :=
  { cache with data := fun k => if k == key then none else cache.data k }

/-- Check if key is in cache -/
def InvalidationCache.contains (cache : InvalidationCache) (key : Nat) : Bool :=
  cache.data key != none

/-- Invalidation completeness: if key in cache and invalidated, then key not in cache after -/
theorem invalidation_completeness (cache : InvalidationCache) (key : Nat)
    (h : cache.contains key = true) :
  (cache.invalidate key).contains key = false := by
  simp [InvalidationCache.contains, InvalidationCache.invalidate] at *

/-- Invalidation idempotency: invalidate(invalidate(cache, key)) = invalidate(cache, key) -/
theorem invalidation_idempotent (cache : InvalidationCache) (key : Nat) :
  (cache.invalidate key).invalidate key = cache.invalidate key := by
  simp [InvalidationCache.invalidate]
  funext k
  split <;> rfl

/-- Consistency after write: if write(data) then read() returns data -/
theorem consistency_after_write (cache : InvalidationCache) (key val : Nat) :
  (cache.write key val).read key = some val := by
  simp [InvalidationCache.write, InvalidationCache.read]

/-- Write preserves other keys -/
theorem write_preserves_others (cache : InvalidationCache) (key key2 val : Nat)
    (h : key2 ≠ key) :
  (cache.write key val).read key2 = cache.read key2 := by
  simp [InvalidationCache.write, InvalidationCache.read]
  omega

/-- Invalidate makes key absent -/
theorem invalidate_absent (cache : InvalidationCache) (key : Nat) :
  (cache.invalidate key).read key = none := by
  simp [InvalidationCache.invalidate, InvalidationCache.read]

/-- Invalidate preserves other keys -/
theorem invalidate_preserves_others (cache : InvalidationCache) (key key2 : Nat)
    (h : key2 ≠ key) :
  (cache.invalidate key).read key2 = cache.read key2 := by
  simp [InvalidationCache.invalidate, InvalidationCache.read]
  omega

/-- Write then invalidate clears the key -/
theorem write_then_invalidate (cache : InvalidationCache) (key val : Nat) :
  (cache.write key val).invalidate key = cache.invalidate key := by
  simp [InvalidationCache.write, InvalidationCache.invalidate]
  funext k
  split <;> rfl

/-- Invalidate then write sets the value (overwrite clears invalidation) -/
theorem invalidate_then_write (cache : InvalidationCache) (key val : Nat) :
  (cache.invalidate key).write key val = cache.write key val := by
  simp [InvalidationCache.invalidate, InvalidationCache.write]
  funext k
  split <;> rfl

/-- Empty cache read returns none -/
theorem empty_read (key : Nat) :
  InvalidationCache.empty.read key = none := by
  rfl

/-- Write then read returns written value (deterministic) -/
theorem write_read_deterministic (cache : InvalidationCache) (key val : Nat) :
  (cache.write key val).read key = some val := by
  simp [InvalidationCache.write, InvalidationCache.read]

/-- Multiple writes to same key returns last value (last-write-wins) -/
theorem write_overwrites (cache : InvalidationCache) (key val1 val2 : Nat) :
  (cache.write key val1).write key val2 = cache.write key val2 := by
  simp [InvalidationCache.write]
  funext k
  split <;> rfl
