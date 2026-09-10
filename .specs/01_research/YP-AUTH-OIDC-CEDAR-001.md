# YP-AUTH-OIDC-CEDAR-001: Authentication (OIDC) & Authorization (Cedar Policy Language)

| Field        | Value                                      |
|--------------|--------------------------------------------|
| Document ID  | YP-AUTH-OIDC-CEDAR-001                     |
| Domain       | Identity & Access Management               |
| Version      | 1.0                                        |
| Status       | Draft                                      |
| Author       | DeepThought (Research Agent, Phase 1)      |
| Date         | 2026-04-18                                 |
| Crate Ref    | `cedar-policy` v4.9.1 (Apache-2.0, AWS)    |
| Spec Ref     | OpenID Connect Core 1.0 (errata set 2)     |

---

## YP-2: Executive Summary

### Problem Statement

Ferro requires an authentication and authorization system that supports Single Sign-On (SSO) via standards-compliant OIDC providers (Keycloak, Authelia, Okta) and fine-grained, verifiable authorization via the Cedar Policy Language. The system must provide deny-by-default security semantics, sub-10ms policy evaluation, and hot-reloadable policy stores.

### OIDC Authentication Layer

OpenID Connect Core 1.0 provides an identity layer on top of OAuth 2.0, enabling Relying Parties (RPs) to verify End-User identity via JWT-based ID Tokens issued by an OpenID Provider (OP). Ferro acts as the RP, validating tokens issued by a configurable OP. The Authorization Code Flow (with PKCE for public clients) is the primary authentication mechanism, returning tokens exclusively from the Token Endpoint — never exposing tokens to the User Agent.

### Cedar Authorization Layer

AWS Cedar is a language and evaluation engine for attribute-based access control (ABAC) policies. Policies are declarative statements of the form `permit/principal/action/resource when condition` that are evaluated against a request tuple `(principal, action, resource, context)`. Cedar provides:

- **Deny-by-default**: Empty policy set produces `Deny` for all requests.
- **Forbid-overrides-permit**: Any satisfied `forbid` policy overrides all `permit` policies.
- **Formal verification**: Correctness properties proven in Lean theorem prover; differential testing against Rust implementation.
- **Schema validation**: Type-checking of policies against entity schemas prevents runtime errors.
- **No I/O or side effects**: Policies are pure expressions guaranteed to terminate.

### Security Properties

| Property                    | Mechanism                                           |
|-----------------------------|-----------------------------------------------------|
| Token integrity             | JWS signature verification against OP's JWKS        |
| Token freshness             | `exp`, `nbf`, `iat` claim validation                |
| CSRF prevention             | `nonce` claim binding request to response           |
| Replay prevention           | `jti` claim + short token TTL + nonce               |
| Authorization deny-by-default | Cedar returns `Deny` when no `permit` matches       |
| Forbid override             | Cedar `forbid` policies always produce `Deny`       |
| Policy validation           | Schema-based type checking before evaluation        |
| Input bounds                | Bounded policy/request sizes prevent memory exhaustion |

---

## YP-3: Nomenclature

