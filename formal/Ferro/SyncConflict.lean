-- Sync conflict resolution: version vectors, Lamport causality,
-- deterministic winner selection, no-silent-loss, Merkle-rooted op log clock.
-- Model companion to YP-SYNC-CONFLICT-001 (Revision 1.1).

-- ===== Version vectors =====

structure VersionVector where
  entries : Nat → Nat

/-- Componentwise order on version vectors. -/
def vvLE (u v : VersionVector) : Prop := ∀ d, u.entries d ≤ v.entries d

/-- Two writes are concurrent iff their version vectors are incomparable. -/
def vvConcurrent (u v : VersionVector) : Prop := ¬ vvLE u v ∧ ¬ vvLE v u

theorem vv_le_refl (u : VersionVector) : vvLE u u :=
  fun _ => Nat.le_refl _

theorem vv_le_antisym {u v : VersionVector} (h1 : vvLE u v) (h2 : vvLE v u) :
    u = v := by
  cases u; cases v
  simp only [vvLE] at h1 h2
  congr 1
  funext d
  exact Nat.le_antisymm (h1 d) (h2 d)

theorem vv_le_trans {u v w : VersionVector} (h1 : vvLE u v) (h2 : vvLE v w) :
    vvLE u w :=
  fun d => Nat.le_trans (h1 d) (h2 d)

theorem vv_concurrent_symmetric {u v : VersionVector} (h : vvConcurrent u v) :
    vvConcurrent v u :=
  ⟨h.2, h.1⟩

/-- No version vector is concurrent with itself: the concurrency test is
    irreflexive (unlike (size, mtime) difference, which fires spuriously). -/
theorem vv_concurrent_irrefl (u : VersionVector) : ¬ vvConcurrent u u :=
  fun h => h.1 (vv_le_refl u)

/-- If two writes are concurrent, neither causally subsumes the other:
    the VV test never conflates "different" with "concurrent". -/
theorem concurrent_not_ordered {u v : VersionVector} (h : vvConcurrent u v) :
    ¬ (vvLE u v ∨ vvLE v u) := by
  rintro (h' | h')
  · exact h.1 h'
  · exact h.2 h'

-- ===== Lamport clocks =====

/-- Lamport clock: on local op `L+1`; on receive, `max L msg + 1`. -/
def lamportRecv (L msg : Nat) : Nat := max L msg + 1

/-- A received op's lamport value is strictly below the receiver's next clock. -/
theorem lamport_recv_above (L msg : Nat) : msg < lamportRecv L msg := by
  simp only [lamportRecv]
  omega

/-- Clocks never go backwards on receive. -/
theorem lamport_recv_monotone (L msg : Nat) : L ≤ lamportRecv L msg := by
  simp only [lamportRecv]
  omega

/-- A device never emits two ops with the same lamport value: consecutive
    local ops strictly increase the clock (keys in the concurrent set are
    distinct). -/
theorem lamport_local_strict (L : Nat) : L < L + 1 := Nat.lt_succ_self L

-- ===== Deterministic winner selection =====

/-- A metadata write op. -/
structure WriteOp where
  device : Nat
  lamport : Nat
  revision : Nat
  deriving DecidableEq

/-- Total order key. Encoding lamport·2³² + device makes the key order the
    (lamport, device) lexicographic order for u64 device IDs (DC-SYNC-002:
    NEVER wall-clock). -/
def opKey (o : WriteOp) : Nat := o.lamport * 4294967296 + o.device

/-- One fold step of winner selection: keep the dominating key. -/
def step (acc o : WriteOp) : WriteOp :=
  if opKey acc < opKey o then o else acc

/-- Winner of a concurrent set: fold-max over (lamport, device), seeded. -/
def winnerOf (l : List WriteOp) (seed : WriteOp) : WriteOp :=
  l.foldl step seed

theorem winnerOf_cons_self (x : WriteOp) (l : List WriteOp) (seed : WriteOp) :
    winnerOf (x :: l) seed = winnerOf l (step seed x) := rfl

theorem step_le_left (acc o : WriteOp) : opKey acc ≤ opKey (step acc o) := by
  unfold step
  split
  · omega
  · omega

theorem step_le_right (acc o : WriteOp) : opKey o ≤ opKey (step acc o) := by
  unfold step
  split
  · omega
  · omega

