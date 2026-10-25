-- SIEVE cache eviction (NSDI'24) with singleflight origin-load suppression.
-- Model companion to YP-CONSISTENCY-CACHE-001 (Revision 1.1).
-- Reference: Zhang, Y., Yang, Y., Yue, G., Vigfusson, Y., Rashmi, K. V.
-- "SIEVE is an Efficient Turn-Key Eviction Algorithm for Web Caches,"
-- NSDI 2024; sievecache.com [VERIFIED]. Mechanism: FIFO queue + one visit
-- bit per object; the hand clears visited bits and rotates; unvisited
-- objects are evicted. Second chances are single-use.

/-- Number of entries currently marked visited. -/
def numVisited (l : List (Nat × Bool)) : Nat :=
  (l.filter (·.2)).length

/-- SIEVE eviction walk: visited head is unmarked and rotated to the tail
    (single-use second chance); unvisited head is evicted. -/
def sieveEvict : List (Nat × Bool) → List (Nat × Bool)
  | (k, true) :: rest => sieveEvict (rest ++ [(k, false)])
  | (k, false) :: rest => rest
  | [] => []
termination_by l => numVisited l
decreasing_by
  all_goals simp [numVisited, List.filter_append] <;> omega

/-- Mechanism, rotation step: a visited head is unmarked and rotated. -/
theorem rotate_unmarks (k : Nat) (rest : List (Nat × Bool)) :
    sieveEvict ((k, true) :: rest) = sieveEvict (rest ++ [(k, false)]) := by
  simp only [sieveEvict]

/-- Mechanism, victim step: an unvisited head is evicted (FIFO fairness,
    no LRU bookkeeping on hits). -/
theorem unvisited_head_victim (k : Nat) (rest : List (Nat × Bool)) :
    sieveEvict ((k, false) :: rest) = rest := by
  simp only [sieveEvict]

/-- Eviction on a nonempty cache removes exactly one entry. -/
theorem sieve_evict_reduces : ∀ (n : Nat) (l : List (Nat × Bool)),
    numVisited l ≤ n → l ≠ [] → (sieveEvict l).length + 1 = l.length := by
  intro n
  induction n with
  | zero =>
      intro l hle hne
      cases l with
      | nil => exact absurd rfl hne
      | cons p rest =>
          obtain ⟨k, kb⟩ := p
          cases kb with
          | true =>
              have h1 : numVisited ((k, true) :: rest) = 1 + numVisited rest := by
                simp [numVisited, List.filter_cons]
                omega
              omega
          | false =>
              have h0 : numVisited ((k, false) :: rest) = numVisited rest := by
                simp [numVisited, List.filter_cons]
              simp only [sieveEvict, List.length_cons]
  | succ n ih =>
      intro l hle hne
      cases l with
      | nil => exact absurd rfl hne
      | cons p rest =>
          obtain ⟨k, kb⟩ := p
          cases kb with
          | false =>
              have h0 : numVisited ((k, false) :: rest) = numVisited rest := by
                simp [numVisited, List.filter_cons]
              simp only [sieveEvict, List.length_cons]
          | true =>
              have h1 : numVisited ((k, true) :: rest) = 1 + numVisited rest := by
                simp [numVisited, List.filter_cons]
                omega
              have happ : numVisited (rest ++ [(k, false)]) = numVisited rest := by
                simp [numVisited, List.filter_append]
              have hlt : numVisited (rest ++ [(k, false)]) ≤ n := by omega
              have hne : rest ++ [(k, false)] ≠ [] := by
                intro he
                have := congrArg List.length he
                simp at this
              rw [sieveEvict]
              have hrec := ih _ hlt hne
              simp only [List.length_append, List.length_cons, List.length_nil,
                List.length_singleton] at hrec ⊢
              omega

/-- THM-CACHE-SIZE-BOUND-UNDER-EVICT: eviction never grows the cache and
    removes exactly one entry from a nonempty one. -/
theorem evict_never_grows (l : List (Nat × Bool)) :
    (sieveEvict l).length ≤ l.length := by
  cases l with
  | nil => simp [sieveEvict]
  | cons p rest =>
      obtain ⟨k, kb⟩ := p
      cases kb with
      | false => simp [sieveEvict]
      | true =>
          have hrec := sieve_evict_reduces (numVisited ((k, true) :: rest))
            ((k, true) :: rest) (Nat.le_refl _) (by simp)
          simp only [List.length_cons] at hrec ⊢
          omega

/-- Lookup marks the entry visited but does NOT reorder the queue —
    SIEVE's advantage over LRU: hits are O(1) flag writes, no list
    surgery, no write contention on the hot path. -/
def sieveLookup (c : List (Nat × Bool)) (k : Nat) :
    List (Nat × Bool) × Option Bool :=
  (c.map (fun p => if p.1 = k then (p.1, true) else p), c.lookup k)

/-- Order preservation: keys keep their FIFO positions across lookups. -/
theorem lookup_preserves_order (c : List (Nat × Bool)) (k : Nat) :
    (sieveLookup c k).1.map (·.1) = c.map (·.1) := by
  unfold sieveLookup
  simp only [List.map_map]
  apply List.map_congr_left
  intro p
  by_cases hp : p.1 = k <;> simp [hp]

/-- The marking lookup returns the key's visit bit unchanged. -/
theorem lookup_flag_stable (c : List (Nat × Bool)) (k : Nat) (b : Bool)
    (h : c.lookup k = some b) :
    (sieveLookup c k).2 = some b := by
  unfold sieveLookup
  exact h

/-- Insert appends at the tail with a clear visit bit. -/
def sieveInsert (cap : Nat) (c : List (Nat × Bool)) (k : Nat) :
    List (Nat × Bool) :=
  if c.length < cap then c ++ [(k, false)]
  else sieveEvict c ++ [(k, false)]

/-- THM-CACHE-SIZE-BOUND: the cache never exceeds its capacity (cap ≥ 1). -/
theorem insert_size_bound (cap : Nat) (c : List (Nat × Bool)) (k : Nat)
    (h : c.length ≤ cap) (hcap : 1 ≤ cap) :
    (sieveInsert cap c k).length ≤ cap := by
  unfold sieveInsert
  split
  · next hlt =>
      simp only [List.length_append, List.length_singleton]
      omega
  · next hlt =>
      by_cases hc : c = []
      · subst hc
        simp only [sieveEvict, List.nil_append, List.length_nil,
          List.length_singleton] at h ⊢
        omega
      · have hrec := sieve_evict_reduces (numVisited c) c (Nat.le_refl _) hc
        simp only [List.length_append, List.length_singleton] at hrec ⊢
        omega

/-- Fresh inserts append at the tail with a clear visit bit (SIEVE
    invariant: new objects get no second chance until used). -/
theorem insert_unvisited (cap : Nat) (c : List (Nat × Bool)) (k : Nat)
    (h : c.length < cap) :
    ∃ rest, sieveInsert cap c k = rest ++ [(k, false)] := by
  unfold sieveInsert
  rw [if_pos h]
  exact ⟨c, rfl⟩

-- ===== Singleflight: at most one origin load per key =====

/-- A singleflight gate: if the key is already in flight, wait (0 loads);
    otherwise claim it (1 load). -/
def sfStep (acc : List Nat × Nat) (k : Nat) : List Nat × Nat :=
  if k ∈ acc.1 then acc else (k :: acc.1, acc.2 + 1)

def sfRun (calls : List Nat) (inflight : List Nat) : List Nat × Nat :=
  calls.foldl sfStep (inflight, 0)

/-- THM-SINGLEFLIGHT-DEDUP: a caller for an in-flight key performs zero
    origin loads — thundering-herd suppression per key. -/
theorem singleflight_dedup (inflight : List Nat) (k : Nat) (n : Nat)
    (h : k ∈ inflight) :
    (sfStep (inflight, n) k).2 = n := by
  show (if k ∈ inflight then (inflight, n) else (k :: inflight, n + 1)).2 = n
  rw [if_pos h]

/-- Singleflight gate over a call sequence with arbitrary seed, given the
    invariant that seed loads ≤ seed keys: origin loads stay bounded by
    the deduplicated key count. -/
theorem stampede_bound_seed : ∀ (calls : List Nat) (seed : List Nat × Nat),
    seed.2 ≤ seed.1.length →
    (calls.foldl sfStep seed).2 ≤ (calls.foldl sfStep seed).1.length := by
  intro calls
  induction calls with
  | nil => intro seed h; exact h
  | cons hd tl ih =>
      intro seed h
      simp only [List.foldl_cons, sfStep]
      by_cases hmem : hd ∈ seed.1
      · rw [if_pos hmem]
        exact ih seed h
      · rw [if_neg hmem]
        have hinv : seed.2 + 1 ≤ (hd :: seed.1).length := by
          simp only [List.length_cons]
          omega
        exact ih (hd :: seed.1, seed.2 + 1) hinv

/-- THM-STAMPEDE-BOUND: driving a call sequence through the gate yields at
    most one origin load per distinct key — origin loads are bounded by
    the deduplicated key count, never the call count. -/
theorem stampede_bound (calls inflight : List Nat) :
    (sfRun calls inflight).2 ≤ (sfRun calls inflight).1.length :=
  stampede_bound_seed calls (inflight, 0) (Nat.zero_le _)