| Term                  | Definition                                                                                                    |
|-----------------------|--------------------------------------------------------------------------------------------------------------|
| ID Token              | JWT containing claims about the authentication event (iss, sub, aud, exp, iat, nonce). Signed via JWS.       |
| Access Token          | Token (opaque or JWT) granting access to protected resources. NOT used by Ferro for authz decisions.         |
| Refresh Token         | Long-lived token used to obtain new Access/ID Tokens. Stored securely; never sent to the browser.            |
| OpenID Provider (OP)  | OAuth 2.0 Authorization Server that authenticates End-Users and issues ID Tokens (e.g., Keycloak, Okta).    |
| Relying Party (RP)    | OAuth 2.0 Client requiring End-User authentication (Ferro acts as the RP).                                   |
| JWKS                  | JSON Web Key Set — the OP's public key endpoint for signature verification.                                   |
| Principal             | Cedar entity representing the authenticated user making an authorization request.                             |
| Action                | Cedar entity representing the operation the principal wants to perform.                                       |
| Resource              | Cedar entity representing the target of the action.                                                          |
| Context               | Cedar record containing request-specific data (IP, time, MFA status).                                        |
| Policy                | Cedar statement declaring which principals are permitted or forbidden to perform actions on resources.        |
| Policy Set            | Collection of all Cedar policies evaluated for an authorization decision.                                     |
| Entity                | Cedar object with a type, unique identifier (UID), and attributes. Principals, actions, and resources are entities. |
| Entity UID            | Unique identifier for an entity: `EntityType::"identifier"` (e.g., `User::"a1b2c3d4-..."`).                  |
| Template              | Cedar policy with placeholders (slots) for principal/resource, used to create template-linked policies.       |
| Schema                | Declaration of entity types, attributes, and actions; used to validate policies before evaluation.           |
| Authorization Request | Cedar tuple `(principal, action, resource, context)` submitted for authorization decision.                  |
| Decision              | Cedar evaluation result: `Allow` or `Deny`.                                                                   |
| Determining Policies  | The set of policies whose evaluation determined the final decision.                                           |
| PKCE                  | Proof Key for Code Exchange — extension to Authorization Code Flow for public clients (RFC 7636).            |
| Claim                 | Piece of information asserted about an Entity in a JWT (e.g., `sub`, `iss`, `aud`).                          |
| Scope                 | OAuth 2.0 mechanism requesting specific access levels (OIDC requires `openid` scope).                        |
| Authorization Code    | Short-lived, single-use code exchanged at the Token Endpoint for tokens.                                     |

---

## YP-4: Theoretical Foundation

### DEF-OIDC-001: OpenID Connect — Formal Model

OpenID Connect (OIDC) is an identity protocol layer on OAuth 2.0 [RFC 6749] that enables End-User authentication via a three-token model.

**Three-Token Model:**

```
Token_Triplet = (id_token: JWT, access_token: Opaque|JWT, refresh_token?: Opaque)
```

1. **ID Token** (`id_token`): A JWS-signed JWT containing authentication claims:
   - `iss` (REQUIRED): Issuer Identifier — HTTPS URL of the OP.
   - `sub` (REQUIRED): Subject Identifier — unique, never-reassigned ID for the End-User within the OP.
   - `aud` (REQUIRED): Audience — must contain the RP's `client_id`.
   - `exp` (REQUIRED): Expiration time — epoch seconds; token MUST be rejected after this time.
   - `iat` (REQUIRED): Issued-at time — epoch seconds.
   - `auth_time` (CONDITIONAL): Time of End-User authentication.
   - `nonce` (RECOMMENDED): Associates client session with ID Token; mitigates replay attacks.
   - `azp` (OPTIONAL): Authorized party — the client to which the token was issued.

2. **Access Token** (`access_token`): Opaque or JWT token granting access to the UserInfo Endpoint and other protected resources. NOT used by Ferro for authorization decisions (Cedar provides that).

3. **Refresh Token** (`refresh_token`): Opaque, long-lived token used to obtain new ID/Access Tokens without re-authentication. MUST be stored server-side (HTTP-only, Secure, SameSite cookie).

**Authorization Code Flow** (server-side, confidential clients):

```
RP                    OP
 |-- AuthN Request (code) -------->|
|                                  |-- Authenticate End-User
|                                  |-- Obtain consent
|<-- Authorization Code ------------|
|-- Token Request (code) --------->|
|                                  |-- Verify client credentials
|<-- ID Token + Access Token -------|
|-- UserInfo Request (AT) -------->|
|<-- UserInfo Claims ---------------|
```

All tokens are returned from the Token Endpoint (never exposed to the User Agent). The RP validates the ID Token signature and claims server-side.

**PKCE** (Proof Key for Code Exchange, RFC 7636):

For public clients (SPAs, mobile apps), PKCE prevents authorization code interception:

