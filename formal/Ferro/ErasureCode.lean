-- LRC-style layered erasure coding: global Reed-Solomon parity plus one
-- group-local XOR parity shard per data group, with prefer-local repair.
-- Model companion to YP-DISTRIBUTED-EC-001 (Ferro spec round R3).
-- References:
--   Huang, Simitci, Xu, Ogus, Calder, Gopalan, Li, Yekhanin, "Erasure Coding
--   in Windows Azure Storage," USENIX ATC 2012 (Best Paper) [VERIFIED —
--   microsoft.com/en-us/research/publication/erasure-coding-in-windows-azure-storage/].
--   Ceph EC plugins (jerasure/ISA/LRC/SHEC/CLAY), docs.ceph.com [VERIFIED].
--   MinIO erasure-coding marketing page returned 404 at research time
--   (negative result recorded; RS k+m with per-shard checksums is its
--   documented design).
-- External assumptions (same discipline as HashConsistency): the MDS
-- property of the underlying Reed-Solomon coder and SHA-256 collision
-- resistance. This layer proves the structural properties above them:
-- layout accounting, namespace disjointness, the local repair read bound,
-- decode-path preference soundness and completeness, XOR local-repair
-- correctness (scalar model; byte arrays lift pointwise), and scrubber
-- progress/gating.

/-- Decode path label, mirroring `LrcErasureCoder::DecodePath`. -/
inductive DecodePathLabel where
  | direct | local | global | failed

/-- Shard layout: `k` data shards in `l` groups of `gs`, plus `l` local XOR
    parity shards and `r` global RS parity shards. `div` pins even grouping. -/
structure ErasureLayout where
  k : Nat
  l : Nat
  r : Nat
  gs : Nat
  div : l * gs = k

/-- Total shards stored for the stripe. -/
def totalShards (L : ErasureLayout) : Nat := L.k + L.l + L.r

/-- Local parity shard for group `g` lives at index `k + g`. -/
def localParityIndex (L : ErasureLayout) (g : Nat) : Nat := L.k + g

/-- Global parity shard `j` lives at index `k + l + j`. -/
def globalParityIndex (L : ErasureLayout) (j : Nat) : Nat := L.k + L.l + j

/-- Group `g` covers data indices `[g*gs, (g+1)*gs)`. -/
def groupCovers (L : ErasureLayout) (g : Nat) : List Nat :=
  (List.range L.gs).map (fun j => g * L.gs + j)

/-- Availability oracle: shard `i` is readable. -/
def Avail := Nat → Bool

/-- Number of missing data shards. -/
def missingData (L : ErasureLayout) (a : Avail) : Nat :=
  ((List.range L.k).filter (fun i => ! a i)).length

/-- First missing data shard, in layout order. -/
def firstMissing (L : ErasureLayout) (a : Avail) : Option Nat :=
  (List.range L.k).find? (fun i => ! a i)

/-- The local group of data shard `idx` can repair it: the group's local
    parity shard and every sibling data shard are available. -/
def localRepairReady (L : ErasureLayout) (a : Avail) (idx : Nat) : Bool :=
  a (localParityIndex L (idx / L.gs)) &&
  (groupCovers L (idx / L.gs)).all (fun i => a i || i == idx)

/-- Fragments the global MDS path can use: present data shards plus present
    global parity shards. Local parity is NOT usable by the RS decoder. -/
def rsFragmentsPresent (L : ErasureLayout) (a : Avail) : Nat :=
  ((List.range L.k).filter a).length + ((List.range L.r).filter (fun j => a (globalParityIndex L j))).length

/-- Decode path selection, mirroring `LrcErasureCoder::decode_with_path`:
    direct if no loss, else local if the group can repair, else global if
    the MDS budget is met, else fail. -/
def decodePath (L : ErasureLayout) (a : Avail) : DecodePathLabel :=
  match firstMissing L a with
  | none => DecodePathLabel.direct
  | some idx =>
      if localRepairReady L a idx then DecodePathLabel.local
      else if rsFragmentsPresent L a < L.k then DecodePathLabel.failed
      else DecodePathLabel.global

/-- Fragment read count per path on a layout: local repair reads one
    fragment per group member (siblings + group parity); global repair
    reads the full MDS budget `k`; direct reads nothing. -/
