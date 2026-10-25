-- Weighted rendezvous hashing (highest-random-weight placement).
-- Model companion to YP-DISTRIBUTED-CH-001 (Ferro spec round R3).
-- References:
--   Thaler, D., Ravishankar, C. "Using Name-Based Mappings to Increase
--   Hit Rates," IEEE/ACM Trans. Networking 6(1), 1998 [HRW rule].
--   Lamping, J., Veach, E. "A Fast, Minimal Memory, Consistent Hash
--   Algorithm" (jump hash), arXiv:1406.2294 [VERIFIED 2026-10-14 — analyzed
--   and rejected for Ferro placement: buckets must be numbered sequentially,
--   no weights, no arbitrary removal; the right shape for flat shard arrays,
--   not for weighted heterogeneous backend routing].
--   Karger, D. et al. "Consistent Hashing and Random Trees," STOC 1997 —
--   the virtual-node ring this module replaces.
-- Uniformity of the underlying hash is an EXTERNAL ASSUMPTION (as
-- elsewhere). Proved here: placement never blackholes, member soundness,
-- the strict-top law (a member strictly out-scoring all others is the
-- unique winner — almost sure under uniform hashing), the snoc law, and
-- addition monotonicity with exact tie semantics (earliest node wins ties).
-- Minimal disruption on removal is pinned by test vectors (TV-CH-*) this
-- round; its full formal proof is deferred.

/-- One HRW comparison step: the incumbent (earlier-scanned) node keeps the
    lead on equal scores; a challenger takes over only by strictly
    out-scoring it. -/
def hrwStep (f : Nat → Nat) (acc : Option Nat) (x : Nat) : Option Nat :=
  match acc with
  | none => some x
  | some y => if f x ≤ f y then some y else some x

/-- Highest-random-weight scan over a node list (earliest-node tie-break). -/
def argmaxBy (f : Nat → Nat) (l : List Nat) : Option Nat :=
  l.foldl (hrwStep f) none

/-- Rendezvous placement of key `k` over `nodes`. -/
def pick (score : Nat → Nat → Nat) (k : Nat) (nodes : List Nat) : Option Nat :=
  argmaxBy (score k) nodes

theorem argmaxBy_nil (f : Nat → Nat) : argmaxBy f [] = none := rfl

/-- SNOC LAW: appending a node `b` feeds exactly one more step to the scan. -/
theorem argmaxBy_snoc (f : Nat → Nat) (l : List Nat) (b : Nat) :
    argmaxBy f (l ++ [b]) = hrwStep f (argmaxBy f l) b := by
  unfold argmaxBy
  rw [List.foldl_append, List.foldl_cons, List.foldl_nil]

/-- A scan step always yields a candidate — placement never blackholes. -/
theorem hrwStep_some (f : Nat → Nat) (acc : Option Nat) (x : Nat) :
    ∃ y, hrwStep f acc x = some y := by
  cases acc with
  | none => exact ⟨x, rfl⟩
  | some y =>
      by_cases h : f x ≤ f y
      · exact ⟨y, by simp [hrwStep, h]⟩
      · exact ⟨x, by simp [hrwStep, h]⟩

theorem foldl_some_of_ne_nil (f : Nat → Nat) : ∀ (l : List Nat) (acc : Option Nat),
    l ≠ [] → ∃ y, l.foldl (hrwStep f) acc = some y := by
  intro l
  induction l with
  | nil => intro acc hne; exact absurd rfl hne
  | cons x rest ih =>
      intro acc hne
      rw [List.foldl_cons]
      obtain ⟨y, hy⟩ := hrwStep_some f acc x
      by_cases hrest : rest = []
      · rw [hrest, List.foldl_nil]
        exact ⟨y, hy⟩
      · rw [hy]
        obtain ⟨y2, hy2⟩ := ih (some y) hrest
        exact ⟨y2, hy2⟩

theorem argmaxBy_exists (f : Nat → Nat) (l : List Nat) (hne : l ≠ []) :
    ∃ m, argmaxBy f l = some m :=
  foldl_some_of_ne_nil f l none hne

/-- Member soundness: a placement is always a node of the set (or, in the
    accumulator-general form, the carried incumbent). -/
