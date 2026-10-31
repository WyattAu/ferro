# YP-AUTHZ-POLICY-001: Cedar Schema Validation, Entity Modeling & Meta-Permissions

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-AUTHZ-POLICY-001                        |
| Domain       | Identity & Access Management               |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Iteration 2026-10) |
| Date         | 2026-10-09                                 |
| Crate Ref    | `crates/auth/src/cedar.rs`, `crates/auth/src/policies.rs` |
| Pinned Deps  | `cedar-policy` locked 4.11.1 (latest 4.13.0, 2026-09-15); language spec 4.5 |
| Extends      | YP-AUTH-OIDC-CEDAR-001                     |

---

## YP-1: Ground Truth (Current Implementation)

| Aspect | Ferro today | Location |
|---|---|---|
| Engine | cedar-policy 4.11.1, in-memory `Arc<RwLock<PolicySet>>` | `crates/auth/src/cedar.rs` |
| Default | deny when no policy matches | same |
| **Schema** | **NONE; no Cedar schema, no entity store, no validator invocation** | verified by read |
| Entities | synthesized ad hoc per request from space membership | same |
| Space read | `context.path like "/_spaces/*"` | `generate_auto_policies` |
| Editor/manager | hand-rolled string prefix matching over `context.path` | same |
| Own home | `/users/{sub}` **bypasses Cedar, enforced in Rust** | same |
| Admin | `context.sub == admin_sub ∨ "ferro-admin" ∈ context.groups` | same |

**Three defects:**
1. **No schema ⇒ no validation ⇒ silent no-ops.** Cedar's docs are explicit: the schema validates policies at create/update time only; policies that error at evaluation are **silently ignored**. A typo'd attribute (`principal.isAcve`) is syntactically valid, fails nothing at write time, and silently drops from every decision; a silent permission loss with no log line. Without a schema Ferro cannot even detect this class. The validator classifies exactly these cases (unrecognized entity types/actions, `in`/`==` misuse, unrecognized attributes, unsafe optional access, type mismatches, invalid entity literals). [VERIFIED; Cedar docs]
2. **String-path authorization is fragile.** Cedar `like "/_spaces/*"` treats `*` as a multi-segment wildcard: `/spaces/a*` also matches `/spaces/ab` and `/spaces/a/b`. Path-as-string is the wrong primary key.
3. **Authorization split across two languages.** The Rust-side `/users/{sub}` bypass diverges from Cedar; every new endpoint must remember it exists.

---

## YP-2: Executive Summary

### Problem Statement

Ferro's authorization is Cedar-in-name but bypasses Cedar's safety machinery (schema, validation, typed entities). The 2024 Cedar OOPSLA paper proved validation soundness in Lean; the machinery exists and is proven; Ferro simply isn't invoking it.

### Decisions

- **P0; Adopt a Cedar schema; validate every policy write; fail startup on invalid generated policies.** Entity types `User, Group, Space, Document, Policy, Application`; actions `read, list, create, update, delete, share, changePermissions`; typed context record. Wire the validator into `policies.rs` on **every** write (reject the write on error); revalidate `generate_auto_policies` at startup and **fail startup** on validation error.
- **P1; Replace string paths with entity identifiers.** `context.path like` → `space_ids` set-membership relations on `Document`; editor/manager prefix checks → entity relations on `Space`. Path remains display-only.
- **P2; Eliminate the Rust bypass via meta-permissions:** actions `viewPermissions / changePermissions / managePolicies` in Cedar; the `/users/{sub}` rule becomes a normal policy; a manager may not change permissions above their own level.
- **P3; Test the meta-permissions** with `cedar-policy-cli` validation and the `cedar-spec` differential corpus as fixtures.
- **P4; Version the policy set** (monotone policy-set version; evaluation consistent per version); prepares multi-instance coherence (SpiceDB-style consistency modes as vocabulary).
- **P5; Bump `cedar-policy` 4.11.1 → 4.13.0** and re-run the full policy suite (minor versions have behavior changes).

### Guarantees