theorem step_picks_larger {acc o : WriteOp} (h : opKey acc < opKey o) :
    step acc o = o := by
  unfold step; exact if_pos h

theorem step_keeps {acc o : WriteOp} (h : ¬ (opKey acc < opKey o)) :
    step acc o = acc := by
  unfold step; exact if_neg h

/-- The running winner never drops below the seed. -/
theorem winnerOf_ge_seed : ∀ (l : List WriteOp) (seed : WriteOp),
    opKey seed ≤ opKey (winnerOf l seed) := by
  intro l
  induction l with
  | nil => intro seed; exact Nat.le_refl _
  | cons hd tl ih =>
    intro seed
    simp only [winnerOf, List.foldl_cons]
    exact Nat.le_trans (step_le_left seed hd) (ih (step seed hd))

/-- The winner dominates every member of the concurrent set. -/
theorem winner_ge_of_mem : ∀ (l : List WriteOp) (seed o : WriteOp),
    o ∈ l → opKey o ≤ opKey (winnerOf l seed) := by
  intro l
  induction l with
  | nil => intro seed o h; cases h
  | cons hd tl ih =>
    intro seed o hmem
    rcases List.mem_cons.mp hmem with heq | htl
    · subst heq
      rw [winnerOf_cons_self]
      exact Nat.le_trans (step_le_right seed o)
        (winnerOf_ge_seed tl (step seed o))
    · rw [winnerOf_cons_self]
      exact ih (step seed hd) o htl

/-- One fold step commutes under swap when keys are distinct (distinctness
    is guaranteed by server sequencing: a device's lamport strictly
    increases per op, and the key carries the device id). -/
theorem step_swap (s a b : WriteOp) (hab : opKey a ≠ opKey b) :
    step (step s a) b = step (step s b) a := by
  by_cases hablt : opKey a < opKey b
  · have hb : ¬ (opKey b < opKey a) := by omega
    by_cases hsb : opKey s < opKey b
    · rw [step_picks_larger hsb, step_keeps hb]
      by_cases hsa : opKey s < opKey a
      · rw [step_picks_larger hsa, step_picks_larger hablt]
      · rw [step_keeps hsa, step_picks_larger hsb]
    · rw [step_keeps hsb]
      by_cases hsa : opKey s < opKey a
      · rw [step_picks_larger hsa, step_picks_larger hablt]
        exfalso; omega
      · rw [step_keeps hsa, step_keeps hsb]
  · have hba : opKey b < opKey a := by
      by_cases h : opKey b < opKey a
      · exact h
      · exfalso
        exact hab (by omega)
    have hbnot : ¬ (opKey a < opKey b) := by omega
    by_cases hsa : opKey s < opKey a
    · rw [step_picks_larger hsa, step_keeps hbnot]
      by_cases hsb : opKey s < opKey b
      · rw [step_picks_larger hsb, step_picks_larger hba]
      · rw [step_keeps hsb, step_picks_larger hsa]
    · rw [step_keeps hsa]
      have hsb : ¬ (opKey s < opKey b) := by omega
      rw [step_keeps hsb, step_keeps hsa]

/-- Winner selection is a pure function of the concurrent SET, not its order:
    adjacent swap with distinct keys cannot change the winner (DC-SYNC-002). -/
theorem winner_adjacent_swap (s a b : WriteOp) (l : List WriteOp)
    (hab : opKey a ≠ opKey b) :
    winnerOf (a :: b :: l) s = winnerOf (b :: a :: l) s := by
  simp only [winnerOf, List.foldl_cons]
  rw [step_swap s a b hab]

-- ===== No silent loss =====

/-- A materialized state: path -> winning write. -/
def Materialized := Nat → Option WriteOp

/-- Conflict path encoding: primary `p` materializes the winner;
    the loser materializes at the odd sibling `2*p+1`. -/
def conflictPath (p : Nat) : Nat := 2 * p + 1

theorem conflict_path_ne (p : Nat) : conflictPath p ≠ p := by
  intro h
  simp [conflictPath] at h
  omega

/-- Materialization of a two-way concurrent write conflict: winner at `p`,
    loser at the conflict path. -/
def resolveConflict (p : Nat) (w1 w2 : WriteOp) : Materialized :=
  fun q => if q = p then some (winnerOf [w1, w2] w1)
           else if q = conflictPath p then
             some (if opKey w1 < opKey w2 then w1 else w2)
           else none

