-- Authentication token refresh properties
-- Models token-based authentication with refresh mechanics

/-- Authentication token with user ID and expiration -/
structure AuthToken where
  userId : String
  expiresAt : Nat
  issuedAt : Nat

/-- Current time (placeholder for real implementation) -/
def AuthToken.currentTime : Nat := 0

/-- Token is valid if not expired and issued before expiration -/
def AuthToken.isValid (token : AuthToken) : Prop :=
  token.expiresAt > AuthToken.currentTime ∧ token.issuedAt < token.expiresAt

/-- Refresh duration (e.g., 1 hour in seconds) -/
def AuthToken.refreshDuration : Nat := 3600

/-- Refresh a token: extend expiration while preserving user ID -/
def AuthToken.refresh (token : AuthToken) (now : Nat) : AuthToken :=
  { token with
    expiresAt := now + AuthToken.refreshDuration
    issuedAt := now
  }

/-- Token equality is determined by all fields -/
theorem token_eq_of_fields_eq {a b : AuthToken}
    (hu : a.userId = b.userId) (he : a.expiresAt = b.expiresAt) (hi : a.issuedAt = b.issuedAt) :
  a = b := by
  cases a with | mk ua ea ia =>
  cases b with | mk ub eb ib =>
  simp only at hu he hi
  rw [hu, he, hi]

/-- Refresh preserves user ID -/
theorem refresh_preserves_userId (token : AuthToken) (now : Nat) :
  (token.refresh now).userId = token.userId := by
  simp [AuthToken.refresh]

/-- Refresh increases expiration when now ≥ expiresAt -/
theorem refresh_increases_expiration (token : AuthToken) (now : Nat)
    (h : now ≥ token.expiresAt) :
  (token.refresh now).expiresAt > token.expiresAt := by
  simp [AuthToken.refresh, AuthToken.refreshDuration]
  omega

/-- Refresh produces valid token when called with a sufficiently large now -/
theorem refresh_valid_when_now_large (token : AuthToken) (now : Nat)
    (h1 : now ≥ token.expiresAt) :
  AuthToken.refreshDuration > 0 → (token.refresh now).isValid := by
  intro hdur
  constructor
  · simp [AuthToken.refresh, AuthToken.currentTime]
    omega
  · simp [AuthToken.refresh]
    omega

/-- Refresh is idempotent for same timestamp -/
theorem refresh_idempotent (token : AuthToken) (now : Nat) :
  (token.refresh now).refresh now = token.refresh now := by
  simp [AuthToken.refresh]

/-- Refresh always produces a token with positive duration (issuedAt < expiresAt) -/
theorem refresh_positive_duration (token : AuthToken) (now : Nat) :
  (token.refresh now).expiresAt > (token.refresh now).issuedAt := by
  simp [AuthToken.refresh, AuthToken.refreshDuration]

/-- Two tokens with same fields are equal -/
theorem token_eq_iff_fields_eq (t1 t2 : AuthToken) :
  t1 = t2 ↔ t1.userId = t2.userId ∧ t1.expiresAt = t2.expiresAt ∧ t1.issuedAt = t2.issuedAt := by
  constructor
  · intro h; subst h; exact ⟨rfl, rfl, rfl⟩
  · intro ⟨hu, he, hi⟩
    exact token_eq_of_fields_eq hu he hi

/-- Refresh duration is non-zero (ensures token extension) -/
theorem refreshDuration_positive : AuthToken.refreshDuration > 0 := by
  simp [AuthToken.refreshDuration]

/-- Refresh updates issuedAt to now -/
theorem refresh_updates_issuedAt (token : AuthToken) (now : Nat) :
  (token.refresh now).issuedAt = now := by
  simp [AuthToken.refresh]

/-- Refresh updates expiresAt to now + refreshDuration -/
theorem refresh_updates_expiresAt (token : AuthToken) (now : Nat) :
  (token.refresh now).expiresAt = now + AuthToken.refreshDuration := by
  simp [AuthToken.refresh]