```
client generates: code_verifier = random(43..128 chars, [A-Za-z0-9-._~])
client computes:  code_challenge = BASE64URL(SHA256(code_verifier))
client sends:     code_challenge + code_challenge_method=S256 in AuthN Request
server sends:     code_challenge in Token Request
OP verifies:      SHA256(code_verifier) == code_challenge
```

---

### THM-OIDC-SEC-001: Token Validation Security

**Claim: OIDC token validation provides authenticity, integrity, and freshness guarantees.**

| Threat              | Mitigation                                     | Mechanism                                  |
|---------------------|------------------------------------------------|--------------------------------------------|
| Token forgery       | Signature verification                          | JWS signature checked against OP's JWKS    |
| Token replay        | Expiration + nonce + jti                        | `exp` claim rejects stale tokens; `nonce` binds token to session |
| Token substitution  | Audience validation                             | `aud` must contain RP's `client_id`        |
| Issuer impersonation| Issuer validation                               | `iss` must match expected OP URL exactly    |
| CSRF                | nonce + state parameter                         | `nonce` in ID Token matches session value  |
| Timing attacks      | Constant-time comparison                        | HMAC verification uses constant-time `==`  |
| Key compromise      | Key rotation                                    | JWKS `kid` header selects correct key; OP rotates keys via `jwks_uri` |

**Proof sketch:**

1. **Authenticity**: Given JWS signature verification passes with a key from the trusted JWKS, the token was signed by the OP (by definition of digital signature security under RSA/ECDSA).

2. **Integrity**: JWS signing covers the entire JWT payload. Any modification invalidates the signature.

3. **Freshness**: The `exp` claim ensures `now < exp` (with configurable clock skew leeway, typically 60s). The `nbf` claim ensures `now >= nbf`. Together they bound the valid lifetime of the token.

4. **Session binding**: The `nonce` claim is an opaque random value generated by the RP, passed to the OP in the Authentication Request, and returned unmodified in the ID Token. Matching `nonce` values prove the token was issued in response to this specific authentication session.

---

### DEF-CEDAR-001: Cedar Policy Language — Formal Model

Cedar is a declarative policy language for attribute-based access control (ABAC). A Cedar policy is a statement of the form:

```
Policy ::= Effect "(" PrincipalConstraint "," ActionConstraint "," ResourceConstraint ")"
           (Condition)* ";"

Effect          ::= "permit" | "forbid"
PrincipalConstraint ::= PrincipalSlot | PrincipalExpr
ActionConstraint    ::= ActionExpr
ResourceConstraint  ::= ResourceSlot | ResourceExpr
Condition       ::= "when" "{" Expression "}" | "unless" "{" Expression "}"
```

**Authorization Model: ABAC (Attribute-Based Access Control)**

Cedar's ABAC model evaluates access decisions based on attributes of four components:

```
AuthorizationRequest = (principal: Entity, action: Entity, resource: Entity, context: Record)
```

Each `Entity` has:
- **Type**: e.g., `User`, `Photo`, `Action`, `Group`
- **UID**: unique identifier, e.g., `User::"a1b2c3d4-e5f6-a1b2-c3d4-EXAMPLE11111"`
- **Attributes**: typed key-value pairs (e.g., `owner`, `department`, `tags`)
- **Parents**: hierarchical relationships for group membership (`in` operator)

**Entity Hierarchy:**

```
entity User::"alice" {
  parents = [Group::"admin", Group::"engineering"]
  department = "HardwareEngineering"
  jobLevel = 7
}
```

The `in` operator traverses the entity hierarchy transitively:
```
User::"alice" in Group::"admin"           => true
User::"alice" in Group::"engineering"      => true
```

**Policy Set Semantics:**

Given a policy set `PS = {p1, p2, ..., pn}` and a request `q`, the authorization decision `D` is computed as:

