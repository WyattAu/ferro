-- Formal verification of core data structures

/-- Content-addressable storage hash -/
structure ContentHash where
  hash : String
  deriving Repr, BEq

/-- Hash is 64 hex characters -/
def ContentHash.isValid (h : ContentHash) : Prop :=
  h.hash.length = 64 ∧ ∀ c ∈ h.hash.data, c ∈ "0123456789abcdef".data

/-- Two equal hashes have equal strings -/
theorem hash_eq_iff_string_eq (h1 h2 : ContentHash) :
  h1 = h2 ↔ h1.hash = h2.hash := by
  constructor
  · intro heq; rw [heq]
  · intro heq; cases h1; cases h2; congr
