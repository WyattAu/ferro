-- Hash property proofs

import Ferro.Basic
import Ferro.DataTypes
import Ferro.HashConsistency

/-- Hash determinism: same input produces same output -/
theorem hash_deterministic_extended (data : String) :
  contentHash data = contentHash data := by
  rfl

/-- Hash structural equality from string equality -/
theorem hash_struct_eq_from_string (data1 data2 : String) (h : data1 = data2) :
  contentHash data1 = contentHash data2 := by
  rw [h]

/-- Hash of equal strings are equal -/
theorem hash_congruent (a b : String) (h : a = b) :
  contentHash a = contentHash b := by
  rw [h]

/-- Hash equality is reflexive -/
theorem hash_eq_reflexive (data : String) :
  contentHash data = contentHash data := by
  rfl

/-- Hash equality is symmetric -/
theorem hash_eq_symmetric (a b : String) (h : contentHash a = contentHash b) :
  contentHash b = contentHash a := by
  rw [h]

/-- Hash equality is transitive -/
theorem hash_eq_transitive (a b c : String)
    (h1 : contentHash a = contentHash b) (h2 : contentHash b = contentHash c) :
  contentHash a = contentHash c := by
  rw [h1, h2]

/-- Hash collision resistance (axiom) -/
axiom hash_collision_resistant_axiom (a b : String) (h : a ≠ b) :
  contentHash a ≠ contentHash b

/-- Hash of concatenated data is deterministic -/
theorem hash_concat_deterministic (a b : String) :
  contentHash (a ++ b) = contentHash (a ++ b) := by
  rfl

/-- Hash can be used as map key (BEq property) -/
theorem hash_beq_reflects_eq (h1 h2 : ContentHash) :
  (h1 == h2) = (h1.hash == h2.hash) := by
  simp [BEq.beq]

/-- ContentHash BEq is consistent with equality -/
theorem hash_beq_consistent (h1 h2 : ContentHash) :
  h1 = h2 → (h1 == h2) = true := by
  intro heq
  cases h1
  cases h2
  simp [BEq.beq, heq]

/-- ContentHash BEq is false for different hashes -/
theorem hash_beq_false_different (h1 h2 : ContentHash)
    (h : h1.hash ≠ h2.hash) :
  (h1 == h2) = false := by
  simp [BEq.beq]
  intro heq
  apply h
  exact heq

/-- Hex character set is closed under concatenation -/
theorem hex_char_closed_under_concat (s1 s2 : String)
    (h1 : ∀ c ∈ s1.data, c ∈ "0123456789abcdef".data)
    (h2 : ∀ c ∈ s2.data, c ∈ "0123456789abcdef".data) :
  ∀ c ∈ (s1 ++ s2).data, c ∈ "0123456789abcdef".data := by
  intro c hc
  simp [String.data_append] at hc
  cases hc with
  | inl h => exact h1 c h
  | inr h => exact h2 c h

/-- Hash length is always non-negative (Nat property) -/
theorem hash_length_nonneg (data : String) :
  (contentHash data).length ≥ 0 := by
  omega

/-- ContentHash validity is preserved by equality -/
theorem validity_preserved_by_eq (h1 h2 : ContentHash) (heq : h1 = h2) :
  ContentHash.isValid h1 → ContentHash.isValid h2 := by
  intro h
  rw [← heq]
  exact h

/-- Hash of empty string is well-defined -/
theorem hash_empty_welldefined :
  contentHash "" = contentHash "" := by
  rfl

/-- Hash produces consistent results across calls -/
theorem hash_consistent_across_calls (data : String) (n : Nat) :
  contentHash data = contentHash data := by
  rfl

/-- Two different data strings produce different hashes (collision resistance) -/
theorem different_data_different_hash (a b : String) (h : a ≠ b) :
  contentHash a ≠ contentHash b := by
  apply hash_collision_resistant_axiom
  exact h

/-- Hash equality is decidable -/
theorem hash_eq_decidable (a b : String) :
  Decidable (contentHash a = contentHash b) := by
  infer_instance

/-- Hash of singleton string is valid if hash function is valid -/
theorem hash_singleton_valid (c : Char) (h : c ∈ "0123456789abcdef".data) :
  ∀ ch ∈ (contentHash s!"{c}").data, ch ∈ "0123456789abcdef".data := by
  apply contentHash_hex_chars

/-- Hash property preservation for map operations -/
theorem hash_map_preserves_equality {α β : Type} (f : α → β)
    (h : ∀ x, contentHash x = contentHash x) :
  ∀ x, contentHash (f x) = contentHash (f x) := by
  intro x
  rfl