theorem foldl_mem (f : Nat → Nat) : ∀ (l : List Nat) (acc : Option Nat) (m : Nat),
    l.foldl (hrwStep f) acc = some m → m ∈ l ∨ acc = some m := by
  intro l
  induction l with
  | nil => intro acc m hc; exact Or.inr hc
  | cons x rest ih =>
      intro acc m hc
      rw [List.foldl_cons] at hc
      cases acc with
      | none =>
          have hc2 : List.foldl (hrwStep f) (some x) rest = some m := hc
          rcases ih (some x) m hc2 with hmem | hxeq
          · exact Or.inl (List.mem_cons_of_mem _ hmem)
          · injection hxeq with he
            subst he
            exact Or.inl (by simp)
      | some y =>
          by_cases hcmp : f x ≤ f y
          · simp only [hrwStep, if_pos hcmp] at hc
            rcases ih (some y) m hc with hmem | haccm
            · exact Or.inl (List.mem_cons_of_mem _ hmem)
            · rw [← haccm]
              exact Or.inr rfl
          · simp only [hrwStep, if_neg hcmp] at hc
            rcases ih (some x) m hc with hmem | hxeq
            · exact Or.inl (List.mem_cons_of_mem _ hmem)
            · injection hxeq with he
              subst he
              exact Or.inl (by simp)

theorem argmaxBy_mem (f : Nat → Nat) (l : List Nat) (m : Nat)
    (hp : argmaxBy f l = some m) : m ∈ l := by
  rcases foldl_mem f l none m hp with h | h
  · exact h
  · exact absurd h (by simp)

/-- Scan invariant: `t` strictly out-scores every list member other than
    itself, and is either a member or the current incumbent. -/
def ScanTop (f : Nat → Nat) (l : List Nat) (acc : Option Nat) (t : Nat) : Prop :=
  (∀ z ∈ l, z ≠ t → f t > f z)
  ∧ (t ∈ l ∨ acc = some t)
  ∧ (∀ y, acc = some y → y ≠ t → f t > f y)

/-- THE STRICT-TOP LAW: a candidate strictly out-scoring everything it is
    compared against ends the scan on top. Under uniform hashing a strict
    top exists almost surely, so this pins placement for the common case. -/
theorem foldl_scan_top (f : Nat → Nat) : ∀ (l : List Nat) (acc : Option Nat) (t : Nat),
    ScanTop f l acc t → l.foldl (hrwStep f) acc = some t := by
  intro l
  induction l with
  | nil =>
      intro acc t hs
      rcases hs.2.1 with h | h
      · cases h
      · exact h
  | cons x rest ih =>
      intro acc t hs
      obtain ⟨hdom, hsplit, hacc⟩ := hs
      rw [List.foldl_cons]
      simp only [hrwStep]
      by_cases hmem : t ∈ rest
      · -- tail strict-top mode
        have hnewacc : ∀ y, hrwStep f acc x = some y → y ≠ t → f t > f y := by
          intro y hy hne
          cases acc with
          | none =>
              simp only [hrwStep] at hy
              injection hy with he
              subst he
              exact hdom x (List.mem_cons_self ..) hne
          | some w =>
              simp only [hrwStep] at hy
              by_cases hcmp : f x ≤ f w
              · rw [if_pos hcmp] at hy
                injection hy with he
                subst he
                exact hacc w rfl hne
              · rw [if_neg hcmp] at hy
                injection hy with he
                subst he
                exact hdom x (List.mem_cons_self ..) hne
        exact ih (hrwStep f acc x) t
          ⟨fun z hz hne => hdom z (List.mem_cons_of_mem _ hz) hne, Or.inl hmem, hnewacc⟩
      · -- champion mode: t = x or acc = some t
        rcases hsplit with hin | hacc_eq
        · rcases List.mem_cons.mp hin with rfl | hr
          · -- t = x (rfl eliminated x; t is the name now)
            cases acc with
            | none =>
                show rest.foldl (hrwStep f) (some t) = some t
                exact ih (some t) t
                  ⟨fun z hz hne => hdom z (List.mem_cons_of_mem _ hz) hne,
                   Or.inr rfl,
                   fun y hy hne => by injection hy with he; subst he; exact absurd rfl hne⟩
            | some w =>
                show rest.foldl (hrwStep f) (if f t ≤ f w then some w else some t) = some t
                by_cases hwt : w = t
                · rw [hwt]
                  by_cases hcmp : f t ≤ f t
                  · rw [if_pos hcmp]
                    exact ih (some t) t
                      ⟨fun z hz hne => hdom z (List.mem_cons_of_mem _ hz) hne,
                       Or.inr rfl,
                       fun y hy hne => by injection hy with he; subst he; exact absurd rfl hne⟩
                  · rw [if_neg hcmp]
                    exact ih (some t) t
                      ⟨fun z hz hne => hdom z (List.mem_cons_of_mem _ hz) hne,
                       Or.inr rfl,
                       fun y hy hne => by injection hy with he; subst he; exact absurd rfl hne⟩
                · have hw := hacc w rfl
                  have hw2 : f t > f w := hw hwt
                  by_cases hcmp : f t ≤ f w
                  · omega
                  · rw [if_neg hcmp]
                    exact ih (some t) t
                      ⟨fun z hz hne => hdom z (List.mem_cons_of_mem _ hz) hne,
                       Or.inr rfl,
                       fun y hy hne => by injection hy with he; subst he; exact absurd rfl hne⟩
          · exact absurd hr hmem
        · -- acc = some t: the incumbent keeps the lead
          subst hacc_eq
          show rest.foldl (hrwStep f) (if f x ≤ f t then some t else some x) = some t
          by_cases hcmp : f x ≤ f t
          · rw [if_pos hcmp]
            exact ih (some t) t
              ⟨fun z hz hne => hdom z (List.mem_cons_of_mem _ hz) hne,
               Or.inr rfl,
               fun y hy hne => by injection hy with he; subst he; exact absurd rfl hne⟩
          · exfalso
            have hxt : x ≠ t := by
              intro hc
              rw [hc] at hcmp
              exact hcmp (Nat.le_refl (f t))
            have hge := hdom x (List.mem_cons_self ..) hxt
            omega

