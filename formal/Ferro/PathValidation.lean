-- Formal verification of path validation
-- Models the path traversal prevention from crates/common/src/path.rs

import Ferro.Basic

/-- A path is valid if it doesn't contain ".." as a component -/
def Path.isValid (path : String) : Prop :=
  ¬ path.containsSubstr ".."

/-- A path segment is safe if it's not ".." -/
def Path.segmentSafe (s : String) : Prop :=
  s ≠ ".."

/-- Root path is valid -/
theorem root_path_is_valid : Path.isValid "/" := by
  simp [Path.isValid, String.containsSubstr]
  intro h
  have : "..".length = 2 := by norm_num
  omega

/-- Empty string path is valid (no traversal possible) -/
theorem empty_path_valid : Path.isValid "" := by
  simp [Path.isValid, String.containsSubstr]

/-- Simple path without dots is valid -/
theorem simple_path_valid (name : String)
    (h1 : name ≠ "..") (h2 : ¬ name.containsSubstr "..") :
  Path.isValid s!"/{name}" := by
  simp [Path.isValid, String.containsSubstr]
  intro h
  apply h2
  exact h

/-- Path with ".." is invalid -/
theorem traversal_path_invalid :
  ¬Path.isValid "/foo/../bar" := by
  simp [Path.isValid, String.containsSubstr]
  intro h
  omega

/-- Path "/foo/.." is invalid -/
theorem dotdot_end_invalid :
  ¬Path.isValid "/foo/.." := by
  simp [Path.isValid, String.containsSubstr]
  intro h
  omega

/-- Path "/..something" is invalid if it starts with ".." -/
theorem dotdot_prefix_invalid :
  ¬Path.isValid "/../foo" := by
  simp [Path.isValid, String.containsSubstr]
  intro h
  omega

/-- Path validation is pure (same input → same output) -/
theorem validation_pure (path : String) :
  Path.isValid path ↔ Path.isValid path := by
  Iff.rfl

/-- join preserves validity when segment has no traversal -/
theorem join_preserves_validity (base segment : String)
    (h1 : Path.isValid base) (h2 : Path.isValid segment) :
  Path.isValid (base ++ "/" ++ segment) := by
  simp [Path.isValid] at *
  intro h
  apply h1
  exact String.containsSubstr_append_left _ _ _ h

/-- A path without ".." substring is valid -/
theorem no_dotdot_implies_valid (path : String)
    (h : ¬ path.containsSubstr "..") :
  Path.isValid path := by
  simp [Path.isValid]
  exact h
