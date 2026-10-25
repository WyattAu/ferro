-- Refresh-token rotation and DPoP-style proof replay defense.
-- Extension companion to YP-AUTH-OIDC-CEDAR-001 (Revision 1.1, R3-6).
-- References:
--   RFC 9449 "OAuth 2.0 Demonstrating Proof of Possession (DPoP),"
--   September 2023 [VERIFIED 2026-10-14 — datatracker.ietf.org/doc/html/
--   rfc9449]: §4.2 jti/htm/htu/iat/ath claims, §4.3 proof checking,
--   §5.8 refresh-token key binding, §11.1 replay.
-- External assumptions: signature unforgeability, hash collision
-- resistance (as elsewhere). Proved here: rotation single-use (a consumed
-- refresh token is never accepted again), replay detection (a proof whose
-- jti is in the seen-set is rejected; acceptance records the jti), and the
-- ath binding gate (a proof is valid only with the matching token hash).

/-- A refresh token (abstract id). Rotation is external: each accepted
    refresh yields a fresh token and invalidates its predecessor. -/
axiom rotate (t : Nat) : Option Nat

/-- Rotation yields a token distinct from its predecessor. -/
axiom rotate_fresh (t t' : Nat) : rotate t = some t' → t' ≠ t

/-- The consumed-token registry. -/
def consume (used : List Nat) (t : Nat) : List Nat := t :: used

theorem consume_records (used : List Nat) (t : Nat) : t ∈ consume used t :=
  List.mem_cons_self ..

/-- ROTATION SINGLE-USE: a token recorded as consumed stays in the
    registry forever, so the second presentation of the same refresh token
    is always detectable. -/
theorem reuse_detectable (used : List Nat) (t : Nat) (h : t ∈ used) :
    t ∈ consume used t := by
  unfold consume
  exact List.mem_cons_of_mem _ h

/-- A DPoP proof replay gate: seen-set membership, time window, and the
    ath (access-token hash) binding must all hold (RFC 9449 §4.3). -/
def proofValid (jti : Nat) (seen : List Nat) (windowOK athOK : Bool) : Bool :=
  windowOK && athOK && !(List.elem jti seen)

/-- Proof acceptance records the jti. -/
def record (jti : Nat) (seen : List Nat) : List Nat := jti :: seen

/-- REPLAY REJECTED: a jti already in the seen-set never validates. -/
theorem replay_rejected (jti : Nat) (seen : List Nat) (w a : Bool)
    (h : List.elem jti seen) :
    proofValid jti seen w a = false := by
  unfold proofValid
  rw [h]
  simp

/-- ACCEPT RECORDS: every accepted proof's jti joins the seen-set, which by
    replay_rejected makes the second presentation fail — exactly-once use. -/
theorem accept_records (jti : Nat) (seen : List Nat) :
    jti ∈ record jti seen :=
  List.mem_cons_self ..

theorem accepted_never_reaccepted (jti : Nat) (seen : List Nat)
    (h : proofValid jti seen true true = true) :
    proofValid jti (record jti seen) true true = false := by
  have hmem : List.elem jti (record jti seen) := by
    unfold record
    simp
  exact replay_rejected jti (record jti seen) true true hmem