- Deny by default (no policy match ⇒ deny).
- Validation soundness: validated policies do not raise evaluation errors on conforming requests (Cedar's proven property, applied).
- Single language: every effective decision is a Cedar evaluation.
- No path-wildcard authorization.
- No escalation: no principal can grant rights it does not hold.

---

## YP-3: Nomenclature

| Term | Definition |
|---|---|
| Cedar schema | Typed declaration of entity types, actions, and the principal/action/resource/context shape per action. |
| Validation soundness | Proven property (OOPSLA 2024): validated policy + conforming request ⇒ no evaluation error. Explicitly does NOT cover integer overflow, missing entities, non-strict extension misuse. |
| Silent no-op | Policy that errors at evaluation and is dropped from the decision without a log. |
| Meta-permission | Policy governing who may change the authorization system itself. |
| Entity synthesis | Constructing the entity store per request from durable state (space membership). |
| Policy-set version | Monotone counter incremented on every policy write; decisions tagged with it. |
| ReBAC | Relationship-based access control (Zanzibar lineage: tuples + computed relations). |

---

## YP-4: Theoretical Foundation

### THM-AUTHZ-DENY-BY-DEFAULT

`∀ req, ps`: `eval(ps, req) = none → Decide(req) = Deny`.

### THM-AUTHZ-VALIDATION-SOUNDNESS (the property Ferro currently cannot claim)

`∀ p : Policy, s : Schema, e : Entities, req : PARC`: `validate(p, s) = ok ∧ conforms(req, s) → evalEntity(e, p, req) ≠ evaluationError`. Cedar proves this for its validator (arXiv:`2407.01688`; OOPSLA 2024); Ferro must *invoke* it.

### THM-AUTHZ-SINGLE-LANGUAGE

`∀ req, d`: `d = EffectiveDecision(req) → ∃ ps. d = evalCedar(ps, req)`. Closes the Rust-bypass divergence gap.

### THM-AUTHZ-PATH-NO-WILDCARD

Path is display-only, never an authz key: `∀ s₁ ≠ s₂, p`: `hasRole(p, Editor, s₂) ∧ ¬pathEq(s₁, s₂) → ¬authorizedViaPathPrefix(p, s₁)`. Guards the `/_spaces/*` over-match class.

### THM-AUTHZ-HOME-RULE

The own-home grant is expressible in policy: `∀ p`: `authorize(p, Read, Document(ownHome(p))) = Allow ∧ ∃ ps. authorize = evalCedar(ps, requestOf(p, ownHome(p)))`.

### THM-AUTHZ-NO-ESCALATION

`∀ p, q, res`: `changePermissions(p, res) ∧ ¬changePermissions(q, res) → ¬(q ∈ principalsOf(p))`; no principal can escalate beyond itself via the permission API.

---

## YP-5: Algorithm Specification

### ALG-SCHEMAVAL-001: Validate-on-Write

```
startup:  schema := Schema::from_str(SCHEMA_JSON)          # once
          auto := generate_auto_policies()
          validate(auto, schema) or FAIL STARTUP            # never continue silently
write:    on policy PUT/DELETE: validate(new_set, schema) or reject(400, errors)
          policy_set_version++
eval:     Cedar eval against schema-typed entities; errors => deny + WARN log
          (schema-typed entities make the silent-no-op class detectable at runtime too)
```

### ALG-ENTITIES-001: Entity Synthesis

```
entities per request:
  User::{sub, groups: Set<String>}                       # groups typed, source-validated
  Space::{id, managers: Set<User>, editors: Set<User>}
  Document::{id, space_id, path, owner}
  Application::"ferro"
note: synthesis is per-request (non-persistent); this also dodges the Cedar
      issue #2341 ancestor-serialization round-trip bug by construction
```

### ALG-METAPERM-001: Meta-Permission Policies

```
permit(admin, "managePolicies");
permit(manager(s), "changePermissions", resource.space == s) unless target_level > own_level;
permit(owner, "changePermissions", resource.owner == principal);
home rule: permit(u, action, resource.path startswith "/users/{u.sub}")   # IN CEDAR now
then delete the Rust bypass in the middleware; CI test asserts bypass-set == policy-set
```

---

## YP-6: Test Vector Specification

See `test_vectors/test_vectors_platform.toml`; sections `authz_*`.

Critical vectors: typo'd policy write → rejected at write time; schema change invalidating a policy → startup failure (not silent); `/spaces/a` vs `/spaces/ab` vs `/spaces/a/b` authorization matrix; manager-cannot-escalate; home-rule via Cedar equals old Rust-bypass behaviour (differential test); policy-set version monotonicity.

---

## YP-7: Domain Constraints

See `domain_constraints/domain_constraints_platform.toml`.

Summary: every policy write MUST pass schema validation (`DC-AUTHZ-001`); startup MUST fail on invalid generated policies (`DC-AUTHZ-002`); no authorization decision MAY consult raw path strings (`DC-AUTHZ-003`); the home bypass MUST live in Cedar, not Rust (`DC-AUTHZ-004`); `context.groups` MUST be schema-typed and source-validated (`DC-AUTHZ-005`); policy set MUST be versioned (`DC-AUTHZ-006`).

---

## YP-8: Bibliography

1. Pang, R., Han, R., Wellons, J., Wilcox, E., Zdonik, S. "Zanzibar: Google's Consistent, Global Authorization System." *USENIX ATC 2019*. <10 ms p95, >99.999% availability, trillions of tuples. The ReBAC *model*; overkill at 50 users but the vocabulary source. [VERIFIED]
2. Li, C., et al. "Cedar: A new language for expressive, fast, safe, and analyzable authorization." *OOPSLA 2024*. DOI `10.1145/3649835`; arXiv:`2403.04651`. Lean-modeled; outperforms OpenFGA/Rego. [VERIFIED]
3. Farkas, A., et al. "Verification-Guided Development of Cedar." arXiv:`2407.01688`. [VERIFIED]
4. `cedar-policy/cedar-spec`; Lean formalization + differential randomized testing Lean↔Rust. [VERIFIED]
5. Cedar documentation: validation against schema; language version 4.5; *"Cedar doesn't use the schema when evaluating … it uses the schema to validate the policies only when you create or update them"*; evaluation errors ⇒ policy silently ignored; schema changes can invalidate previously valid policies; validation is deliberately a separate API. [VERIFIED]
6. Cedar GitHub issue #2341: entity JSON serialization emits transitive ancestors (round-trip lossy); dodged by per-request synthesis. [VERIFIED]
7. SpiceDB consistency docs; four modes: fully-consistent / min-freshness / at-exactly-snapshot / latency-optimized. Vocabulary for P4. [VERIFIED]
8. OpenFGA modeling guide; worked example is a simplified Google Drive model (`parent/owner/editor/viewer`, `can_view/can_write/...`, schema 1.1). [VERIFIED]
9. AWS Verified Permissions; managed Cedar service (existence/context only). [VERIFIED]
10. **Negative results:** "Consistency in Zanzibar" (OSDI 2021) does **not** exist (full proceedings checked); do not cite; Permify/Cerbos details unverified this session ([UNCERTAIN]; verify before citing in downstream docs). [VERIFIED ABSENCE]