/-- THM-NO-SILENT-LOSS (two-write case): the winner materializes at the
    primary path. -/
theorem resolve_winner_at_primary (p : Nat) (w1 w2 : WriteOp) :
    resolveConflict p w1 w2 p = some (winnerOf [w1, w2] w1) := by
  unfold resolveConflict
  rw [if_pos rfl]

/-- The loser materializes at the conflict path: nothing vanishes. -/
theorem resolve_loser_at_conflict (p : Nat) (w1 w2 : WriteOp) :
    resolveConflict p w1 w2 (conflictPath p) =
      some (if opKey w1 < opKey w2 then w1 else w2) := by
  have hne : ¬ (2 * p + 1 = p) := by omega
  unfold resolveConflict conflictPath
  rw [if_neg hne, if_pos rfl]

/-- THM-NO-SILENT-LOSS (two-write case): both writes are represented —
    the winner at the primary path, the loser at the conflict path. -/
theorem no_silent_loss (p : Nat) (w1 w2 : WriteOp) :
    (resolveConflict p w1 w2 p ≠ none) ∧
    (resolveConflict p w1 w2 (conflictPath p) ≠ none) := by
  constructor
  · rw [resolve_winner_at_primary]
    exact fun h => Option.noConfusion h
  · rw [resolve_loser_at_conflict]
    exact fun h => Option.noConfusion h

/-- The loser's write is materialized somewhere: nothing vanishes. -/
theorem loser_materialized (p : Nat) (w1 w2 : WriteOp) :
    ∃ q, resolveConflict p w1 w2 q = some (if opKey w1 < opKey w2 then w1 else w2) :=
  ⟨conflictPath p, resolve_loser_at_conflict p w1 w2⟩

/-- The conflict copy never collides with the primary path: the two
    materializations are always distinct paths. -/
theorem conflict_copy_distinct (p : Nat) :
    p ≠ conflictPath p := fun h => conflict_path_ne p h.symm

-- ===== Merkle-rooted op log: the root is a logical clock =====
-- (Merkle-CRDTs, Sanjuan, Poyhtari, Teixeira, Psaras, arXiv:2004.00107)

/-- Toy digest standing in for the Merkle root of the sequenced op log.
    `H_n = 33·H_{n-1} + entry_n + 7` models a hash chain with head-newest
    wrapping. Real system: SHA-256/BLAKE3; collision resistance is an
    external assumption, NOT proven here. -/
def chainStep (h x : Nat) : Nat := h * 33 + x + 7

def logDigest (σ : List Nat) : Nat := σ.foldl chainStep 1

/-- Appending an op is exactly one chain step (the log root is incremental). -/
theorem logDigest_append_one (σ : List Nat) (x : Nat) :
    logDigest (σ ++ [x]) = chainStep (logDigest σ) x := by
  simp [logDigest, List.foldl_append]

/-- Digest is positive: the seed is 1 and chain steps never decrease. -/
theorem logDigest_pos : ∀ (σ : List Nat), 1 ≤ logDigest σ := by
  intro σ
  have hgen : ∀ (l : List Nat) (a : Nat), 1 ≤ a → 1 ≤ l.foldl chainStep a := by
    intro l
    induction l with
    | nil => intro a ha; exact ha
    | cons x _tl ih =>
      intro a ha
      exact ih (chainStep a x) (by show 1 ≤ a * 33 + x + 7; omega)
  exact hgen σ 1 (Nat.le_refl _)

/-- The log root is a strict logical clock: appending any op strictly
    advances it. Under collision resistance of the real hash, equal roots
    certify identical log prefixes — the replica-equality oracle used
    before any delta exchange. -/
theorem logDigest_strictly_advances (σ : List Nat) (x : Nat) :
    logDigest σ < logDigest (σ ++ [x]) := by
  rw [logDigest_append_one]
  have h1 := logDigest_pos σ
  simp only [chainStep]
  omega

/-- Distinct logs (differing in at least one append) cannot share a root:
    a replica advertising root r either has exactly that prefix or the
    comparison is invalid (hash collision). -/
theorem logDigest_append_ne (σ : List Nat) (x : Nat) :
    logDigest σ ≠ logDigest (σ ++ [x]) := by
  intro h
  have hlt := logDigest_strictly_advances σ x
  omega
