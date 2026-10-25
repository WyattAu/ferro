-- Path safety proofs (Lean 4.12).
-- Component-based traversal prevention: a path is safe iff no "/"-separated
-- component is ".." (lexical normalization; the runtime companion is
-- openat2(RESOLVE_BENEATH) — kernel-enforced containment, see
-- YP-TENANCY-ISOLATION-001).

set_option linter.unusedVariables false

/-- A path is safe if no component is ".." (component-based check) -/
def Path.componentsSafe (path : String) : Prop :=
  ∀ part ∈ path.splitOn "/", part ≠ ".."

/-- A path component is safe if it's not ".." -/
def Path.componentSafe (part : String) : Prop :=
  part ≠ ".."

/-- Root path is safe (components ["", ""]) -/
theorem root_safe : Path.componentsSafe "/" := by
  intro part hpart
  have h : "/".splitOn "/" = ["", ""] := by rfl
  rw [h] at hpart
  rcases List.mem_cons.mp hpart with heq | htail
  · subst heq; decide
  · rcases List.mem_cons.mp htail with heq | hnil
    · subst heq; decide
    · exact absurd hnil (by simp)

/-- Empty path is safe (single empty component) -/
theorem empty_safe : Path.componentsSafe "" := by
  intro part hpart
  have h : "".splitOn "/" = [""] := by rfl
  rw [h] at hpart
  rcases List.mem_cons.mp hpart with heq | hnil
  · subst heq; decide
  · exact absurd hnil (by simp)

/-- A path is safe when its computed components are all non-".." —
    the general form; concrete instances below instantiate `parts`. -/
theorem componentsSafe_of_components (path : String) (parts : List String)
    (hsplit : path.splitOn "/" = parts) (hsafe : ∀ p ∈ parts, p ≠ "..") :
    Path.componentsSafe path := by
  unfold Path.componentsSafe
  rw [hsplit]
  exact hsafe

/-- Single-name path is safe when the name is not ".." (symbolic form:
    the caller supplies the component decomposition — no "/" inside the
    name). -/
theorem simple_safe (name : String)
    (hsplit : ("/" ++ name).splitOn "/" = ["", name]) (h : name ≠ "..") :
    Path.componentsSafe ("/" ++ name) :=
  componentsSafe_of_components ("/" ++ name) ["", name] hsplit (by
    intro p hp
    rcases List.mem_cons.mp hp with heq | hp
    · subst heq; decide
    · rcases List.mem_cons.mp hp with heq | hnil
      · subst heq; exact h
      · exact absurd hnil (by simp))

/-- Path with traversal is unsafe -/
theorem traversal_unsafe : ¬Path.componentsSafe "/foo/../bar" := by
  intro hsafe
  have h : "/foo/../bar".splitOn "/" = ["", "foo", "..", "bar"] := by rfl
  unfold Path.componentsSafe at hsafe
  rw [h] at hsafe
  have hmem : ".." ∈ ["", "foo", "..", "bar"] := by simp
  exact hsafe ".." hmem rfl

/-- Path "/foo/.." is unsafe -/
theorem dotdot_end_unsafe : ¬Path.componentsSafe "/foo/.." := by
  intro hsafe
  have h : "/foo/..".splitOn "/" = ["", "foo", ".."] := by rfl
  unfold Path.componentsSafe at hsafe
  rw [h] at hsafe
  have hmem : ".." ∈ ["", "foo", ".."] := by simp
  exact hsafe ".." hmem rfl

/-- Path "/../foo" is unsafe -/
theorem dotdot_start_unsafe : ¬Path.componentsSafe "/../foo" := by
  intro hsafe
  have h : "/../foo".splitOn "/" = ["", "..", "foo"] := by rfl
  unfold Path.componentsSafe at hsafe
  rw [h] at hsafe
  have hmem : ".." ∈ ["", "..", "foo"] := by simp
  exact hsafe ".." hmem rfl

/-- Safe component is not ".." -/
theorem safe_component_not_dotdot (part : String) (h : Path.componentSafe part) :
  part ≠ ".." := h

/-- "/" is a safe separator -/
theorem slash_safe : "/" ≠ ".." := by decide

/-- Empty string is a safe component -/
theorem empty_component_safe : "" ≠ ".." := by decide

/-- Multiple "/" separators don't affect safety -/
theorem multi_slash_safe : Path.componentsSafe "///" := by
  intro part hpart
  have h : "///".splitOn "/" = ["", "", "", ""] := by rfl
  rw [h] at hpart
  rcases List.mem_cons.mp hpart with heq | htail
  · subst heq; decide
  · rcases List.mem_cons.mp htail with heq | htail
    · subst heq; decide
    · rcases List.mem_cons.mp htail with heq | htail
      · subst heq; decide
      · rcases List.mem_cons.mp htail with heq | htail
        · subst heq; decide
        · exact absurd htail (by simp)

/-- Path with dots (not "..") is safe -/
theorem single_dot_safe : Path.componentsSafe "/." := by
  intro part hpart
  have h : "/.".splitOn "/" = ["", "."] := by rfl
  rw [h] at hpart
  rcases List.mem_cons.mp hpart with heq | htail
  · subst heq; decide
  · rcases List.mem_cons.mp htail with heq | htail
    · subst heq; decide
    · exact absurd htail (by simp)

/-- Path with dots at end is safe -/
theorem dot_at_end_safe : Path.componentsSafe "/foo/." := by
  intro part hpart
  have h : "/foo/.".splitOn "/" = ["", "foo", "."] := by rfl
  rw [h] at hpart
  rcases List.mem_cons.mp hpart with heq | htail
  · subst heq; decide
  · rcases List.mem_cons.mp htail with heq | htail
    · subst heq; decide
    · rcases List.mem_cons.mp htail with heq | htail
      · subst heq; decide
      · exact absurd htail (by simp)

/-- THM-PATH-CHECK-COMPLETE: the lexical check catches ANY parent
    reference anywhere in the path — there is no traversal encoding that
    slips past `componentsSafe` while containing a ".." component.
    (Encoding-level bypasses like "..%2F" are the URL-decoder's
    responsibility: this check runs on the DECODED path.) -/
theorem dotdot_detected (path : String) (hmem : ".." ∈ path.splitOn "/") :
    ¬ Path.componentsSafe path := by
  intro hsafe
  exact hsafe ".." hmem rfl
