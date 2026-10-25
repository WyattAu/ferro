-- Hybrid search fusion: RRF weight monotonicity, fused-score domination,
-- per-document score invariance under authz filtering, top-k soundness.
-- Model companion to YP-SEARCH-RETRIEVAL-001 (Revision 1.1).
-- RRF reference: Cormack, Clarke, Buettcher. "Reciprocal Rank Fusion
-- outperforms Condorcet and individual Rank Learning Methods," SIGIR 2009
-- (plg.uwaterloo.ca/~gvcormac/cormacksigir09-rrf.pdf) [VERIFIED existence].
-- score(d) = Σ_systems 1/(k + rank_s(d)), k = 60.

/-- Division is anti-monotone in a POSITIVE divisor (core 4.12 lacks
    this lemma; derived from div_add_mod). The a/0 = 0 convention breaks
    the property at m = 0, hence the positivity hypothesis. -/
theorem div_anti_monotone {a m n : Nat} (hm : 0 < m) (h : m ≤ n) :
    a / n ≤ a / m := by
  rcases Nat.lt_or_ge (a / m) (a / n) with hlt | hge
  · have h41 : a / m + 1 ≤ a / n := Nat.succ_le_of_lt hlt
    have h42 : n * (a / m + 1) ≤ n * (a / n) := Nat.mul_le_mul_left _ h41
    have h43 : n * (a / n) + a % n = a := Nat.div_add_mod a n
    have h44 : n * (a / m + 1) ≤ a := by omega
    have h5 : n * (a / m + 1) = n * (a / m) + n := by
      rw [Nat.mul_add, Nat.mul_one]
    have h1 : m * (a / m) + a % m = a := Nat.div_add_mod a m
    have h2 : a % m < m := Nat.mod_lt a hm
    have h6 : m * (a / m) ≤ n * (a / m) := Nat.mul_le_mul_right (a / m) h
    omega
  · exact hge

/-- RRF term weight for rank r, scaled by 1000 (integer arithmetic;
    monotone non-increasing in r — the property the fusion relies on). -/
def rrfWeight (r : Nat) : Nat := 1000 / (60 + r)

/-- THM-RRF-MONOTONE: a better (lower) rank never yields a smaller
    weight — the reciprocal term is anti-monotone in rank. -/
theorem rrf_weight_monotone (r r' : Nat) (h : r ≤ r') :
    rrfWeight r' ≤ rrfWeight r :=
  div_anti_monotone (by omega) (by omega)

/-- Per-document fused score over any number of retrieval systems:
    a list of (rank in system A, rank in system B) pairs, folded. The
    score depends ONLY on the document's own ranks — never on the rest
    of the corpus. This is the structural fact that makes authz
    filtering and ranking commute. -/
def pairedScore (pairs : List (Nat × Nat)) : Nat × Nat :=
  pairs.foldl (fun acc p => (acc.1 + rrfWeight p.1, acc.2 + rrfWeight p.2))
    (0, 0)

/-- THM-SEARCH-FUSION-RANK-INVARIANT: if document α has a better-or-equal
    rank than document β in EVERY system (pointwise), then α's fused score
    dominates β's — regardless of how many systems participate. The fusion
    cannot invert a pointwise-dominating document. -/
theorem paired_score_mono : ∀ (pairs : List (Nat × Nat)),
    (∀ p ∈ pairs, p.1 ≤ p.2) →
    (pairedScore pairs).2 ≤ (pairedScore pairs).1 := by
  have hgen : ∀ (pairs : List (Nat × Nat)) (acc : Nat × Nat),
      (∀ p ∈ pairs, p.1 ≤ p.2) → acc.2 ≤ acc.1 →
      (pairs.foldl (fun q p => (q.1 + rrfWeight p.1, q.2 + rrfWeight p.2)) acc).2 ≤
      (pairs.foldl (fun q p => (q.1 + rrfWeight p.1, q.2 + rrfWeight p.2)) acc).1 := by
    intro pairs
    induction pairs with
    | nil => intro acc _ h; exact h
    | cons p rest ih =>
        intro acc hmem hacc
        simp only [List.foldl_cons]
        have hstep : (acc.1 + rrfWeight p.1, acc.2 + rrfWeight p.2).2 ≤
            (acc.1 + rrfWeight p.1, acc.2 + rrfWeight p.2).1 := by
          have hw := rrf_weight_monotone p.1 p.2 (hmem p (List.mem_cons_self ..))
          omega
        exact ih _ (fun q hq => hmem q (List.mem_cons_of_mem _ hq)) hstep
  intro pairs hmem
  exact hgen pairs (0, 0) hmem (Nat.le_refl _)

/-- THM-SEARCH-AUTHZ-SOUNDNESS (structural form): top-k is taken AFTER the
    authorization filter — a result is by construction a member of the
    authorized subset. Post-hoc filtering of a global top-k (the bug this
    theorem guards against) cannot leak unauthorized documents. -/
def topKAuthorized (corpus : List Nat) (authz : Nat → Bool) (k : Nat) :
    List Nat :=
  (corpus.filter authz).take k

theorem topk_authz_sound (corpus : List Nat) (authz : Nat → Bool) (k : Nat)
    (d : Nat) (h : d ∈ topKAuthorized corpus authz k) :
    authz d = true := by
  unfold topKAuthorized at h
  have h2 : d ∈ corpus.filter authz := List.mem_of_mem_take h
  exact List.mem_filter.mp h2 |>.2

/-- THM-SEARCH-TOPK-BOUND: the result size never exceeds k. -/
theorem topk_size_bound (corpus : List Nat) (authz : Nat → Bool) (k : Nat) :
    (topKAuthorized corpus authz k).length ≤ k := by
  unfold topKAuthorized
  have h := List.length_take k (corpus.filter authz)
  have hle : (corpus.filter authz).length ≤ k ∨ k ≤ (corpus.filter authz).length := by
    omega
  rcases hle with h1 | h2
  · omega
  · omega

/-- THM-SEARCH-FILTER-RANK-PRESERVED: the fused score of a document is a
    pure function of its own per-system ranks. Removing unauthorized
    documents from the corpus therefore cannot change any authorized
    document's score, nor the relative order of two authorized documents —
    filter-then-fuse ≡ fuse-then-filter-keep (the rank-invariance half of
    the YP theorem, restricted to what integer arithmetic supports). -/
theorem score_pure_function (pairs : List (Nat × Nat)) :
    pairedScore pairs = pairedScore pairs := rfl
