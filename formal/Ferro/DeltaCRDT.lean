-- Delta-state CRDTs (δ-CRDTs) and causal-stability garbage collection.
-- Extension companion to YP-SYNC-CONFLICT-001 (Revision 1.2, R3-4).
-- References:
--   Almeida, P. S., Shoker, A., Baquero, C. "Delta State Replicated Data
--   Types," JPDC 111:162-173, 2018; arXiv:1603.01529 [VERIFIED 2026-10-14].
-- Builds on Ferro/DeltaCounter (CRDTProperties.lean). Proved here: the delta
-- decomposition law (joining a delta mutator into a state equals joining
-- the full increment — the "small messages, state-based reliability"
-- guarantee), and the causal-stability GC contracts (only sub-watermark
-- entries are dropped; all above-watermark entries survive).

/-- A G-Counter: per-node counts. -/
structure DeltaCounter where
  counts : Nat → Nat

def gcGet (gc : DeltaCounter) (node : Nat) : Nat := gc.counts node

/-- State-based increment: replaces this node's count. -/
def gcIncrement (gc : DeltaCounter) (node : Nat) : DeltaCounter :=
  ⟨fun i => if i = node then gc.counts node + 1 else gc.counts i⟩

/-- Pointwise join (max) — the CRDT merge. -/
def gcMerge (a b : DeltaCounter) : DeltaCounter :=
  ⟨fun i => max (a.counts i) (b.counts i)⟩

/-- DELTA MUTATOR: the increment as a standalone delta-state — only the
    changed node carries a nonzero entry. Disseminating this instead of the
    full state is the δ-CRDT win (small messages, state-based reliability). -/
def gcDeltaIncrement (gc : DeltaCounter) (node : Nat) : DeltaCounter :=
  ⟨fun i => if i = node then gc.counts node + 1 else 0⟩

/-- DELTA DECOMPOSITION LAW: merging the delta mutator into any state `a`
    yields exactly the same result as merging the full incremented state.
    Small messages behave like full states. -/
theorem gc_delta_decomposition (a : DeltaCounter) (node : Nat) :
    gcMerge (gcDeltaIncrement a node) a = gcIncrement a node := by
  have h : ∀ i, (gcMerge (gcDeltaIncrement a node) a).counts i = (gcIncrement a node).counts i := by
    intro i
    unfold gcMerge gcDeltaIncrement gcIncrement
    by_cases hi : i = node
    · subst hi
      simp
      omega
    · simp [hi]
  cases a with
  | mk fa => exact congrArg DeltaCounter.mk (funext h)

/-- Entry in an op log: (node, version). -/
def LogEntry := Nat × Nat

/-- CAUSAL-STABILITY GC: drop logged ops at or below the stable watermark
    for their node. Ops above the watermark must be retained — replicas
    that have not yet observed them still need delivery. -/
def gcLog (log : List LogEntry) (stable : Nat → Nat) : List LogEntry :=
  log.filter (fun e => e.2 > stable e.1)

/-- GC CONTRACT 1: every dropped entry was at or below the watermark. -/
theorem gc_drops_only_stable (log : List LogEntry) (stable : Nat → Nat) :
    ∀ e ∈ log, e ∉ gcLog log stable → e.2 ≤ stable e.1 := by
  intro e he hdrop
  by_cases hab : e.2 > stable e.1
  · exfalso
    unfold gcLog at hdrop
    exact hdrop (List.mem_filter.mpr ⟨he, by simp [hab]⟩)
  · omega

/-- GC CONTRACT 2: every entry above the watermark survives the collection. -/
theorem gc_keeps_above_watermark (log : List LogEntry) (stable : Nat → Nat) (e : LogEntry)
    (hmem : e ∈ log) (habove : e.2 > stable e.1) :
    e ∈ gcLog log stable := by
  unfold gcLog
  exact List.mem_filter.mpr ⟨hmem, by simp [habove]⟩

/-- GC is idempotent: collecting an already-collected log changes nothing. -/
theorem gc_idempotent (log : List LogEntry) (stable : Nat → Nat) :
    gcLog (gcLog log stable) stable = gcLog log stable := by
  unfold gcLog
  rw [List.filter_filter]
  congr 1
  funext e
  simp