/-- Strict-top witness for a plain scan (no accumulator). -/
def StrictTop (f : Nat → Nat) (l : List Nat) (t : Nat) : Prop :=
  t ∈ l ∧ ∀ z ∈ l, z ≠ t → f t > f z

theorem scanTop_of_strictTop (f : Nat → Nat) (l : List Nat) (t : Nat)
    (hs : StrictTop f l t) : ScanTop f l none t :=
  ⟨hs.2, Or.inl hs.1, by intro y hy; cases hy⟩

theorem strict_top_is_pick (f : Nat → Nat) (l : List Nat) (t : Nat)
    (hs : StrictTop f l t) (hne : l ≠ []) :
    argmaxBy f l = some t :=
  foldl_scan_top f l none t (scanTop_of_strictTop f l t hs)

/-- Uniqueness: the strict top is the ONLY possible winner. -/
theorem strict_top_unique (f : Nat → Nat) (l : List Nat) (t m : Nat)
    (hs : StrictTop f l t) (hp : argmaxBy f l = some m) : m = t := by
  have hmem := argmaxBy_mem f l m hp
  by_cases hm : m = t
  · exact hm
  · exfalso
    have hstrict : argmaxBy f l = some t := strict_top_is_pick f l t hs (by
      intro hnil
      rw [hnil] at hs
      exact absurd hs.1 (by simp))
    rw [hstrict] at hp
    injection hp with he
    exact hm he.symm

/-- ADDITION, keep case: a node that does not out-score the incumbent
    changes no placement (ties keep the incumbent — earliest node wins). -/
theorem addition_keeps (f : Nat → Nat) (l : List Nat) (m nw : Nat)
    (hp : argmaxBy f l = some m) (hnew : f nw ≤ f m) :
    argmaxBy f (l ++ [nw]) = some m := by
  rw [argmaxBy_snoc, hp]
  show (if f nw ≤ f m then some m else some nw) = some m
  rw [if_pos hnew]

/-- ADDITION, move case: a node out-scoring the incumbent captures the
    incumbent's placement. -/
theorem addition_moves (f : Nat → Nat) (l : List Nat) (m nw : Nat)
    (hp : argmaxBy f l = some m) (hnew : f m < f nw) :
    argmaxBy f (l ++ [nw]) = some nw := by
  rw [argmaxBy_snoc, hp]
  show (if f nw ≤ f m then some m else some nw) = some nw
  rw [if_neg (by omega)]

