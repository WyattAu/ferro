-- Cache consistency proofs

/-- Consistent cache model with key-value mapping using Nat keys -/
structure ConsistentCache where
  entries : Nat → Option Nat
  maxSize : Nat

/-- Empty cache: all keys map to none -/
def ConsistentCache.empty (maxSize : Nat) : ConsistentCache where
  entries := fun _ => none
  maxSize := maxSize

/-- Insert a key-value pair -/
def ConsistentCache.insert (cache : ConsistentCache) (key val : Nat) : ConsistentCache :=
  { cache with entries := fun k => if k == key then some val else cache.entries k }

/-- Remove a key from the cache -/
def ConsistentCache.remove (cache : ConsistentCache) (key : Nat) : ConsistentCache :=
  { cache with entries := fun k => if k == key then none else cache.entries k }

/-- Lookup a key in the cache -/
def ConsistentCache.lookup (cache : ConsistentCache) (key : Nat) : Option Nat :=
  cache.entries key

/-- Empty cache returns None for all keys -/
theorem empty_cache_miss (key : Nat) :
  (ConsistentCache.empty 10).entries key = none := by
  rfl

/-- Insert makes key present -/
theorem insert_present (cache : ConsistentCache) (key val : Nat) :
  (cache.insert key val).entries key = some val := by
  simp [ConsistentCache.insert]

/-- Insert does not affect other keys -/
theorem insert_preserves_others (cache : ConsistentCache) (key key2 val : Nat)
    (h : key2 ≠ key) :
  (cache.insert key val).entries key2 = cache.entries key2 := by
  simp [ConsistentCache.insert, h]

/-- Insert overwrites existing value -/
theorem insert_overwrites (cache : ConsistentCache) (key val1 val2 : Nat) :
  (cache.insert key val1).insert key val2 = cache.insert key val2 := by
  simp [ConsistentCache.insert]
  funext k
  simp [Bool.decEq_eq_false_iff_ne]
  split <;> rfl

/-- Insert is idempotent for same value -/
theorem insert_idempotent (cache : ConsistentCache) (key val : Nat) :
  (cache.insert key val).insert key val = cache.insert key val := by
  simp [ConsistentCache.insert]
  funext k
  simp [Bool.decEq_eq_false_iff_ne]
  split <;> rfl

/-- Remove makes key absent -/
theorem remove_absent (cache : ConsistentCache) (key : Nat) :
  (cache.remove key).entries key = none := by
  simp [ConsistentCache.remove]

/-- Remove does not affect other keys -/
theorem remove_preserves_others (cache : ConsistentCache) (key key2 : Nat)
    (h : key2 ≠ key) :
  (cache.remove key).entries key2 = cache.entries key2 := by
  simp [ConsistentCache.remove, h]

/-- Remove is idempotent -/
theorem remove_idempotent (cache : ConsistentCache) (key : Nat) :
  (cache.remove key).remove key = cache.remove key := by
  simp [ConsistentCache.remove]
  funext k
  simp [Bool.decEq_eq_false_iff_ne]
  split <;> rfl

/-- Insert then remove at same key clears the value -/
theorem insert_then_remove (cache : ConsistentCache) (key val : Nat) :
  (cache.insert key val).remove key = cache.remove key := by
  simp [ConsistentCache.insert, ConsistentCache.remove]
  funext k
  simp [Bool.decEq_eq_false_iff_ne]
  split <;> rfl

/-- Remove then insert sets the value -/
theorem remove_then_insert (cache : ConsistentCache) (key val : Nat) :
  (cache.remove key).insert key val = cache.insert key val := by
  simp [ConsistentCache.remove, ConsistentCache.insert]
  funext k
  simp [Bool.decEq_eq_false_iff_ne]
  split <;> rfl

/-- Insert then lookup returns the inserted value -/
theorem insert_then_lookup (cache : ConsistentCache) (key val : Nat) :
  (cache.insert key val).lookup key = some val := by
  simp [ConsistentCache.lookup, ConsistentCache.insert]

/-- Remove then lookup returns none at that key -/
theorem remove_then_lookup (cache : ConsistentCache) (key : Nat) :
  (cache.remove key).lookup key = none := by
  simp [ConsistentCache.lookup, ConsistentCache.remove]

/-- Cache consistency: after insert, lookup returns value -/
theorem cache_consistent
  (cache : ConsistentCache) (key val : Nat) :
  (cache.insert key val).entries key = some val := by
  simp [ConsistentCache.insert]

/-- Empty cache lookup returns none -/
theorem empty_lookup (key : Nat) :
  (ConsistentCache.empty 10).lookup key = none := by
  rfl

/-- Remove then insert then lookup returns inserted value -/
theorem remove_insert_lookup (cache : ConsistentCache) (key val : Nat) :
  (cache.remove key).insert key val |>.lookup key = some val := by
  simp [ConsistentCache.remove, ConsistentCache.insert, ConsistentCache.lookup]

/-- Lookup is deterministic for same cache and key -/
theorem lookup_deterministic (cache : ConsistentCache) (key : Nat) :
  cache.lookup key = cache.lookup key := by
  rfl

/-- Insert preserves maxSize -/
theorem insert_preserves_maxSize (cache : ConsistentCache) (key val : Nat) :
  (cache.insert key val).maxSize = cache.maxSize := by
  simp [ConsistentCache.insert]

/-- Remove preserves maxSize -/
theorem remove_preserves_maxSize (cache : ConsistentCache) (key : Nat) :
  (cache.remove key).maxSize = cache.maxSize := by
  simp [ConsistentCache.remove]

/-- Empty cache has correct maxSize -/
theorem empty_maxSize (maxSize : Nat) :
  (ConsistentCache.empty maxSize).maxSize = maxSize := by
  rfl
