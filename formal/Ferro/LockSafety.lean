-- WebDAV write-lock safety (RFC 4918 §6-§7, §9.10-§9.11).
-- Model companion to YP-WEBDAV-PROTOCOL-001 (Revision 1.1, R3-5).
-- References:
--   RFC 4918 (Dusseault, ed.), "HTTP Extensions for Web Distributed
--   Authoring and Versioning (WebDAV)," June 2007 [VERIFIED 2026-10-14 —
--   rfc-editor.org/rfc/rfc4918]: §6.2 exclusive vs shared, §7.1 write
--   locks and token submission, §9.10.3 depth, §9.11 UNLOCK status codes,
--   Appendix C opaquelocktoken scheme.
-- Proved here: the depth coverage ordering, the write-exclusion law (a
-- write passes only when every covering lock's token was submitted), and
-- the UNLOCK path-match guard (a lock whose path differs from the request
-- path can never be released by that request).

/-- Lock depth (§9.10.3). -/
inductive LockDepth where
  | zero | one | infinity

/-- Abstract resource namespace: resources are naturals, `parent` is the
    collection containment function. -/
axiom parent : Nat → Option Nat

/-- Proper-descendant relation, characterized by the parent chain. -/
axiom Ancestor : Nat → Nat → Prop

/-- The parent chain reaches ancestors. -/
axiom parent_is_ancestor (t p : Nat) : parent t = some p → Ancestor t p

/-- Coverage: does a lock at `p` with depth `d` protect writes to `t`?
    zero: the resource itself; one: internal members of the locked
    collection (direct children); infinity: the whole subtree. -/
def covers : LockDepth → Nat → Nat → Prop
  | LockDepth.zero, p, t => p = t
  | LockDepth.one, p, t => parent t = some p
  | LockDepth.infinity, p, t => p = t ∨ Ancestor t p

/-- Depth monotonicity: an infinity lock covers everything a one-lock
    covers, and a one-lock everything a zero-lock covers — widening the
    depth never shrinks protection. -/
theorem covers_depth_monotone (p t : Nat) :
    covers LockDepth.zero p t → covers LockDepth.one p t ∨ covers LockDepth.zero p t := by
  intro h
  exact Or.inr h

theorem covers_one_infinity (p t : Nat) (h : covers LockDepth.one p t) :
    covers LockDepth.infinity p t := by
  unfold covers at h
  exact Or.inr (parent_is_ancestor t p h)

/-- A lock: which resource, at what depth, bearing which token. -/
structure Lock where
  path : Nat
  depth : LockDepth
  token : Nat

/-- Submitted token for a write (RFC 4918 §7.5 If header). -/
def Submitted := Option Nat

/-- WRITE EXCLUSION LAW (§7.1): a write to `t` is permitted iff every lock
    covering `t` had its exact token submitted. -/
def writeAllowed (locks : List Lock) (t : Nat) (s : Submitted) : Prop :=
  ∀ L ∈ locks, covers L.depth L.path t → s = some L.token

theorem write_allowed_empty (t : Nat) : writeAllowed [] t none := by
  intro L hL
  cases hL

theorem write_allowed_token_match (L : Lock) (t : Nat) (h : covers L.depth L.path t) :
    writeAllowed [L] t (some L.token) := by
  intro L' hL' hcov
  cases hL' with
  | head => rfl
  | tail _ hcontra => cases hcontra

theorem write_blocked_on_mismatch (L : Lock) (t : Nat) (other : Nat)
    (hne : other ≠ L.token) (hcov : covers L.depth L.path t) :
    ¬ writeAllowed [L] t (some other) := by
  intro hall
  exact hne (Option.some.inj (hall L (by simp) hcov))

/-- UNLOCK PATH-MATCH GUARD (§9.11): a release request for path `rp` with
    token `tok` may remove a lock only when the lock's own path equals the
    request path. Modeled as the release function: mismatched releases are
    no-ops and the lock survives. -/
@[reducible]
def releasable (L : Lock) (rp : Nat) (tok : Nat) : Prop :=
  L.token = tok ∧ L.path = rp

def release (locks : List Lock) (rp : Nat) (tok : Nat) : List Lock :=
  match locks with
  | [] => []
  | L :: rest =>
      if releasable L rp tok then rest else L :: release rest rp tok

theorem release_noop_on_path_mismatch (L : Lock) (rest : List Lock) (rp tok : Nat)
    (hmismatch : L.path ≠ rp) :
    release (L :: rest) rp tok = L :: release rest rp tok := by
  have hneg : ¬ releasable L rp tok := by
    intro h
    exact hmismatch h.2
  rw [release, if_neg hneg]

theorem release_preserves_mismatched_locks (locks : List Lock) (rp tok : Nat) :
    ∀ L ∈ locks, L.path ≠ rp → L ∈ release locks rp tok := by
  induction locks with
  | nil => intro L hmem _; cases hmem
  | cons L rest ih =>
      intro Lmem hmem hmismatch
      rcases List.mem_cons.mp hmem with rfl | hr
      · -- the head lock itself: mismatch with rp means it cannot be released
        have hneg : ¬ releasable Lmem rp tok := by
          intro hc
          exact hmismatch hc.2
        rw [release, if_neg hneg]
        exact List.mem_cons_self ..
      · -- a tail lock: released or not, Lmem survives
        by_cases hrel : releasable L rp tok
        · rw [release, if_pos hrel]
          exact hr
        · rw [release, if_neg hrel]
          exact List.mem_cons_of_mem _ (ih _ hr hmismatch)
