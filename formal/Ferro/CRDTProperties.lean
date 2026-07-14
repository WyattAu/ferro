-- CRDT (Conflict-free Replicated Data Type) properties
-- Models a G-Counter CRDT for distributed counters

/-- A G-Counter is a grow-only counter represented as a map from node IDs to Nat -/
structure GCounter where
  counts : Nat → Nat

/-- Two GCounters are equal when their counts functions agree on all inputs -/
theorem gcounter_eq_of_counts_eq {a b : GCounter} (h : a.counts = b.counts) :
  a = b := by
  rcases a with ⟨fa⟩
  rcases b with ⟨fb⟩
  simp at h
  rw [h]

/-- Empty GCounter with all zero counts -/
def GCounter.empty : GCounter where
  counts := fun _ => 0

/-- Get count for a specific node -/
def GCounter.get (gc : GCounter) (node : Nat) : Nat :=
  gc.counts node

/-- Increment counter for a specific node -/
def GCounter.increment (gc : GCounter) (node : Nat) : GCounter :=
  { gc with counts := fun n => if n == node then gc.counts n + 1 else gc.counts n }

/-- Merge two G-Counters by taking the max of each node's count -/
def GCounter.merge (a b : GCounter) : GCounter :=
  { counts := fun n => max (a.counts n) (b.counts n) }

/-- Total count for first n nodes -/
def GCounter.totalUpTo (gc : GCounter) (n : Nat) : Nat :=
  match n with
  | 0 => 0
  | n + 1 => gc.counts n + gc.totalUpTo n

/-- Merge is commutative: merge(a, b) = merge(b, a) -/
theorem merge_commutative (a b : GCounter) :
  GCounter.merge a b = GCounter.merge b a := by
  apply gcounter_eq_of_counts_eq
  funext n
  simp [GCounter.merge, Nat.max_comm]

/-- Merge is idempotent: merge(a, a) = a -/
theorem merge_idempotent (a : GCounter) :
  GCounter.merge a a = a := by
  apply gcounter_eq_of_counts_eq
  funext n
  simp [GCounter.merge, Nat.max_self]

/-- Merge is associative: merge(merge(a, b), c) = merge(a, merge(b, c)) -/
theorem merge_associative (a b c : GCounter) :
  GCounter.merge (GCounter.merge a b) c = GCounter.merge a (GCounter.merge b c) := by
  apply gcounter_eq_of_counts_eq
  funext n
  simp [GCounter.merge, Nat.max_assoc]

/-- Merge preserves non-negative counts (trivial for Nat) -/
theorem merge_preserves_nonneg (a b : GCounter) (n : Nat) :
  (GCounter.merge a b).counts n ≥ 0 := by
  omega

/-- Increment increases count for the specific node -/
theorem increment_increases_count (gc : GCounter) (node : Nat) :
  (gc.increment node).counts node = gc.counts node + 1 := by
  simp [GCounter.increment]

/-- Increment does not affect other nodes -/
theorem increment_preserves_others (gc : GCounter) (node other : Nat) (h : other ≠ node) :
  (gc.increment node).counts other = gc.counts other := by
  simp [GCounter.increment]
  omega

/-- Empty merge with any counter is identity (left) -/
theorem merge_empty_left (a : GCounter) :
  GCounter.merge GCounter.empty a = a := by
  apply gcounter_eq_of_counts_eq
  funext n
  simp [GCounter.merge, GCounter.empty, Nat.zero_max]

/-- Merge empty right is identity -/
theorem merge_empty_right (a : GCounter) :
  GCounter.merge a GCounter.empty = a := by
  apply gcounter_eq_of_counts_eq
  funext n
  simp [GCounter.merge, GCounter.empty, Nat.max_zero]

/-- Increment does not affect different nodes -/
theorem increment_counts_diff_node (gc : GCounter) (node1 node2 : Nat) (h : node1 ≠ node2) :
  (gc.increment node1).counts node2 = gc.counts node2 := by
  simp [GCounter.increment]
  omega

/-- Merge get symmetric -/
theorem merge_get_symmetric (a b : GCounter) (n : Nat) :
  GCounter.get (GCounter.merge a b) n = max (GCounter.get a n) (GCounter.get b n) := by
  simp [GCounter.get, GCounter.merge]

/-- Get empty is zero -/
theorem get_empty (node : Nat) :
  GCounter.get GCounter.empty node = 0 := by
  simp [GCounter.get, GCounter.empty]