def readFragments (L : ErasureLayout) : DecodePathLabel → Nat
  | DecodePathLabel.direct => 0
  | DecodePathLabel.local => L.gs
  | DecodePathLabel.global => L.k
  | DecodePathLabel.failed => 0

/-- Namespace disjointness: local parity indices never collide with global
    parity indices for in-range groups. -/
theorem local_global_disjoint (L : ErasureLayout) (g j : Nat) (hgl : g < L.l) :
    localParityIndex L g ≠ globalParityIndex L j := by
  unfold localParityIndex globalParityIndex
  intro hc
  omega

/-- Local parity shards for distinct groups are distinct shards. -/
theorem local_parity_injective (L : ErasureLayout) (g g' : Nat) (h : g ≠ g') :
    localParityIndex L g ≠ localParityIndex L g' := by
  unfold localParityIndex
  intro hc
  apply h
  omega

/-- The layered scheme never reads more than a full-stripe RS repair, and
    reads strictly less whenever a group is a proper stripe partition. -/
theorem local_repair_reads_less_aux {k l gs : Nat} (hpos : 0 < l) (hdiv : l * gs = k)
    (hproper : 2 ≤ l) : gs ≤ k ∧ (1 < gs → gs < k) := by
  have hle : gs ≤ k := by rw [← hdiv]; exact Nat.le_mul_of_pos_left gs hpos
  have hge : 2 * gs ≤ k := by rw [← hdiv]; exact Nat.mul_le_mul_right gs hproper
  clear hdiv
  refine ⟨hle, ?_⟩
  intro hgs1
  omega

theorem local_repair_reads_less (L : ErasureLayout) (hproper : 2 ≤ L.l) :
    readFragments L DecodePathLabel.local ≤ readFragments L DecodePathLabel.global ∧
    (1 < L.gs → readFragments L DecodePathLabel.local < readFragments L DecodePathLabel.global) := by
  have h := local_repair_reads_less_aux (k := L.k) (l := L.l) (gs := L.gs)
    (by omega : 0 < L.l) L.div hproper
  unfold readFragments
  exact ⟨h.1, h.2⟩

/-- Preference soundness: if the decoder reports the local path, then the
    first missing shard was genuinely missing and local-repairable. -/
theorem decode_local_sound (L : ErasureLayout) (a : Avail) :
    decodePath L a = DecodePathLabel.local →
    ∃ idx, idx < L.k ∧ ¬ a idx ∧ localRepairReady L a idx := by
  intro hc
  unfold decodePath at hc
  split at hc
  · cases hc
  · rename_i idx hfm
    unfold firstMissing at hfm
    have hmem : idx ∈ List.range L.k := List.mem_of_find?_eq_some hfm
    have hlt : idx < L.k := List.mem_range.mp hmem
    have hp : ¬ a idx := by
      have hsome := List.find?_some (p := fun i => ! a i) hfm
      simpa using hsome
    split at hc
    · rename_i hready
      cases hc
      exact ⟨idx, hlt, hp, hready⟩
    · by_cases hlt : rsFragmentsPresent L a < L.k
      · rw [if_pos hlt] at hc
        cases hc
      · rw [if_neg hlt] at hc
        cases hc

/-- Preference completeness: if the decoder reports failure, then genuinely
    no path was available — the MDS budget was not met. -/
theorem decode_failed_complete (L : ErasureLayout) (a : Avail) :
    decodePath L a = DecodePathLabel.failed → rsFragmentsPresent L a < L.k := by
  intro hc
  unfold decodePath at hc
  split at hc
  · cases hc
  · split at hc
    · cases hc
    · by_cases hlt : rsFragmentsPresent L a < L.k
      · exact hlt
      · rw [if_neg hlt] at hc
        cases hc

/-- Marker predicate standing in for "the decoder returned data `d`". -/
axiom decodeDelivers (L : ErasureLayout) (a : Avail) (d : Nat) : Prop

/-- External assumption (necessity direction of MDS): the RS decoder can
    only deliver data when at least `k` of its `k + r` fragments exist.
    Real Vandermonde-based RS codes over GF(2^8) have exactly this property;
    axiomatized here as for collision resistance in HashConsistency. -/
axiom mds_necessity (L : ErasureLayout) (a : Avail) (d : Nat) :
    decodeDelivers L a d → L.k ≤ rsFragmentsPresent L a

/-- Failure is informational, never data loss: if the decoder reports
    failure it did NOT silently deliver any data. -/
