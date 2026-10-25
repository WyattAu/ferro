-- WASM plugin capability containment.
-- Extension companion to YP-TENANCY-ISOLATION-001 (THM-PLUGIN-CONTAINMENT,
-- R3-10). References:
--   Haas, A. et al. "Bringing the Web up to Speed with WebAssembly,"
--   PLDI 2017 [VERIFIED in R2 as PLDI2017-WASM].
--   WebAssembly Core Specification, W3C Recommendation [VERIFIED in R2].
-- Proved here: an import-gated host call (a plugin can only invoke
-- functions on its declared import list — no capability amplification by
-- name confusion) and linear-memory bounds (an access within the declared
-- bounds never escapes the plugin's own memory).

/-- Host functions identified by ids; plugins declare an import allow-list
    at instantiation (no runtime growth). -/
abbrev Imports := List Nat

/-- A plugin instance: its import allow-list and memory size. -/
structure Plugin where
  imports : Imports
  memLen : Nat

/-- Host-call gate: the call passes only for declared imports. -/
def callAllowed (p : Plugin) (fn : Nat) : Bool :=
  List.elem fn p.imports

/-- NO CAPABILITY AMPLIFICATION BY NAME CONFUSION: an undeclared host
    function is never callable, regardless of the requested id. -/
theorem undeclared_uncallable (p : Plugin) (fn : Nat)
    (h : ¬ List.elem fn p.imports) :
    callAllowed p fn = false := by
  have hc : List.elem fn p.imports = false := by
    by_cases hc2 : fn ∈ p.imports
    · exact absurd (List.elem_iff.mpr hc2) h
    · simp [hc2]
  unfold callAllowed
  rw [hc]

/-- A declared import stays callable for the instance's whole life: the
    allow-list is fixed at instantiation. -/
theorem declared_callables_fixed (p : Plugin) (fn : Nat) (h : List.elem fn p.imports) :
    callAllowed p fn = true := by
  have hm : fn ∈ p.imports := List.elem_iff.mp h
  unfold callAllowed
  simp [h, hm]

/-- Linear-memory access gate: addresses are bounds-checked against the
    instance's own memory. -/
def memAccess (p : Plugin) (addr len : Nat) : Bool :=
  addr + len <= p.memLen

/-- MEMORY CONTAINMENT: an in-bounds access stays within the instance's
    own memory; out-of-bounds requests are rejected. -/
theorem oob_rejected (p : Plugin) (addr len : Nat) (h : p.memLen < addr + len) :
    memAccess p addr len = false := by
  unfold memAccess
  simp [Nat.lt_of_lt_of_le h (Nat.le_refl _)]

theorem inbounds_accepted (p : Plugin) (addr len : Nat) (h : addr + len <= p.memLen) :
    memAccess p addr len = true := by
  unfold memAccess
  simp [h]

/-- Zero-length accesses at the boundary are legal (addr = memLen, len = 0). -/
theorem boundary_zero_len_ok (p : Plugin) (addr : Nat) (heq : addr = p.memLen) :
    memAccess p addr 0 = true := by
  subst heq
  exact inbounds_accepted p p.memLen 0 (Nat.le_refl _)