```
eval(q, pi) ∈ {true, false, error}  for each pi ∈ PS

forbids  = {pi ∈ PS | eval(q, pi) = true ∧ pi.effect = "forbid"}
permits  = {pi ∈ PS | eval(q, pi) = true ∧ pi.effect = "permit"}
errors   = {pi ∈ PS | eval(q, pi) = error}

D = Deny  if forbids ≠ ∅
D = Allow if permits ≠ ∅ ∧ forbids = ∅
D = Deny  otherwise
```

**Schema:**

A Cedar schema declares entity types, their attributes (with types), and the action hierarchy. The validator uses the schema to statically check policies for type errors, invalid entity references, and attribute misuse before they are evaluated at runtime.

```cedar
type User = {
  department: String,
  jobLevel: Long,
  active: Bool,
};

type Photo = {
  owner: User,
  tags: Set<String>,
  private: Bool,
};

action "viewPhoto", "editPhoto", "deletePhoto";
```

---

### THM-CEDAR-001: Cedar Evaluation Correctness

**Claim: Cedar evaluates policies correctly with three key properties: default-deny, forbid-overrides-permit, and skip-on-error.**

**Property 1 — Default-Deny:**

> For any request `q` and empty policy set `PS = ∅`, `D(q, ∅) = Deny`.

*Proof*: With `permits = ∅` and `forbids = ∅`, the decision falls through to the third case: `D = Deny`. ∎

**Property 2 — Forbid-Overrides-Permit:**

> For any request `q` and policy set `PS`, if `forbids ≠ ∅` then `D(q, PS) = Deny`, regardless of `permits`.

*Proof*: The first case in the decision algorithm checks `forbids ≠ ∅` before `permits`. If any forbid policy evaluates to `true`, the decision is immediately `Deny`. ∎

**Property 3 — Skip-on-Error:**

> Policies that evaluate to `error` do not contribute to `permits` or `forbids`. They are reported in diagnostics but do not affect the decision.

*Proof*: The sets `permits` and `forbids` are defined as policies evaluating to `true`, not `error`. Error policies are tracked separately in the `errors` set and included in response diagnostics. ∎

**Complexity Analysis:**

Cedar policy evaluation is bounded by:

```
Time(q, PS) = O(|PS| × max_conditions × max_attribute_lookups)
```

Where:
- `|PS|` = number of policies in the policy set
- `max_conditions` = maximum number of `when`/`unless` clauses per policy
- `max_attribute_lookups` = maximum entity attribute traversals per condition evaluation

For typical workloads (100 policies, 2-3 conditions per policy, 1-2 attribute lookups per condition), evaluation completes in microseconds. Cedar's formal model guarantees termination for all bounded inputs.

**Differential Testing:**

Cedar's correctness is verified through:
1. **Lean formal model**: Executable semantics with proven properties.
2. **Rust production implementation**: Safe Rust only (memory safety, type safety, data-race safety).
3. **Differential testing**: Automated comparison of Lean and Rust evaluation results on millions of random inputs.

---

## YP-5: Algorithm Specification

### ALG-OIDC-VALIDATE-001: Token Validation

**Input**: HTTP `Authorization: Bearer <token>` header value.

**Output**: Validated principal identity (`sub` claim value + extracted attributes) or rejection error.

**Algorithm:**

