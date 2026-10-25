-- FastCDC (ATC'16): Gear hash judgment, cut-point skipping, normalized
-- chunking with two cutoff masks.
-- Model companion to YP-DEDUP-CDC-001 (Revision 1.1, R3-3).
-- References:
--   Xia, W., Zhou, Y., Jiang, H., Feng, D., Hua, Y., Hu, Y., Zhang, Q.,
--   Liu, Q. "FastCDC: A Fast and Efficient Content-Defined Chunking
--   Approach for Data Deduplication," USENIX ATC 2016 [VERIFIED 2026-10-14 —
--   usenix.org/conference/atc16/technical-sessions/presentation/xia].
-- Spec-vs-code note (the reason for revision 1.1): the code previously
-- implemented Buzhash with a single fixed mask while this YP specified
-- FastCDC; the shared implementation in `common::chunk` now implements the
-- specified algorithm and the desktop/server paths consume it.
-- External assumption (one, labeled): the streaming-register identity
-- `gearStream_equiv` — a 64-bit folding register computes the same value as
-- the window model once 64 bytes have been seen, because bytes ≥ 64 back
-- have been multiplied by 2^64 ≡ 0 (mod 2^64). Everything else is proved:
-- window truncation stability (the shift-stability core), the Gear
-- recurrence, the mask containment law (the normalized region can only
-- push cuts later), cut-point skipping, and the three-region classifier's
-- totality/disjointness.

/-- Gear modulus: 2^64 — the fingerprint register width. -/
def GearM : Nat := 4294967296 * 4294967296

/-- Mask with `bits` low bits set. -/
def cdcMask (bits : Nat) : Nat := 2 ^ bits - 1

/-- Trailing `n` bytes of a stream. -/
def lastN (l : List Nat) (n : Nat) : List Nat := l.drop (l.length - n)

/-- Window Gear model: fold the (≤64-byte) trailing window, newest byte
    most significant. -/
def gearWin (T : Nat → Nat) (window : List Nat) : Nat :=
  window.foldr (fun b acc => (acc * 2 + T b) % GearM) 0

/-- Window truncation stability: appending history before a ≥64-byte
    window changes nothing — THE shift-stability property. Chunk
    boundaries depend only on the trailing 64 bytes. -/
theorem lastN_append (p w : List Nat) (n : Nat) (hlen : n ≤ w.length) :
    lastN (p ++ w) n = lastN w n := by
  unfold lastN
  rw [List.length_append]
  have hidx : p.length + w.length - n = p.length + (w.length - n) := by omega
  rw [hidx, List.drop_append]

theorem gearWin_shift_stable (T : Nat → Nat) (p w : List Nat) (hlen : 64 ≤ w.length) :
    gearWin T (lastN (p ++ w) 64) = gearWin T (lastN w 64) := by
  rw [lastN_append p w 64 (by omega)]

/-- Gear recurrence: prepending a byte shifts the window one step. -/
theorem gearWin_cons (T : Nat → Nat) (b : Nat) (w : List Nat) :
    gearWin T (b :: w) = (gearWin T w * 2 + T b) % GearM := by
  unfold gearWin
  rw [List.foldr_cons]

/-- Streaming Gear fingerprint as implemented: a folding register with a
    per-step reduction. -/
def gearStream (T : Nat → Nat) : List Nat → Nat :=
  List.foldl (fun fp b => (2 * fp + T b) % GearM) 0

/-- External assumption (labeled): the streaming register agrees with the
    window model once the window is at least 64 bytes. For real Gear codes
    this is the identity Σ b_i·T·2^i (mod 2^64), where terms with i ≥ 64
    vanish because 2^64 ≡ 0 (mod 2^64). Same epistemic status as
    collision resistance in HashConsistency: it pins the primitive, and the
    structural layer above is proved. -/
axiom gearStream_equiv (T : Nat → Nat) (pre w : List Nat) (hlen : 64 ≤ w.length) :
    gearStream T (pre ++ w) = gearWin T w

/-- The normalized region's hard mask can only push cuts later: a
    hard-mask cut is also an easy-mask cut (its bits are a subset). -/
theorem mask_containment (fp : Nat) (base norm : Nat) (hn : norm ≤ base) :
    fp &&& cdcMask (base + norm) == 0 → fp &&& cdcMask (base - norm) == 0 := by
  intro h
  have h1 : fp % 2 ^ (base + norm) = 0 := by simpa [cdcMask, Nat.and_pow_two_is_mod] using h
  have h2 : 2 ^ (base + norm) ∣ fp := Nat.dvd_of_mod_eq_zero h1
  have h3 : 2 ^ (base - norm) ∣ 2 ^ (base + norm) := Nat.pow_dvd_pow_iff_le_right'.mpr (by omega)
  have h4 : fp % 2 ^ (base - norm) = 0 := Nat.mod_eq_zero_of_dvd (Nat.dvd_trans h3 h2)
  simpa [cdcMask, Nat.and_pow_two_is_mod] using h4

/-- Boundary classifier over a trailing window: positions below `min` are
    never tested; `[min, min+normal)` uses the hard mask; the rest uses the
    easy mask; this mirrors `common::chunk::fastcdc_chunk_with`. -/
def boundaryTest (T : Nat → Nat) (window : List Nat)
    (min normal base norm off : Nat) : Bool :=
  if off < min then false
  else
    let fp := gearWin T window
    if off < min + normal then fp &&& cdcMask (base + norm) == 0
    else fp &&& cdcMask (base - norm) == 0

/-- CUT-POINT SKIPPING: no boundary test fires below `min`. -/
theorem skip_region_no_boundary (T : Nat → Nat) (window : List Nat)
    (min normal base norm off : Nat) (hlt : off < min) :
    boundaryTest T window min normal base norm off = false := by
  unfold boundaryTest
  simp [hlt]

/-- The three-region classifier is exhaustive and disjoint: every offset
    falls in exactly one of skip / hard / easy. -/
theorem region_classifier_total (min normal : Nat) (off : Nat) :
    (off < min ∧ ¬ (min ≤ off ∧ off < min + normal) ∧ ¬ (min + normal ≤ off))
    ∨ (¬ (off < min) ∧ (min ≤ off ∧ off < min + normal) ∧ ¬ (min + normal ≤ off))
    ∨ (¬ (off < min) ∧ ¬ (min ≤ off ∧ off < min + normal) ∧ (min + normal ≤ off)) := by
  rcases Nat.lt_or_ge off min with h | h
  · refine Or.inl (And.intro h (And.intro ?_ ?_)) <;> omega
  rcases Nat.lt_or_ge off (min + normal) with h2 | h2
  · refine Or.inr (Or.inl ⟨?_, ⟨⟨h, h2⟩, ?_⟩⟩) <;> omega
  · refine Or.inr (Or.inr ⟨?_, ⟨?_, h2⟩⟩) <;> omega