/-- Strict-top refinement of addition-keeps. -/
theorem strict_addition_keeps (f : Nat → Nat) (l : List Nat) (m nw : Nat)
    (hs : StrictTop f l m) (hne : l ≠ []) (hnew : f nw ≤ f m) :
    argmaxBy f (l ++ [nw]) = some m :=
  addition_keeps f l m nw (strict_top_is_pick f l m hs hne) hnew

/-- Strict-top refinement of addition-moves. -/
theorem strict_addition_moves (f : Nat → Nat) (l : List Nat) (m nw : Nat)
    (hs : StrictTop f l m) (hne : l ≠ []) (hnew : f m < f nw) :
    argmaxBy f (l ++ [nw]) = some nw :=
  addition_moves f l m nw (strict_top_is_pick f l m hs hne) hnew

/-- Snoc decomposition: every nonempty list is a list plus a last element. -/
theorem exists_snoc (l : List Nat) (hne : l ≠ []) :
    ∃ l' b, l = l' ++ [b] := by
  induction l with
  | nil => exact absurd rfl hne
  | cons a rest ih =>
      by_cases h : rest = []
      · subst h
        exact ⟨[], a, rfl⟩
      · obtain ⟨l', b, heq⟩ := ih h
        exact ⟨a :: l', b, by rw [heq]; simp [List.cons_append]⟩

/-- Dominance (snoc-induction form): the winner out-scores every member. -/
theorem argmax_dominates_snoc (f : Nat → Nat) :
    ∀ (n : Nat) (l : List Nat) (m : Nat),
      l.length ≤ n → argmaxBy f l = some m → ∀ z ∈ l, f z ≤ f m := by
  intro n
  induction n with
  | zero =>
      intro l m hlen hp
      have hl : l = [] := List.eq_nil_of_length_eq_zero (Nat.eq_zero_of_le_zero hlen)
      subst hl
      cases hp
  | succ n ih =>
      intro l m hlen hp
      by_cases hne : l = []
      · subst l
        cases hp
      · obtain ⟨l', b, hsnoc⟩ := exists_snoc l hne
        subst hsnoc
        have hstep : argmaxBy f (l' ++ [b]) = hrwStep f (argmaxBy f l') b :=
          argmaxBy_snoc f l' b
        rw [hstep] at hp
        have hlen' : l'.length ≤ n := by
          have h1 : (l' ++ [b]).length = l'.length + 1 := by
            simp [List.length_append, List.length_singleton]
          omega
        by_cases hl' : l' = []
        · subst hl'
          have hm : some b = some m := by
            simpa [argmaxBy, hrwStep] using hp
          injection hm with he
          subst he
          intro z hz
          rcases List.mem_append.mp hz with h1 | h2
          · cases h1
          · rcases List.mem_cons.mp h2 with rfl | hr
            · exact Nat.le_refl _
            · exact absurd hr (by simp)
        · have hne' : l' ≠ [] := hl'
          obtain ⟨w, hw⟩ := argmaxBy_exists f l' (by
            cases l' with
            | nil => exact absurd rfl hne'
            | cons x t => simp)
          rw [hw] at hp
          simp only [hrwStep] at hp
          by_cases hcmp : f b ≤ f w
          · rw [if_pos hcmp] at hp
            injection hp with he
            subst he
            intro z hz
            rcases List.mem_append.mp hz with h1 | h2
            · exact ih _ _ hlen' hw z h1
            · rcases List.mem_cons.mp h2 with rfl | hr
              · exact hcmp
              · exact absurd hr (by simp)
          · rw [if_neg hcmp] at hp
            injection hp with he
            subst he
            intro z hz
            rcases List.mem_append.mp hz with h1 | h2
            · have h1' := ih _ _ hlen' hw z h1
              omega
            · rcases List.mem_cons.mp h2 with rfl | hr
              · exact Nat.le_refl _
              · exact absurd hr (by simp)

/-- MINIMAL DISRUPTION (proved): removing a node `r` moves only the keys
    that were placed on `r` — every other placement is stable. Closes the
    R3 test-pinned gap. -/
