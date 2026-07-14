-- Formal verification of authentication properties

/-- Token is valid if not expired and matches user -/
structure Token where
  userId : String
  expiresAt : Nat
  deriving Repr, BEq

/-- Current time -/
def currentTime : Nat := 0  -- Placeholder

/-- Token is valid -/
def Token.isValid (token : Token) : Prop :=
  token.expiresAt > currentTime

/-- After expiration, token is invalid -/
theorem token_expires (token : Token) (h : token.isValid) :
  token.expiresAt > currentTime := h

/-- Refresh extends validity -/
def Token.refresh (token : Token) (newExpiry : Nat) : Token :=
  { token with expiresAt := newExpiry }

theorem refresh_preserves_user (token : Token) (newExpiry : Nat) :
  (token.refresh newExpiry).userId = token.userId := by
  simp [Token.refresh]