```
function validate_token(token_string: String, jwks: JWKS, config: OIDCConfig) -> Result<Principal, TokenError>
    // Step 1: Parse JWT structure
    parts = split(token_string, ".")
    require(parts.length == 3, InvalidTokenFormat)
    header = base64url_decode(parts[0])
    payload = base64url_decode(parts[1])
    signature = base64url_decode(parts[2])

    // Step 2: Select verification key from JWKS
    kid = header["kid"]
    alg = header["alg"]
    require(alg in config.allowed_algorithms, AlgorithmNotAllowed)
    key = jwks.get_key(kid)
    require(key != null, KeyNotFound)

    // Step 3: Verify JWS signature (constant-time comparison)
    verified = verify_signature(
        signing_input = parts[0] + "." + parts[1],
        signature = signature,
        key = key,
        algorithm = alg
    )
    require(verified, SignatureInvalid)

    // Step 4: Parse claims
    claims = json_parse(payload)

    // Step 5: Validate issuer
    require(claims["iss"] == config.expected_issuer, InvalidIssuer)

    // Step 6: Validate audience
    require(claims["aud"] contains config.client_id, InvalidAudience)

    // Step 7: Validate expiration
    now = current_time_seconds()
    require(now < claims["exp"] + config.clock_skew, TokenExpired)

    // Step 8: Validate not-before (if present)
    if claims contains "nbf":
        require(now >= claims["nbf"] - config.clock_skew, TokenNotYetValid)

    // Step 9: Validate issued-at (if present)
    if claims contains "iat":
        require(claims["iat"] <= now + config.clock_skew, TokenIssuedInFuture)

    // Step 10: Validate nonce (if present and nonce expected)
    if claims contains "nonce" && config.expected_nonce != null:
        require(claims["nonce"] == config.expected_nonce, NonceMismatch)

    // Step 11: Extract principal identity
    sub = claims["sub"]
    require(sub != null && sub.length <= 255, InvalidSubject)

    return Principal {
        uid = CedarEntityUid(type = "User", id = sub),
        attributes = extract_claims_as_attributes(claims)
    }
```

**Complexity**: O(n) where n = token length (dominated by signature verification: RSA-2048 ~1ms, ECDSA P-256 ~0.1ms).

**Correctness Argument**:
- Steps 2-3 ensure token authenticity (signed by trusted OP).
- Steps 5-6 ensure token was issued for this RP.
- Steps 7-9 ensure token is temporally valid (freshness).
- Step 10 ensures session binding (prevents replay).
- Step 11 extracts a unique principal identifier.

---

### ALG-CEDAR-EVAL-001: Authorization Evaluation

**Input**: Authorization request `(principal, action, resource, context)`, policy set `PS`, entity store `E`.

**Output**: Authorization decision `Allow | Deny` with diagnostics.

**Algorithm:**

```
function authorize(request: Request, policy_set: PolicySet, entities: Entities) -> Response
    permits = []
    forbids = []
    errors  = []

    for policy in policy_set.all():
        result = evaluate_policy(request, policy, entities)

        if result == Error:
            errors.push((policy.id, result.details))
            continue  // skip-on-error

        if result == true:
            if policy.effect == "permit":
                permits.push(policy.id)
            else:  // forbid
                forbids.push(policy.id)

    // Decision logic
    if forbids is not empty:
        decision = Deny
        determining = forbids
    else if permits is not empty:
        decision = Allow
        determining = permits
    else:
        decision = Deny
        determining = []

    return Response {
        decision = decision,
        determining_policies = determining,
        error_policies = errors
    }

function evaluate_policy(request: Request, policy: Policy, entities: Entities) -> EvalResult
    // Bind request variables
    principal = request.principal
    action    = request.action
    resource  = request.resource
    context   = request.context

    // Step 1: Check scope constraints
    principal_match = eval_expr(policy.principal_constraint, {principal, action, resource, context}, entities)
    if principal_match != true:
        return false

    action_match = eval_expr(policy.action_constraint, {principal, action, resource, context}, entities)
    if action_match != true:
        return false

    resource_match = eval_expr(policy.resource_constraint, {principal, action, resource, context}, entities)
    if resource_match != true:
        return false

    // Step 2: Check conditions
    for condition in policy.conditions:
        result = eval_expr(condition.expression, {principal, action, resource, context}, entities)
        if result == Error:
            return Error(result.details)
        if condition.type == "when" && result != true:
            return false
        if condition.type == "unless" && result != false:
            return false

    return true
```

**Pseudocode — Rust Integration:**

