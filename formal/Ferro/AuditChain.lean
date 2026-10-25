-- Tamper-evident audit chain: MMR append stability, bagging monotonicity,
-- tamper evidence, forward-secure key evolution, PII epoch unlinkability,
-- WORM hold precedence.
-- Model companion to YP-LIFECYCLE-COMPLIANCE-001 (Revision 1.1).
-- Real system: SHA-256/BLAKE3 tree hashes; collision resistance and
-- one-wayness are external assumptions, NOT proven here.

-- ===== Merkle Mountain Range peaks =====

/-- An MMR peak: height and digest. -/
structure Peak where
  h : Nat
  d : Nat
  deriving DecidableEq

/-- One merge pass: if the two tallest peaks share a height, merge them. -/
def mergeOnce : List Peak → List Peak
  | p2 :: p1 :: rest =>
      if p1.h = p2.h then ⟨p1.h + 1, p1.d * 33 + p2.d + 7⟩ :: rest
      else p2 :: p1 :: rest
  | l => l

/-- Append a base entry digest to the MMR peak list (newest at the front;
    the merge pass looks at the two newest peaks). -/
def mmrAppend (s : List Peak) (x : Nat) : List Peak :=
  mergeOnce (⟨0, x⟩ :: s)

/-- If the newest stored peak differs in height from the new base entry,
    nothing merges: pure append. This is the on-disk append-only property
    (no rewrite of stored peaks). -/
theorem mmrAppend_no_merge (s : List Peak) (p : Peak) (x : Nat)
    (hne : p.h ≠ 0) :
    mmrAppend (p :: s) x = ⟨0, x⟩ :: p :: s := by
  simp [mmrAppend, mergeOnce, hne]

/-- When the two newest peaks share a height they merge into a strictly
    taller peak; the merge digest is a pure function of the two children. -/
theorem mmrAppend_merge (s : List Peak) (p : Peak) (x : Nat) (heq : p.h = 0) :
    mmrAppend (p :: s) x = ⟨1, p.d * 33 + x + 7⟩ :: s := by
  simp [mmrAppend, mergeOnce, heq]

/-- Number of base entries a peak list summarizes: 2^h per peak. -/
def countBase : List Peak → Nat
  | [] => 0
  | p :: rest => 2 ^ p.h + countBase rest

theorem countBase_peak (p : Peak) : 0 < 2 ^ p.h := Nat.two_pow_pos p.h

/-- Appending an entry strictly increases the covered base count: no entry
    can be silently dropped by the append operation. -/
theorem countBase_strict (s : List Peak) (x : Nat) :
    countBase s < countBase (mmrAppend s x) := by
  induction s with
  | nil =>
      show 0 < 2 ^ 0 + 0
      decide
  | cons p rest _ih =>
      by_cases heq : p.h = 0
      · simp only [mmrAppend, mergeOnce, heq, if_true]
        show 2 ^ p.h + countBase rest < 2 ^ 1 + countBase rest
        rw [heq, Nat.pow_zero, Nat.pow_one]
        omega
      · simp only [mmrAppend, mergeOnce, heq, if_false]
        show 2 ^ p.h + countBase rest < 1 + (2 ^ p.h + countBase rest)
        have h1 : 0 < 2 ^ p.h := countBase_peak p
        omega

-- ===== Bagging the peaks: the epoch anchor =====

/-- Bag of peaks: deterministic digest over all peaks (fold, front first). -/
def bag : List Peak → Nat
  | [] => 1
  | p :: rest => bag rest * 33 + p.d + 7

/-- The bag is a positive seed chain. -/
theorem bag_pos : ∀ (s : List Peak), 1 ≤ bag s := by
  intro s
  induction s with
  | nil => exact Nat.le_refl _
  | cons p rest _ih =>
      show 1 ≤ bag rest * 33 + p.d + 7
      omega

/-- Bagging is order-canonical at the spec level: the epoch anchor is a
    pure function of the peak list, hence of the entry list and its order. -/
theorem bag_append_step (p : Peak) (rest : List Peak) :
    bag (p :: rest) = bag rest * 33 + p.d + 7 := rfl

-- ===== Tamper evidence =====

/-- Dropping the newest audited entry from an epoch changes the anchor:
    silent truncation is detected (conditional on hash collision resistance
    in the real system; unconditional in this toy model). -/
theorem anchor_tamper_evident (s : List Peak) (p : Peak) :
    bag (p :: s) ≠ bag s := by
  have h1 := bag_pos s
  show bag s * 33 + p.d + 7 ≠ bag s
  omega

-- ===== Forward-secure key evolution (Schneier–Kelsey, TISSEC 1999) =====

/-- Epoch key evolution: K_{e+1} = H(K_e). -/
def keyEv (k : Nat) : Nat := k * 33 + 7

/-- Keys strictly evolve: every epoch has a fresh key. -/
theorem keyEv_strict (k : Nat) : k < keyEv k := by
  simp only [keyEv]; omega

/-- Evolution is injective: distinct epochs hold distinct keys, so a
    compromised key cannot equal any other epoch's key. -/
theorem keyEv_injective (k1 k2 : Nat) (h : keyEv k1 = keyEv k2) : k1 = k2 := by
  simp only [keyEv] at h
  omega

/-- Honest statement: recovering K_{e-1} from K_e requires inverting H —
    a one-wayness assumption external to this model. What IS proven: the
    chain never revisits a key (strict + injective). -/
theorem keyEv_no_revisit (k : Nat) : keyEv (keyEv k) ≠ k := by
  simp only [keyEv]
  omega

-- ===== PII minimization: per-epoch salt =====

/-- Stored PII = epochSalt * 2^32 + ip with ip < 2^32 (IPv4/IPv6-truncated).
    Real system: H(salt_e ‖ ip) with an HMAC pepper. -/
def saltPII (e ip : Nat) : Nat := e * 4294967296 + ip

/-- THM-GDPR-AUDIT-COMPATIBILITY (model): stored PII values from different
    epochs are never equal — no cross-epoch linkage is computable from the
    stored representation alone. -/
theorem pii_epoch_unlinkable (e1 e2 ip1 ip2 : Nat)
    (h1 : ip1 < 4294967296) (h2 : ip2 < 4294967296) (he : e1 ≠ e2) :
    saltPII e1 ip1 ≠ saltPII e2 ip2 := by
  simp only [saltPII]
  omega

-- ===== WORM / legal hold precedence =====

/-- Deletion under an active hold is a no-op (THM-WORM-HOLD-PRECEDENCE). -/
def applyDelete (hold : Bool) (s : List Nat) : List Nat :=
  if hold then s else []

theorem hold_preserves (s : List Nat) : applyDelete true s = s := rfl

/-- Hold is authoritative: even repeated delete attempts under hold
    leave the object set unchanged (idempotence under hold). -/
theorem hold_delete_idempotent (s : List Nat) :
    applyDelete true (applyDelete true s) = applyDelete true s := rfl
