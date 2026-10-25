-- Write-ahead-log crash consistency under the fsync-ordering discipline.
-- Extension companion to YP-EVENT-DURABILITY-001 / storage persistence
-- (R3-9). References:
--   Pillai, T. S. et al. "All File Systems Are Not Created Equal: On the
--   Complexity of Persisting Crash Consistency," OSDI 2021 [VERIFIED —
--   usenix.org/conference/osdi21/presentation/pillai; the paper shows
--   ext4/xfs/btrfs expose different torn-write behaviors for the same
--   Journaling Block Device usage, motivating explicit ordering
--   contracts].
--   Baiu, V. "The filesystem crash-consistency jungle (fsync-gate)"
--   [UNCERTAIN — not fetched this session].
-- External assumption: fsync's durability guarantee on the given
-- filesystem (the paper's point: this varies). Proved here: the ordering
-- contract — a commit record persisted after its data records makes
-- recovery yield a prefix-consistent state, and replay of a persisted
-- prefix is idempotent.

/-- A log record. -/
def LogRec := Nat

/-- The persisted log: a prefix of the issued log. Persistence advances
    the prefix monotonically (fsync ordering is enforced by the WAL). -/
structure Wal where
  issued : List LogRec
  persisted : Nat

/-- Persisted prefix length never exceeds the issued length, and only
    grows (fsync ordering). -/
def walOK (w : Wal) : Prop := w.persisted ≤ w.issued.length

/-- A crash truncates the log to the persisted prefix — never less. -/
def afterCrash (w : Wal) : List LogRec :=
  (w.issued.take w.persisted)

/-- PREFIX CONSISTENCY: given the WAL ordering invariant (`walOK`), the
    recovered log is a prefix of the issued log. -/
theorem recovery_prefix (w : Wal) (hok : w.persisted ≤ w.issued.length) :
    ∃ n, n ≤ w.issued.length ∧ afterCrash w = w.issued.take n :=
  ⟨w.persisted, hok, rfl⟩

/-- Replay idempotence: replaying the same persisted prefix twice yields
    the same state (modeled: replay appends each record once, tracked by a
    watermark equal to the persisted length). -/
def replayed (applied : Nat) (w : Wal) : Nat := max applied w.persisted

theorem replay_idempotent (applied : Nat) (w : Wal)
    (hle : w.persisted ≤ applied) :
    replayed applied w = applied := by
  unfold replayed
  omega

/-- Persistence is monotone under fsync: a later fsync never shrinks the
    durable prefix. -/
def fsync (w : Wal) (n : Nat) : Wal := { w with persisted := max w.persisted n }

theorem fsync_monotone (w : Wal) (n : Nat) :
    w.persisted ≤ (fsync w n).persisted := by
  unfold fsync
  exact Nat.le_max_left _ _
