-- Basic definitions for formal verification

/-- A natural number is even if it is divisible by 2 -/
def Even (n : Nat) : Prop :=
  ∃ k, n = 2 * k

/-- Proof that 0 is even -/
theorem zero_is_even : Even 0 :=
  ⟨0, by decide⟩

/-- Proof that the sum of two even numbers is even -/
theorem even_plus_even {m n : Nat} (hm : Even m) (hn : Even n) : Even (m + n) := by
  obtain ⟨k₁, rfl⟩ := hm
  obtain ⟨k₂, rfl⟩ := hn
  exact ⟨k₁ + k₂, by omega⟩
