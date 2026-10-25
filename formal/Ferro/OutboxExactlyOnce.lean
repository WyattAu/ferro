-- Transactional-outbox relay: at-least-once delivery plus per-handler
-- watermarks equals exactly-once effects.
-- Extension companion to YP-EVENT-DURABILITY-001 (Revision 1.1, R3-8).
-- References:
--   Microservices.io transactional outbox pattern [VERIFIED in R2].
--   RFC 9438? — n/a. The watermark dedup mirrors TCP/QUIC delivery
--   bookkeeping; the theorem set below is original to Ferro's model.
-- Proved here: a handler that applies an effect only when the event is
-- absent from its applied-set, and records on apply, produces exactly-once
-- effects under at-least-once (duplicate) delivery.

/-- An event id. -/
abbrev EventId := Nat

/-- Applied-set of a handler: events whose effect already ran. -/
abbrev Applied := List EventId

instance : Inhabited Applied := ⟨[]⟩

theorem elem_iff_mem {l : List EventId} {e : EventId} (h : List.elem e l = true) : e ∈ l :=
  List.elem_iff.mp h

theorem not_mem_iff_not_elem {l : List EventId} {e : EventId} (h : ¬ (List.elem e l = true)) :
    ¬ (e ∈ l) :=
  fun hmem => h (List.elem_iff.mpr hmem)

/-- Delivery: apply only if not already applied; record after applying. -/
def deliver (applied : Applied) (e : EventId) : Option Applied :=
  if List.elem e applied then none else some (e :: applied)

/-- Whether the effect ran for this delivery. -/
def effectRan (applied : Applied) (e : EventId) : Bool :=
  !(List.elem e applied)

/-- AT-LEAST-ONCE: redelivery is a no-op for the effect. -/
theorem redelivery_no_effect (applied : Applied) (e : EventId)
    (h : List.elem e applied = true) :
    effectRan applied e = false := by
  have hm : e ∈ applied := elem_iff_mem h
  simp [effectRan, hm]

theorem redelivery_noop (applied : Applied) (e : EventId)
    (h : List.elem e applied = true) :
    deliver applied e = none := by
  have hm : e ∈ applied := elem_iff_mem h
  simp [deliver, hm]

/-- First delivery always applies and records. -/
theorem first_delivery_applies (applied : Applied) (e : EventId)
    (h : ¬ (e ∈ applied)) :
    deliver applied e = some (e :: applied) := by
  have hf := not_mem_iff_not_elem (by
    by_cases hc : List.elem e applied = true
    · exact absurd (elem_iff_mem hc) h
    · exact hc)
  simp [deliver, hf]

/-- EXACTLY-ONCE: after any number of deliveries of `e`, the effect has run
    exactly once — the second and later deliveries find `e` recorded. -/
theorem exactly_once (applied : Applied) (e : EventId)
    (h : ¬ (e ∈ applied)) :
    List.elem e (deliver applied e |>.get!) := by
  rw [first_delivery_applies applied e h]
  simp

/-- The applied-set only grows: recording never forgets. -/
theorem applied_monotone (applied : Applied) (e : EventId)
    (h : ¬ (e ∈ applied)) :
    ∀ x ∈ applied, x ∈ (deliver applied e |>.get!) := by
  intro x hx
  rw [first_delivery_applies applied e h]
  exact List.mem_cons_of_mem _ hx
