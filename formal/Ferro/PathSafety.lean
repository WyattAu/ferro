-- Path safety proofs
-- TODO: Update proofs for Lean4 v4.12.0 API changes.
-- omega no longer handles String.splitOn list reasoning.
-- introN tactic changed. These proofs need manual reconstruction.

/-- A path is safe if no component is ".." (component-based check) -/
def Path.componentsSafe (path : String) : Prop :=
  ∀ part ∈ path.splitOn "/", part ≠ ".."

/-- A path component is safe if it's not ".." -/
def Path.componentSafe (part : String) : Prop :=
  part ≠ ".."

/-- Root path is safe -/
theorem root_safe : Path.componentsSafe "/" := by
  simp [Path.componentsSafe, String.splitOn]
  intro part h
  sorry -- TODO: prove "" ≠ ".."

/-- Empty path is safe (no components) -/
theorem empty_safe : Path.componentsSafe "" := by
  simp [Path.componentsSafe, String.splitOn]

/-- Simple path is safe when name is not ".." -/
theorem simple_safe (name : String) (h : name ≠ "..") :
  Path.componentsSafe s!"/{name}" := by
  simp [Path.componentsSafe, String.splitOn]
  intro part hp
  sorry -- TODO: prove part ∈ [name] → part ≠ ".."

/-- Path with traversal is unsafe -/
theorem traversal_unsafe :
  ¬Path.componentsSafe "/foo/../bar" := by
  sorry -- TODO: reconstruct proof for v4.12.0

/-- Path "/foo/.." is unsafe -/
theorem dotdot_end_unsafe :
  ¬Path.componentsSafe "/foo/.." := by
  sorry -- TODO: reconstruct proof for v4.12.0

/-- Path "/../foo" is unsafe -/
theorem dotdot_start_unsafe :
  ¬Path.componentsSafe "/../foo" := by
  sorry -- TODO: reconstruct proof for v4.12.0

/-- Safe component is not ".." -/
theorem safe_component_not_dotdot (part : String) (h : Path.componentSafe part) :
  part ≠ ".." := h

/-- "/" is a safe separator -/
theorem slash_safe : "/" ≠ ".." := by decide

/-- Empty string is a safe component -/
theorem empty_component_safe : "" ≠ ".." := by decide

/-- Multiple "/" separators don't affect safety -/
theorem multi_slash_safe : Path.componentsSafe "///" := by
  simp [Path.componentsSafe, String.splitOn]

/-- Path with dots (not "..") is safe -/
theorem single_dot_safe : Path.componentsSafe "/." := by
  simp [Path.componentsSafe, String.splitOn]
  intro part h
  sorry -- TODO: prove ". " ≠ ".."

/-- Path with dots at end is safe -/
theorem dot_at_end_safe : Path.componentsSafe "/foo/." := by
  simp [Path.componentsSafe, String.splitOn]
  intro part h
  sorry -- TODO: prove part ∈ ["foo", "."] → part ≠ ".."