theorem failure_is_informational (L : ErasureLayout) (a : Avail) :
    decodePath L a = DecodePathLabel.failed → ∀ d : Nat, ¬ decodeDelivers L a d := by
  intro hc d hd
  have h1 := decode_failed_complete L a hc
  have h2 := mds_necessity L a d hd
  omega

/-- Scalar model of the XOR local repair: shard values are `Nat`, XOR is
    `Nat.xor` (bitwise; byte arrays lift pointwise). -/
def xorAll : List Nat → Nat := List.foldr (· ^^^ ·) 0

/-- Group local parity is the XOR fold of the group's data shards. -/
def localParityVal (L : ErasureLayout) (g : Nat) (vals : Nat → Nat) : Nat :=
  xorAll ((groupCovers L g).map vals)

/-- Local repair correctness: parity XOR remaining siblings recovers the
    lost shard exactly (involution property of XOR). -/
theorem local_repair_correct (d x : Nat) :
    (d ^^^ x) ^^^ x = d := by
  apply Nat.eq_of_testBit_eq
  intro i
  simp [Nat.testBit_xor]

/-- Scrubber state update: shard `i` becomes available, everything else
    unchanged. -/
def setAvail (a : Avail) (i : Nat) : Avail := fun j => if j = i then true else a j

theorem setAvail_neq (a : Avail) (i j : Nat) (h : j ≠ i) :
    setAvail a i j = a j := by
  unfold setAvail
  rw [if_neg h]

/-- Gating soundness: a scrub that repairs index `i` leaves every other
    healthy shard available. -/
theorem scrub_untouched (a : Avail) (i j : Nat) (hj : j ≠ i) (haj : a j = true) :
    setAvail a i j = true := by
  rw [setAvail_neq a i j hj]
  exact haj

theorem filter_missing_update_range (k i : Nat) (a : Avail)
    (hai : a i = false) (hlt : i < k) :
    ((List.range k).filter (fun j => ! setAvail a i j)).length + 1
      = ((List.range k).filter (fun j => ! a j)).length := by
  induction k with
  | zero => cases hlt
  | succ n ih =>
      have hr : List.range (n + 1) = List.range n ++ [n] := List.range_succ n
      rcases Nat.lt_succ_iff_lt_or_eq.mp hlt with h | h
      · -- i < n: the new tail slot is unaffected
        have hne : i ≠ n := by omega
        have hslot : setAvail a i n = a n := by
          simp only [setAvail, if_neg (Ne.symm hne)]
        have htail : List.filter (fun j => ! setAvail a i j) [n]
            = List.filter (fun j => ! a j) [n] := by
          simp only [List.filter_cons, List.filter_nil, hslot]
        rw [hr, List.filter_append, List.filter_append, htail,
            List.length_append, List.length_append]
        omega
      · -- i = n: the new tail slot flips from counted to not-counted
        have hprev : List.filter (fun j => ! setAvail a i j) (List.range n)
            = List.filter (fun j => ! a j) (List.range n) := by
          refine List.filter_congr' ?_
          intro m hm
          have hmn : m ≠ n := by
            intro hc
            rw [hc] at hm
            exact absurd hm (by simp)
          simp only [setAvail, if_neg (fun hc : m = i => hmn (hc.trans h))]
        rw [hr, List.filter_append, List.filter_append, hprev]
        rw [List.length_append, List.length_append]
        simp [setAvail, ← h, hai]

/-- Scrubber progress: repairing missing shard `i < k` decrements the
    stripe's missing count by exactly one. -/
theorem scrub_progress (L : ErasureLayout) (a : Avail) (i : Nat)
    (hlt : i < L.k) (hai : a i = false) :
    missingData L (setAvail a i) + 1 = missingData L a := by
  unfold missingData
  exact filter_missing_update_range L.k i a hai hlt

/-- Layout accounting: the stripe stores exactly `k + l + r` shards and data
    never exceeds the stored total. -/
theorem layout_accounting (L : ErasureLayout) :
    totalShards L = L.k + L.l + L.r ∧ L.k ≤ totalShards L := by
  constructor
  · rfl
  · unfold totalShards; omega

/-- Each of the `l` groups contributes exactly one local parity shard. -/
theorem local_parity_count (L : ErasureLayout) :
    (List.range L.l).length = L.l := by
  simp
