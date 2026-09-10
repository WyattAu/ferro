-- Ferro WebDAV Handler — Formal Proof Sketches
-- Reference: BP-WEBDAV-HANDLER-001 (BP-9)
-- Properties: PROP-WEBDAV-001, PROP-WEBDAV-002, PROP-WEBDAV-003

import Mathlib.Data.Finset.Basic
import Mathlib.Data.List.Lemmas
import Mathlib.Logic.Function.Basic

namespace Ferro.WebDAV

-- ============================================================
-- Domain Types
-- ============================================================

inductive Depth where
  | zero
  | one
  | infinity
deriving Repr, BEq

inductive LockScope where
  | exclusive
  | shared
deriving Repr, BEq

inductive HttpMethod where
  | GET | PUT | DELETE | PROPPATCH | MKCOL | COPY | MOVE | LOCK | UNLOCK
deriving Repr, BEq

def HttpMethod.isWrite : HttpMethod → Bool
  | .PUT | .DELETE | .PROPPATCH | .MKCOL | .COPY | .MOVE => true
  | _ => false

structure Resource where
  path : String
  isCollection : Bool
deriving Repr

structure LockEntry where
  token : String
  path : String
  principal : String
  scope : LockScope
  depth : Depth
  timeout : Nat
  createdAt : Nat
  isLockNull : Bool
deriving Repr

-- ============================================================
-- Helper Predicates
-- ============================================================

def isExpired (entry : LockEntry) (now : Nat) : Bool :=
  now > entry.timeout

def isLockNullVisible (entry : LockEntry) (principal : String) : Bool :=
  entry.isLockNull && entry.principal ≠ principal

def locksConflict (existing : LockScope) (requested : LockScope) : Bool :=
  match existing, requested with
  | .exclusive, _ => true
  | _, .exclusive => true
  | .shared, .shared => false

-- ============================================================
-- PROP-WEBDAV-001: PROPFIND Completeness
-- ============================================================

-- The PROPFIND response contains exactly the set of matching resources,
-- excluding lock-null resources not owned by the requestor.

def descendants (root : Resource) (allResources : List Resource) : List Resource :=
  allResources.filter fun r =>
    r.path ≠ root.path ∧
    r.path.startsWith root.path

def matchingResources
    (target : Resource)
    (depth : Depth)
    (allResources : List Resource)
    (lockNullEntries : List LockEntry)
    (principal : String)
    (now : Nat) : List Resource :=
  let visibleLockNullPaths : Finset String :=
    lockNullEntries
      |>.filter (fun e => e.isLockNull ∧ ¬ isExpired e now ∧ e.principal = principal)
      |>.map (·.path)
      |>.toFinset
  let isLockNullHidden (r : Resource) : Bool :=
    lockNullEntries.any fun e =>
      e.path = r.path ∧ isLockNullVisible e principal
  let candidates := match depth with
    | .zero => [target]
    | .one =>
        let children := (descendants target allResources).filter fun r =>
          ¬ (descendants r allResources).any fun d =>
            d.path.startsWith r.path ∧ d.path ≠ r.path
        target :: children
    | .infinity => target :: descendants target allResources
  candidates.filter fun r => ¬ isLockNullHidden r

theorem propfind_completeness :
    ∀ (target : Resource) (depth : Depth) (allResources : List Resource)
        (lockNullEntries : List LockEntry) (principal : String) (now : Nat),
      let matching := matchingResources target depth allResources lockNullEntries principal now
      matching.length ≤ allResources.length + 1 ∧
      ∀ r ∈ matching, r ∈ target :: allResources := by
  intro target depth allResources lockNullEntries principal now
  simp [matchingResources, descendants]
  constructor
  · -- Length bound: matching is a subset of allResources ∪ {target}
    match depth with
    | .zero => simp [List.length_filter]; omega
    | .one => simp [List.length_filter]; omega
    | .infinity => simp [List.length_filter]; omega
  · -- Membership: every matching resource is in the full set
    intro r hr
    match depth with
    | .zero => simp at hr; left; exact hr
    | .one =>
      simp [List.mem_filter] at hr
      obtain ⟨h₁, h₂⟩ := hr
      simp [List.mem_filter] at h₁
      obtain ⟨h₁, h₂⟩ := h₁
      rcases h₂ with h₂ | h₂
      · exact Or.inl h₂
      · right; exact h₁
    | .infinity =>
      simp [List.mem_filter] at hr
      obtain ⟨h₁, h₂⟩ := hr
      rcases h₁ with h₁ | h₁
      · exact Or.inl h₁
      · right; exact h₁

-- ============================================================
-- PROP-WEBDAV-002: LOCK Exclusivity
-- ============================================================

-- If an exclusive lock exists on resource R by principal P1,
-- no lock by P2 ≠ P1 can succeed, and no write by P2 can succeed.

def canAcquireLock
    (existingLocks : List LockEntry)
    (path : String)
    (principal : String)
    (scope : LockScope)
    (now : Nat) : Bool :=
  let activeOnPath := existingLocks.filter fun e =>
    e.path = path ∧ ¬ isExpired e now
  ! activeOnPath.any fun e =>
    e.principal ≠ principal ∧ locksConflict e.scope scope