```rust
use cedar_policy::{Authorizer, Decision, Entities, PolicySet, Request, Context, EntityUid};

fn authorize(
    principal: EntityUid,   // e.g., User::"alice"
    action: EntityUid,      // e.g., Action::"view"
    resource: EntityUid,    // e.g., File::"doc.pdf"
    context: Context,       // e.g., {"ip": "10.0.0.1", "mfa": true}
    policies: &PolicySet,
    entities: &Entities,
) -> Decision {
    let request = Request::new(principal, action, resource, context, None)
        .expect("valid request");

    let authorizer = Authorizer::new();
    let response = authorizer.is_authorized(&request, policies, entities);
    response.decision()
}
```

**Complexity**: O(|PS| × C × A) where:
- |PS| = number of policies
- C = average conditions per policy (typically 1-3)
- A = average attribute lookups per condition (typically 1-2)

For 100 policies with 2 conditions each and 1 attribute lookup: ~200 evaluations, completing in < 1ms.

**Correctness Argument**:
- The algorithm iterates over ALL policies (no early termination), ensuring that forbid policies are never missed.
- Forbid results are collected first in the decision logic (checked before permits), implementing forbid-overrides-permit.
- Empty permits set with empty forbids set produces Deny (default-deny).
- Error results are skipped but logged, implementing skip-on-error.

---

## YP-6: Test Vector Specification

Test vectors are defined in `test_vectors/test_vectors_auth.toml`. The following categories are covered:

### OIDC Token Validation Vectors

| Vector ID            | Category      | Description                                |
|----------------------|---------------|--------------------------------------------|
| TV-OIDC-001          | nominal       | Valid token with all required claims       |
| TV-OIDC-002          | boundary      | Expired token rejected                     |
| TV-OIDC-003          | adversarial   | Invalid signature rejected                 |
| TV-OIDC-004          | boundary      | Wrong issuer rejected                      |
| TV-OIDC-005          | boundary      | Wrong audience rejected                    |
| TV-OIDC-006          | boundary      | Missing `sub` claim rejected               |
| TV-OIDC-007          | adversarial   | Alg substitution attack rejected (`none`)  |
| TV-OIDC-008          | nominal       | Valid token with extra claims preserved    |

### Cedar Authorization Vectors

| Vector ID            | Category      | Description                                |
|----------------------|---------------|--------------------------------------------|
| TV-CEDAR-001         | nominal       | Permit policy match → Allow                |
| TV-CEDAR-002         | nominal       | Forbid policy match → Deny                 |
| TV-CEDAR-003         | boundary      | No matching policy → Deny (default-deny)   |
| TV-CEDAR-004         | nominal       | Permit + no forbid → Allow                 |
| TV-CEDAR-005         | nominal       | Permit + forbid on same request → Deny     |
| TV-CEDAR-006         | boundary      | Condition evaluates false → policy skipped |
| TV-CEDAR-007         | nominal       | Policy with attribute conditions           |
| TV-CEDAR-008         | boundary      | Entity hierarchy traversal (`in` operator) |
| TV-CEDAR-009         | nominal       | Context-based authorization (IP, MFA)      |
| TV-CEDAR-010         | adversarial   | Concurrent policy evaluation consistency   |
| TV-CEDAR-011         | boundary      | Policy evaluation with missing attributes  |
| TV-CEDAR-012         | nominal       | Template-linked policy evaluation          |

---

## YP-7: Domain Constraints

Domain constraints are defined in `domain_constraints/domain_constraints_auth.toml`. Summary of key constraints:

### Token Validation Constraints

| Constraint ID         | Category | Value               | Rationale                                       |
|-----------------------|----------|---------------------|-------------------------------------------------|
| DC-AUTH-TIMING-001    | timing   | < 5ms (p99)         | Token validation must not add perceptible latency to request path |
| DC-AUTH-ALG-001       | security | RS256, RS384, RS512, ES256, ES384, ES512 | Allowed JWS algorithms; `none` is forbidden |
| DC-AUTH-CLOCK-001     | security | 60s clock skew      | Max allowed clock skew between RP and OP         |
| DC-AUTH-NONCE-001     | security | required            | All authentication flows must use nonce          |
| DC-AUTH-TOKEN-TTL-001 | security | 600s (10 min)       | Max acceptable ID Token lifetime                 |
| DC-AUTH-JWKS-001      | reliability | 24h cache          | JWKS cache TTL; supports key rotation            |