theorem minimal_disruption (f : Nat → Nat) :
    ∀ (n : Nat) (l : List Nat) (r m : Nat),
      l.length ≤ n →
      argmaxBy f l = some m → m ≠ r →
      argmaxBy f (l.filter (fun z => z != r)) = some m := by
  intro n
  induction n with
  | zero =>
      intro l r m hlen hp _
      have hl : l = [] := List.eq_nil_of_length_eq_zero (Nat.eq_zero_of_le_zero hlen)
      subst hl
      simp only [List.filter_nil, argmaxBy] at hp
      cases hp
  | succ n ih =>
      intro l r m hlen hp hmr
      by_cases hne : l = []
      · subst l
        simp only [List.filter_nil, argmaxBy] at hp
        cases hp
      · obtain ⟨l', b, hsnoc⟩ := exists_snoc l hne
        subst hsnoc
        have hstep : argmaxBy f (l' ++ [b]) = hrwStep f (argmaxBy f l') b :=
          argmaxBy_snoc f l' b
        rw [hstep] at hp
        have hlen' : l'.length ≤ n := by
          have h1 : (l' ++ [b]).length = l'.length + 1 := by
            simp [List.length_append, List.length_singleton]
          omega
        -- filter over the snoc decomposition, with the last element kept
        have hbf : ¬ (b = r) → (l' ++ [b]).filter (fun z => z != r)
            = (l'.filter (fun z => z != r)) ++ [b] := by
          intro hb
          rw [List.filter_append]
          have h1 : List.filter (fun z => z != r) [b] = [b] := by
            simp [hb]
          rw [h1]
        by_cases hb : b = r
        · subst hb
          by_cases hl' : l' = []
          · subst hl'
            have hm : some b = some m := by
              simpa [argmaxBy, hrwStep] using hp
            injection hm with he
            exact absurd he.symm hmr
          · have hne' : l' ≠ [] := hl'
            obtain ⟨w, hw⟩ := argmaxBy_exists f l' (by
              cases l' with
              | nil => exact hne'
              | cons x t => simp)
            rw [hw] at hp
            simp only [hrwStep] at hp
            by_cases hcmp : f b ≤ f w
            · rw [if_pos hcmp] at hp
              injection hp with he
              subst he
              have hbf2 : (l' ++ [b]).filter (fun z => z != b)
                  = l'.filter (fun z => z != b) := by
                rw [List.filter_append]
                have h1 : List.filter (fun z => z != b) [b] = ([] : List Nat) := by
                  simp
                rw [h1, List.append_nil]
              rw [hbf2]
              exact ih l' b w hlen' hw hmr
            · rw [if_neg hcmp] at hp
              injection hp with he
              exact absurd he.symm hmr
        · by_cases hl' : l' = []
          · subst hl'
            have hm : some b = some m := by
              simpa [argmaxBy, hrwStep] using hp
            injection hm with he
            subst he
            rw [List.filter_append]
            simp [hb]
            simp [argmaxBy, hrwStep]
          · have hne' : l' ≠ [] := hl'
            obtain ⟨w, hw⟩ := argmaxBy_exists f l' (by
              cases l' with
              | nil => exact hne'
              | cons x t => simp)
            rw [hw] at hp
            simp only [hrwStep] at hp
            by_cases hcmp : f b ≤ f w
            · rw [if_pos hcmp] at hp
              injection hp with he
              rw [he] at hw hcmp
              have hw' := ih _ _ _ hlen' hw hmr
              rw [hbf hb, argmaxBy_snoc, hw']
              simp only [hrwStep]
              rw [if_pos hcmp]
            · rw [if_neg hcmp] at hp
              injection hp with he
              subst he
              by_cases hfilt : l'.filter (fun z => z != r) = []
              · rw [hbf hb, hfilt]
                simp [argmaxBy, hrwStep]
              · obtain ⟨w', hw'⟩ := argmaxBy_exists f
                  (l'.filter (fun z => z != r)) (by
                  intro hc
                  exact hfilt hc)
                have hdomw : ∀ z ∈ l', f z ≤ f w :=
                  argmax_dominates_snoc f n l' w hlen' hw
                have hmemw' : w' ∈ l'.filter (fun z => z != r) :=
                  argmaxBy_mem f _ w' hw'
                rcases List.mem_filter.mp hmemw' with ⟨h1, _⟩
                have hfww' : f w' ≤ f w := hdomw w' h1
                have hf : ¬ (f b ≤ f w') := by omega
                rw [hbf hb, argmaxBy_snoc, hw']
                simp only [hrwStep]
                rw [if_neg hf]