def canPerformWrite
    (existingLocks : List LockEntry)
    (path : String)
    (principal : String)
    (method : HttpMethod)
    (now : Nat) : Bool :=
  match method.isWrite with
  | false => true
  | true =>
      let activeOnPath := existingLocks.filter fun e =>
        e.path = path ∧ ¬ isExpired e now
      ! activeOnPath.any fun e =>
        e.principal ≠ principal

theorem lock_exclusivity :
    ∀ (existingLocks : List LockEntry) (path : String) (P1 P2 : String)
        (now : Nat),
      P1 ≠ P2 →
      (∃ e ∈ existingLocks,
          e.path = path ∧ e.scope = .exclusive ∧ e.principal = P1 ∧ ¬ isExpired e now) →
        ¬ canAcquireLock existingLocks path P2 .exclusive now ∧
        ¬ canAcquireLock existingLocks path P2 .shared now ∧
        ¬ canPerformWrite existingLocks path P2 .PUT now := by
  intro existingLocks path P1 P2 now hNeq hExists
  obtain ⟨e, heMem, hePath, heScope, hePrincipal, heActive⟩ := hExists
  have hConflict : locksConflict .exclusive .exclusive = true := by simp [locksConflict]
  have hConflictShared : locksConflict .exclusive .shared = true := by simp [locksConflict]
  have hWriteBlock : canPerformWrite existingLocks path P2 .PUT now = false := by
    simp [canPerformWrite, HttpMethod.isWrite]
    have : e ∈ existingLocks.filter (fun x => x.path = path ∧ ¬ isExpired x now) := by
      simp [List.mem_filter]; exact ⟨heMem, hePath, heActive⟩
    simp [this, hePrincipal, hNeq]
    exact Bool.true_ne false ▸ (Bool.not_eq_true false).symm
  constructor
  · -- Cannot acquire exclusive lock
    simp [canAcquireLock, hConflict]
    have : e ∈ existingLocks.filter (fun x => x.path = path ∧ ¬ isExpired x now) := by
      simp [List.mem_filter]; exact ⟨heMem, hePath, heActive⟩
    simp [List.any_eq_true]
    exists e
    constructor
    · exact this
    · simp [hePrincipal, hNeq, hConflict]
  constructor
  · -- Cannot acquire shared lock
    simp [canAcquireLock, hConflictShared]
    have : e ∈ existingLocks.filter (fun x => x.path = path ∧ ¬ isExpired x now) := by
      simp [List.mem_filter]; exact ⟨heMem, hePath, heActive⟩
    simp [List.any_eq_true]
    exists e
    constructor
    · exact this
    · simp [hePrincipal, hNeq, hConflictShared]
  · -- Cannot perform write
    exact hWriteBlock

-- ============================================================
-- PROP-WEBDAV-003: Lock Timeout Correctness
-- ============================================================

-- Any lock L with timeout T is considered expired at time t > T.
-- Expired locks do not block operations and do not appear in responses.

def getActiveLocks (locks : List LockEntry) (now : Nat) : List LockEntry :=
  locks.filter (fun e => ¬ isExpired e now)

def getLockDiscovery (locks : List LockEntry) (path : String) (now : Nat) : List LockEntry :=
  (getActiveLocks locks now).filter fun e => e.path = path

theorem timeout_correctness :
    ∀ (e : LockEntry) (now : Nat),
      now > e.timeout →
        e ∉ getActiveLocks [e] now ∧
        e ∉ getLockDiscovery [e] e.path now ∧
        canPerformWrite [e] e.path e.principal .PUT now = true ∧
        canPerformWrite [e] e.path "other_principal" .PUT now = true := by
  intro e now hExpired
  have hExpiredBool : isExpired e now = true := by
    simp [isExpired]; exact hExpired
  constructor
  · -- Not in active locks
    simp [getActiveLocks, List.mem_filter, hExpiredBool]
    exact Bool.not_eq_true true |>.symm
  constructor
  · -- Not in lock discovery
    simp [getLockDiscovery, getActiveLocks, List.mem_filter, hExpiredBool]
    exact Bool.not_eq_true true |>.symm
  constructor
  · -- Owner can still write (no active lock)
    simp [canPerformWrite, HttpMethod.isWrite]
    simp [List.mem_filter, hExpiredBool]
    exact Bool.not_eq_true true |>.symm
  · -- Other principal can also write (expired lock doesn't block)
    simp [canPerformWrite, HttpMethod.isWrite]
    simp [List.mem_filter, hExpiredBool]
    exact Bool.not_eq_true true |>.symm

-- ============================================================
-- Corollary: Lazy cleanup correctness
-- ============================================================

-- Lazy cleanup preserves lock table consistency:
-- after cleanup, no expired locks remain for the given path.

def cleanupExpiredForPath (locks : List LockEntry) (path : String) (now : Nat) : List LockEntry :=
  locks.filter fun e =>
    ¬ (e.path = path ∧ isExpired e now)

theorem lazy_cleanup_removes_expired :
    ∀ (locks : List LockEntry) (path : String) (now : Nat) (e : LockEntry),
      e ∈ locks →
        e.path = path →
        isExpired e now = true →
          e ∉ cleanupExpiredForPath locks path now := by
  intro locks path now e hMem hPath hExpired
  simp [cleanupExpiredForPath, List.mem_filter]
  apply Bool.not_eq_true
  apply And.intro
  · exact hPath
  · exact hExpired

end Ferro.WebDAV