### Cedar Policy Evaluation Constraints

| Constraint ID            | Category | Value                    | Rationale                                          |
|--------------------------|----------|--------------------------|----------------------------------------------------|
| DC-AUTH-CEDAR-TIMING-001 | timing   | < 10ms (p99) for 100 policies | Policy evaluation must not bottleneck request path |
| DC-AUTH-CEDAR-MAX-001    | scale    | 10,000 policies          | Maximum supported policy count in policy store      |
| DC-AUTH-CEDAR-HOT-001    | ops      | hot-reload               | Policy store changes take effect without restart    |
| DC-AUTH-CEDAR-SIZE-001   | security | 256 KiB per policy       | Maximum policy size to prevent memory exhaustion    |
| DC-AUTH-CEDAR-VALIDATE-001 | security | required             | All policies must pass schema validation before evaluation |

### Integration Constraints

| Constraint ID          | Category | Value                    | Rationale                                          |
|------------------------|----------|--------------------------|----------------------------------------------------|
| DC-AUTH-END-END-001    | timing   | < 15ms (p99)             | Combined OIDC validate + Cedar evaluate latency     |
| DC-AUTH-PRINCIPAL-001  | identity | UUID-based entity UIDs   | Must use non-reusable, non-recyclable identifiers   |
| DC-AUTH-LOG-001        | audit    | all decisions logged     | Every Allow/Deny decision logged with determining policies |
| DC-AUTH-ERROR-001      | security | deny-on-validation-failure | If OIDC validation fails, return 401; if Cedar errors, return 403 |

---

## YP-8: Bibliography

1. **OpenID Connect Core 1.0** (incorporating errata set 2). N. Sakimura, J. Bradley, M. Jones, B. de Medeiros, C. Mortimore. December 15, 2023. https://openid.net/specs/openid-connect-core-1_0.html

2. **OAuth 2.0 Authorization Framework** (RFC 6749). D. Hardt. October 2012. https://tools.ietf.org/html/rfc6749

3. **OAuth 2.0 Bearer Token Usage** (RFC 6750). M. Jones, D. Hardt. October 2012. https://tools.ietf.org/html/rfc6750

4. **JSON Web Token (JWT)** (RFC 7519). M. Jones, J. Bradley, N. Sakimura. May 2015. https://tools.ietf.org/html/rfc7519

5. **JSON Web Signature (JWS)** (RFC 7515). M. Jones, J. Bradley, N. Sakimura. May 2015. https://tools.ietf.org/html/rfc7515

6. **Proof Key for Code Exchange by OAuth Public Clients** (RFC 7636). N. Sakimura, J. Bradley. September 2015. https://tools.ietf.org/html/rfc7636

7. **Cedar Policy Language Reference Guide**. AWS Cedar Team. https://docs.cedarpolicy.com

8. **cedar-policy crate v4.9.1**. AWS. https://docs.rs/cedar-policy/4.9.1/cedar_policy/

9. **How we built Cedar with automated reasoning and differential testing**. Amazon Science Blog. https://www.amazon.science/blog/how-we-built-cedar-with-automated-reasoning-and-differential-testing

10. **Cedar Security Best Practices**. https://docs.cedarpolicy.com/other/security.html

11. **OpenID Connect Discovery 1.0** (incorporating errata set 1). N. Sakimura, J. Bradley, M. Jones, E. Jay. December 2023. https://openid.net/specs/openid-connect-discovery-1_0.html

12. **JSON Web Key (JWK)** (RFC 7517). M. Jones. May 2015. https://tools.ietf.org/html/rfc7517

13. **ISO/IEC 29115:2013** — Entity authentication assurance framework. https://www.iso.org/standard/45138.html
