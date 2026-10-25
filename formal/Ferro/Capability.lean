-- Mesh capability authorization: attenuated delegation chains (UCAN-shaped)
-- anchored at the tenant boundary. Closes the DC-MESH-007 design gap.
-- Model companion to YP-MESH-TRANSPORT-001 (Revision 1.1).
-- Reference: UCAN Specification v1.0.0 (ucan-wg/spec, fetched 2026-10-14);
-- command ordering by /-segment prefix; attenuation-only delegation;
-- chain validity interval = [max nbf, min exp] over the chain.
-- Real system: Ed25519 signatures over DAG-CBOR; signature verification
-- and one-wayness of hashes are external assumptions.

/-- A command is a list of path segments (UCAN `/ferro/block/read`
    becomes [ferro, block, read]; `[]` is "top", proves everything). -/
abbrev Cmd := List Nat

/-- A shorter command proves a longer one with the same prefix:
    `a` proves `b` iff `b` is `a` plus a suffix. -/
def cmdProves (a b : Cmd) : Prop := ∃ sfx, a ++ sfx = b

theorem cmd_proves_refl (a : Cmd) : cmdProves a a :=
  ⟨[], by simp⟩

theorem cmd_proves_antisym (a b : Cmd) (h1 : cmdProves a b)
    (h2 : cmdProves b a) : a = b := by
  obtain ⟨s, hs⟩ := h1
  obtain ⟨t, ht⟩ := h2
  have hl : (a ++ s).length = b.length := congrArg List.length hs
  rw [List.length_append] at hl
  have hl2 : (b ++ t).length = a.length := congrArg List.length ht
  rw [List.length_append] at hl2
  have hz : s.length = 0 := by omega
  have : s = [] := by
    cases s with
    | nil => rfl
    | cons x ss => simp at hz
  rw [this, List.append_nil] at hs
  exact hs

theorem cmd_proves_trans (a b c : Cmd) (h1 : cmdProves a b)
    (h2 : cmdProves b c) : cmdProves a c := by
  obtain ⟨s, hs⟩ := h1
  obtain ⟨t, ht⟩ := h2
  exact ⟨s ++ t, by rw [← List.append_assoc, hs, ht]⟩

/-- "Top" (`[]`, the empty command) proves every command — the UCAN `/`
    super-capability. It is the ONLY command with this property, hence
    MUST be delegated sparingly (UCAN §"/" AKA "Top"). -/
theorem top_proves_all (b : Cmd) : cmdProves [] b :=
  ⟨b, by simp [List.append_nil]⟩

/-- A capability: (subject, command) with validity interval [nbf, exp]. -/
structure Cap where
  subject : Nat
  cmd : Cmd
  nbf : Nat
  exp : Nat

/-- `super` attenuates to `sub`: same subject, narrower-or-equal command,
    validity interval nested (UCAN: each delegation MUST restate or
    attenuate). -/
def attenuates (super sub : Cap) : Prop :=
  super.subject = sub.subject ∧ cmdProves super.cmd sub.cmd
    ∧ super.nbf ≤ sub.nbf ∧ sub.exp ≤ super.exp

theorem attenuates_refl (c : Cap) : attenuates c c :=
  ⟨rfl, cmd_proves_refl _, Nat.le_refl _, Nat.le_refl _⟩

/-- THM-CAP-CHAIN-SOUNDNESS: attenuation is transitive — a delegation
    chain's leaf authority is derivable from its root authority. This is
    the property that makes the whole chain checkable by verifying only
    adjacent links. -/
theorem attenuates_trans (r m l : Cap) (h1 : attenuates r m)
    (h2 : attenuates m l) : attenuates r l := by
  obtain ⟨hs1, hc1, hn1, he1⟩ := h1
  obtain ⟨hs2, hc2, hn2, he2⟩ := h2
  exact ⟨hs1 ▸ hs2, cmd_proves_trans _ _ _ hc1 hc2,
    Nat.le_trans hn1 hn2, Nat.le_trans he2 he1⟩

/-- THM-CAP-NO-AMPLIFICATION: any invocation command `c` authorized by a
    leaf was already authorized by the chain root — delegation can only
    narrow authority, never widen it. -/
theorem no_amplification (r l : Cap) (c : Cmd) (h : attenuates r l)
    (hinv : cmdProves l.cmd c) : cmdProves r.cmd c :=
  cmd_proves_trans _ _ _ h.2.1 hinv

/-- THM-CAP-TENANT-CONTAINMENT: attenuation preserves the subject — a
    capability chain rooted at tenant T can never authorize an operation
    against tenant T'. Cross-tenant mesh fetch (DC-MESH-007) is therefore
    structurally impossible, not merely policy-forbidden. -/
theorem tenant_containment (r l : Cap) (h : attenuates r l) :
    l.subject = r.subject := h.1.symm

/-- THM-CAP-INTERVAL-CONTAINMENT: the leaf's validity interval is contained
    in the root's — a chain is never valid outside its weakest link. -/
theorem interval_containment (r l : Cap) (h : attenuates r l)
    (t : Nat) (hvalid : l.nbf ≤ t ∧ t ≤ l.exp) :
    r.nbf ≤ t ∧ t ≤ r.exp :=
  ⟨Nat.le_trans h.2.2.1 hvalid.1, Nat.le_trans hvalid.2 h.2.2.2⟩

/-- Zero-time delegation (nbf = exp) is valid only at that instant —
    the degenerate interval is still sound under containment. -/
theorem degenerate_interval (r l : Cap) (h : attenuates r l)
    (heq : l.nbf = l.exp) (t : Nat) (hvalid : l.nbf ≤ t ∧ t ≤ l.exp) :
    t = l.nbf := by
  have := hvalid
  omega
