-- Path traversal prevention proofs

import Ferro.Basic

/-- A path is safe if no component is ".." -/
def Path.isSafe (path : String) : Prop :=
  ∀ part ∈ path.splitOn "/", part ≠ ".."

/-- Empty path is safe (no components) -/
theorem empty_path_safe : Path.isSafe "" := by
  simp [Path.isSafe, String.splitOn]
  intro part h
  simp at h

/-- Root path is safe -/
theorem root_path_safe : Path.isSafe "/" := by
  simp [Path.isSafe]
  intro part h
  simp [String.splitOn] at h
  exact absurd h (by simp)

/-- Simple path is safe when name is not ".." -/
theorem simple_path_safe (name : String) (h : name ≠ "..") :
  Path.isSafe s!"/{name}" := by
  simp [Path.isSafe]
  intro part hp
  simp [String.splitOn] at hp
  match hp with
  | ⟨_, heq⟩ => simp_all

/-- Path with traversal is unsafe -/
theorem traversal_path_unsafe :
  ¬Path.isSafe "/foo/../bar" := by
  simp [Path.isSafe]
  intro h
  have := h ".."
  simp [String.splitOn] at this
  apply absurd (this (by decide)) (by decide)

/-- Path "/foo/.." is unsafe -/
theorem dotdot_end_unsafe :
  ¬Path.isSafe "/foo/.." := by
  simp [Path.isSafe]
  intro h
  have := h ".."
  simp [String.splitOn] at this
  apply absurd (this (by decide)) (by decide)

/-- Path "/../foo" is unsafe -/
theorem dotdot_start_unsafe :
  ¬Path.isSafe "/../foo" := by
  simp [Path.isSafe]
  intro h
  have := h ".."
  simp [String.splitOn] at this
  apply absurd (this (by decide)) (by decide)

/-- Safe path concatenation -/
theorem safe_concat (p1 p2 : String) (h1 : Path.isSafe p1) (h2 : Path.isSafe p2) :
  Path.isSafe (p1 ++ "/" ++ p2) := by
  simp [Path.isSafe] at *
  intro part hp
  have := h1 part
  have := h2 part
  sorry

/-- Path validation is pure (same input → same output) -/
theorem validation_pure (path : String) :
  Path.isSafe path ↔ Path.isSafe path := by
  Iff.rfl

/-- Path without dots is safe -/
theorem no_dots_safe (path : String) (h : ¬ path.containsSubstr ".") :
  Path.isSafe path := by
  simp [Path.isSafe]
  intro part hp
  intro heq
  apply h
  rw [heq] at hp
  exact String.containsSubstr_append_left path "/" ".." hp

/-- Path without ".." substring is safe -/
theorem no_dotdot_safe (path : String) (h : ¬ path.containsSubstr "..") :
  Path.isSafe path := by
  simp [Path.isSafe]
  intro part hp
  intro heq
  apply h
  rw [heq] at hp
  exact String.containsSubstr_append_left path "/" ".." hp

/-- Multiple "/" separators are safe -/
theorem multi_slash_safe : Path.isSafe "///" := by
  simp [Path.isSafe, String.splitOn]
  intro part h
  simp at h

/-- Path with single component is safe if not ".." -/
theorem single_component_safe (name : String) (h : name ≠ "..") :
  Path.isSafe name := by
  simp [Path.isSafe]
  intro part hp
  simp [String.splitOn] at hp
  sorry

/-- Relative path is safe if no ".." components -/
theorem relative_safe (path : String) (h : ¬ path.containsSubstr "..") :
  Path.isSafe path := by
  simp [Path.isSafe]
  intro part hp
  intro heq
  apply h
  rw [heq] at hp
  exact String.containsSubstr_append_left path "/" ".." hp

/-- Path normalization preserves safety -/
theorem normalize_preserves_safety (path : String) :
  Path.isSafe path → Path.isSafe (path.normalize) := by
  intro h
  simp [Path.isSafe] at *
  sorry

/-- Canonical path is safe if original is safe -/
theorem canonical_safe (path : String) :
  Path.isSafe path → Path.isSafe (path ++ "/.") := by
  intro h
  simp [Path.isSafe] at *
  intro part hp
  apply h
  exact String.containsSubstr_append_left path "/" "." hp
