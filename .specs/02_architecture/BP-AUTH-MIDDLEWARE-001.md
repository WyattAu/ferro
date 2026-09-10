# BP-AUTH-MIDDLEWARE-001: Authentication & Authorization Middleware

| Field         | Value                                              |
|---------------|----------------------------------------------------|
| Document ID   | BP-AUTH-MIDDLEWARE-001                             |
| Domain        | Identity & Access Management                       |
| Version       | 1.0                                                |
| Status        | Draft                                              |
| Author        | Construct (Systems Architect, Phase 2)             |
| Date          | 2026-04-18                                         |
| Supersedes    | YP-AUTH-OIDC-CEDAR-001                             |
| Compliance    | IEEE 1016-2009 (Software Design Descriptions)      |

---

## BP-1: Design Overview

### Purpose

Ferro's authentication and authorization subsystem provides a two-layer middleware pipeline for every inbound request:

1. **Authentication** (who are you?) -- OIDC ID Token validation via JWS signature verification against a configured OpenID Provider's JWKS.
2. **Authorization** (what can you do?) -- Cedar ABAC policy evaluation against a principal/action/resource/context tuple.

The subsystem is implemented as composable Axum middleware layers, ensuring that no request reaches a handler without passing both gates.

### C4 Context Diagram

```
┌──────────┐       ┌───────────────────────────────────────────────────┐
│          │       │                    Ferro System                   │
│  Client  │──────>│  ┌─────────────────────────────────────────────┐  │
│(rclone,  │ HTTPS │  │              Axum Router                     │  │
│ WOPI,    │       │  │                                             │  │
│ WebDAV,  │       │  │  ┌───────────────┐  ┌─────────────────────┐  │  │
│ Browser) │       │  │  │  Auth Layer   │  │   Cedar Layer       │  │  │
│          │       │  │  │  (OidcValidator)──>(CedarAuthorizer)    │  │  │
│          │       │  │  └───────┬───────┘  └──────────┬──────────┘  │  │
│          │       │  │          │                     │             │  │
│          │       │  │          ▼                     ▼             │  │
│          │       │  │  ┌───────────────────────────────────────┐  │  │
│          │       │  │  │           Route Handler               │  │  │
│          │       │  │  └───────────────────────────────────────┘  │  │
└──────────┘       │  └─────────────────────────────────────────────┘  │
                   └───────────────────────────────────────────────────┘
                                          │
                           ┌──────────────┼──────────────┐
                           ▼              ▼              ▼
                    ┌────────────┐ ┌────────────┐ ┌────────────┐
                    │ OIDC Provider│ │ PolicyStore│ │ EntityStore│
                    │ (Keycloak,  │ │ (DB/File)  │ │ (App DB)   │
                    │  Okta)      │ │            │ │            │
                    └────────────┘ └────────────┘ └────────────┘
```

### Design Principles

- **Deny-by-default**: Unauthenticated requests return 401; authenticated-but-unauthorized requests return 403.
- **Separation of concerns**: Authentication and authorization are independent middleware layers with distinct error semantics.
- **Stateless evaluation**: The Cedar authorizer holds no mutable request state; all context is passed per-request.
- **Hot-reload**: Policy changes take effect without restart via atomic swap of the active policy set.

---

## BP-2: Design Decomposition

### Component Inventory

| Component ID  | Name             | Responsibility                                       |
|---------------|------------------|------------------------------------------------------|
| COMP-AUTH-001 | OidcValidator    | JWT parsing, JWS signature verification, claim validation, JWKS caching |
| COMP-CEDAR-002| CedarAuthorizer  | Policy set management, authorization evaluation, decision logging |
| COMP-SESSION-003| SessionManager  | Session creation, idle timeout enforcement, refresh token rotation, revocation |
| COMP-POLICY-004| PolicyStore     | CRUD for Cedar policies, schema validation, atomic hot-reload, admin API |

### Dependency Graph

```
COMP-AUTH-001 (OidcValidator)
├── jsonwebtoken   -- JWT decode/verify
├── reqwest        -- JWKS HTTP fetch
└── COMP-SESSION-003 (nonce binding)

COMP-CEDAR-002 (CedarAuthorizer)
├── cedar-policy v4.9.1  -- policy parse/validate/evaluate
├── COMP-POLICY-004 (PolicyStore)
└── EntityStore (application DB entities)

COMP-SESSION-003 (SessionManager)
├── redis or in-memory  -- session store
├── COMP-AUTH-001 (token validation on refresh)
└── Tower service trait  -- Axum middleware integration

COMP-POLICY-004 (PolicyStore)
├── cedar-policy v4.9.1  -- policy parsing + schema validation
├── sqlx or sea-orm      -- database persistence (when DB-backed)
├── notify               -- filesystem watch (when file-backed)
└── serde                -- policy serialization
```

---

## BP-3: Design Rationale

### ADR-004: Cedar as Authorization Engine

**Status**: Accepted

**Context**: Ferro requires fine-grained, attribute-based authorization for file operations across multiple principals with varying access levels. Options considered: custom RBAC engine, Casbin, OPA (Rego), Cedar.

**Decision**: Use AWS Cedar Policy Language.

**Rationale**:
- Cedar provides *formal verification* of evaluation semantics (proven in Lean 4).
- *Deny-by-default* and *forbid-overrides-permit* are guaranteed by the evaluation algorithm.
- *No I/O or side effects* in policy evaluation -- pure functions guaranteed to terminate.
- *Differential testing* between Lean formal model and Rust implementation provides high assurance.
- Native Rust crate (`cedar-policy` v4.9.1) with zero unsafe code.
- Schema validation catches policy errors at load time, not at evaluation time.
- ABAC model supports Ferro's requirements for attribute-based conditions (resource tags, owner, path).

**Consequences**: Adds a dependency on `cedar-policy`. Policy authors must learn Cedar syntax. Evaluation is linear in policy count.

### ADR-005: OIDC with PKCE for All Clients

**Status**: Accepted

**Context**: Ferro clients include web browsers (SPAs), desktop apps (Tauri), and programmatic clients (rclone). Authentication must be secure across all client types.

**Decision**: Use OIDC Authorization Code Flow with PKCE (S256) for all client types.

**Rationale**:
- Authorization Code Flow returns tokens from the Token Endpoint, never exposing them to the User Agent.
- PKCE (RFC 7636) prevents authorization code interception attacks for public clients (SPAs, desktop).
- S256 challenge method is mandatory (plain method is not supported).
- OIDC ID Tokens provide standardized claims (`sub`, `email`, `groups`) that map directly to Cedar principal attributes.
- Any OIDC-compliant provider (Keycloak, Authelia, Okta) can be used without custom integration.

**Consequences**: Requires a browser-based redirect for initial authentication. Refresh tokens must be stored server-side.

### Rationale: Separate Auth Middleware from Handlers

Authentication and authorization logic is implemented as Axum middleware layers rather than inline handler code because:

1. **Uniform enforcement**: Middleware applies to all routes, preventing accidental bypass.
2. **Composability**: Layers can be independently tested, replaced, or reordered.
3. **Separation of concerns**: Handlers focus on business logic; middleware handles security.
4. **Early rejection**: Invalid requests are rejected before reaching handler code, reducing resource consumption.

---

## BP-4: Traceability

### Yellow Paper Algorithm Mapping

| Blue Paper Component | Yellow Paper Reference | Algorithm / Theorem |
|----------------------|------------------------|---------------------|
| COMP-AUTH-001        | YP-AUTH-OIDC-CEDAR-001 | ALG-OIDC-VALIDATE-001 (Token Validation) |
| COMP-CEDAR-002       | YP-AUTH-OIDC-CEDAR-001 | ALG-CEDAR-EVAL-001 (Authorization Evaluation) |
| COMP-CEDAR-002       | YP-AUTH-OIDC-CEDAR-001 | THM-CEDAR-001 (Default-Deny, Forbid-Overrides-Permit) |
| COMP-AUTH-001        | YP-AUTH-OIDC-CEDAR-001 | THM-OIDC-SEC-001 (Token Validation Security) |

### Requirements Mapping

| Requirement | Components | Verification |
|-------------|------------|--------------|
| REQ-AUTH-001 (OIDC Authentication) | COMP-AUTH-001, COMP-SESSION-003 | Full Authorization Code + PKCE flow; token validation per ALG-OIDC-VALIDATE-001 |
| REQ-AUTH-002 (Cedar Policy Engine) | COMP-CEDAR-002, COMP-POLICY-004 | Evaluation per ALG-CEDAR-EVAL-001; deny-by-default per THM-CEDAR-001 |
| REQ-AUTH-003 (Fine-Grained ABAC) | COMP-CEDAR-002 | Attribute-based conditions in Cedar policies; OIDC claims mapped to principal attributes |
| REQ-AUTH-004 (Session Management) | COMP-SESSION-003 | Idle timeout, absolute lifetime, refresh rotation, revocation |

### Test Vector Mapping

| Test Vector Category | Covered By | Verification Method |
|----------------------|------------|---------------------|
| TV-OIDC-001..008     | COMP-AUTH-001 | Unit tests: validate_token() against each vector |
| TV-CEDAR-001..012    | COMP-CEDAR-002 | Unit tests: is_authorized() against each vector |

### Domain Constraint Mapping

| Constraint | Enforcing Component |
|------------|-------------------|
| DC-AUTH-TIMING-001 (p99 < 5ms) | COMP-AUTH-001 |
| DC-AUTH-ALG-001 (algorithm allowlist) | COMP-AUTH-001 |
| DC-AUTH-CLOCK-001 (60s skew) | COMP-AUTH-001 |
| DC-AUTH-NONCE-001 (required) | COMP-AUTH-001, COMP-SESSION-003 |
| DC-AUTH-TOKEN-TTL-001 (600s max) | COMP-AUTH-001 |
| DC-AUTH-JWKS-001 (24h cache) | COMP-AUTH-001 |
| DC-AUTH-CEDAR-TIMING-001 (p99 < 10ms) | COMP-CEDAR-002 |
| DC-AUTH-CEDAR-MAX-001 (10K policies) | COMP-POLICY-004 |
| DC-AUTH-CEDAR-HOT-001 (hot-reload) | COMP-POLICY-004 |
| DC-AUTH-CEDAR-SIZE-001 (256 KiB max) | COMP-POLICY-004 |
| DC-AUTH-CEDAR-VALIDATE-001 (schema validation) | COMP-POLICY-004 |
| DC-AUTH-END-END-001 (p99 < 15ms) | Middleware chain |
| DC-AUTH-PRINCIPAL-001 (UUID-based UIDs) | COMP-AUTH-001 |
| DC-AUTH-LOG-001 (audit logging) | COMP-CEDAR-002 |
| DC-AUTH-ERROR-001 (error mapping) | Middleware chain |
| DC-AUTH-PRINCIPAL-CACHE-001 | COMP-CEDAR-002 |
| DC-AUTH-ENTITY-STORE-001 | COMP-CEDAR-002 |

---

## BP-5: Interface Design

### IF-AUTH-001: OidcValidator

```rust
/// OIDC token validator with JWKS caching.
pub trait OidcValidator: Send + Sync {
    /// Validate a bearer token string.
    /// Returns Ok(Claims) on success, Err(TokenError) on any validation failure.
    /// Implements ALG-OIDC-VALIDATE-001.
    fn validate_token(&self, token: &str) -> Result<Claims, TokenError>;

    /// Force-refresh the JWKS from the OP's jwks_uri.
    /// Called on cache expiry or when a token references an unknown `kid`.
    fn refresh_jwks(&self) -> Result<(), JwksError>;

    /// Fetch and cache the OIDC Discovery document.
    /// Returns issuer, authorization_endpoint, token_endpoint, jwks_uri, etc.
    fn get_discovery_document(&self) -> Result<DiscoveryDoc, DiscoveryError>;
}
```

**Error types:**

```rust
pub enum TokenError {
    InvalidTokenFormat,
    AlgorithmNotAllowed(String),
    KeyNotFound(String),
    SignatureInvalid,
    InvalidIssuer { expected: String, actual: String },
    InvalidAudience { expected: String, actual: String },
    TokenExpired,
    TokenNotYetValid,
    TokenIssuedInFuture,
    NonceMismatch,
    InvalidSubject,
    TokenTtlExceeded { max_ttl: u64, actual_ttl: u64 },
}
```

### IF-CEDAR-001: CedarAuthorizer

```rust
/// Cedar policy-based authorization engine.
pub trait CedarAuthorizer: Send + Sync {
    /// Evaluate an authorization request against the active policy set.
    /// Implements ALG-CEDAR-EVAL-001.
    fn is_authorized(
        &self,
        principal: EntityUid,
        action: EntityUid,
        resource: EntityUid,
        context: Context,
    ) -> Result<AuthResponse, EvalError>;

    /// Replace the active policy set atomically.
    /// All policies are validated against the schema before activation.
    fn load_policies(&self, policies: Vec<String>) -> Result<(), PolicyLoadError>;

    /// Add or update an entity in the entity store.
    fn add_entity(&self, entity: Entity) -> Result<(), EntityStoreError>;

    /// Remove an entity from the entity store.
    fn remove_entity(&self, uid: &EntityUid) -> Result<(), EntityStoreError>;
}
```

**Return type:**

```rust
pub struct AuthResponse {
    pub decision: AuthDecision,
    pub determining_policies: Vec<PolicyId>,
    pub error_policies: Vec<(PolicyId, String)>,
}

pub enum AuthDecision {
    Allow,
    Deny,
}
```

### IF-SESSION-001: SessionManager

```rust
/// User session lifecycle management.
pub trait SessionManager: Send + Sync {
    /// Create a new session from validated OIDC tokens.
    fn create_session(&self, tokens: TokenSet, nonce: String) -> Result<Session, SessionError>;

    /// Validate and refresh an existing session's tokens.
    fn refresh_session(&self, session_id: &str) -> Result<Session, SessionError>;

    /// Revoke a session immediately (logout).
    fn revoke_session(&self, session_id: &str) -> Result<(), SessionError>;

    /// Look up a session by its ID. Returns None if expired or revoked.
    fn get_session(&self, session_id: &str) -> Result<Option<Session>, SessionError>;
}
```

### IF-POLICY-001: PolicyStore

```rust
/// Persistent storage and management of Cedar policies.
pub trait PolicyStore: Send + Sync {
    /// List all policies.
    fn list_policies(&self) -> Result<Vec<CedarPolicy>, PolicyStoreError>;

    /// Get a policy by ID.
    fn get_policy(&self, id: &str) -> Result<Option<CedarPolicy>, PolicyStoreError>;

    /// Create a new policy. Validates against schema before persisting.
    fn create_policy(&self, policy: CreatePolicyRequest) -> Result<CedarPolicy, PolicyStoreError>;

    /// Update an existing policy. Old policy remains active until new version passes validation.
    fn update_policy(&self, id: &str, policy: UpdatePolicyRequest) -> Result<CedarPolicy, PolicyStoreError>;

    /// Delete a policy by ID.
    fn delete_policy(&self, id: &str) -> Result<(), PolicyStoreError>;

    /// Get the current active PolicySet for evaluation.
    fn active_policy_set(&self) -> Arc<PolicySet>;
}
```

---

## BP-6: Data Design

### Claims (OIDC Token)

```
Claims {
    sub:    String           -- REQUIRED, maps to User::"<sub>" entity UID
    iss:    String           -- REQUIRED, validated against config
    aud:    String or [String] -- REQUIRED, must contain client_id
    exp:    u64              -- REQUIRED, epoch seconds
    iat:    u64              -- REQUIRED, epoch seconds
    nonce:  String           -- RECOMMENDED, validated against session
    email:  Option<String>   -- mapped to principal attribute
    name:   Option<String>   -- mapped to principal attribute
    groups: Option<Vec<String>> -- mapped to principal attributes + Group parents
    auth_time: Option<u64>   -- time of user authentication at OP
    azp:    Option<String>   -- authorized party
}
```

### AuthResponse

```
AuthResponse {
    decision:              AuthDecision     -- Allow | Deny
    determining_policies:  Vec<PolicyId>    -- policies that determined the decision
    error_policies:        Vec<(PolicyId, String)> -- policies that errored (skip-on-error)
}
```

### CedarPolicy (Persistent)

```
CedarPolicy {
    id:          String       -- unique policy identifier (UUID or admin-assigned)
    description: String       -- human-readable description
    policy_text: String       -- raw Cedar policy source
    effect:      Effect       -- permit | forbid (extracted during parsing)
    created_at:  DateTime<Utc>
    updated_at:  DateTime<Utc>
    version:     u32          -- optimistic concurrency version
}
```

### PolicyStore Schema (SQL)

```sql
CREATE TABLE cedar_policies (
    id           TEXT PRIMARY KEY,
    description  TEXT NOT NULL,
    policy_text  TEXT NOT NULL CHECK (length(policy_text) <= 262144),
    effect       TEXT NOT NULL CHECK (effect IN ('permit', 'forbid')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    version      INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_cedar_policies_effect ON cedar_policies(effect);
CREATE INDEX idx_cedar_policies_updated ON cedar_policies(updated_at);
```

### Session

```
Session {
    id:              String          -- session identifier
    user_sub:        String          -- OIDC subject claim
    id_token:        EncryptedString -- encrypted at rest
    access_token:    EncryptedString -- encrypted at rest
    refresh_token:   Option<EncryptedString> -- encrypted at rest
    created_at:      DateTime<Utc>
    last_active_at:  DateTime<Utc>   -- updated on each request
    expires_at:      DateTime<Utc>   -- absolute lifetime
    idle_timeout:    Duration        -- configurable, default 30min
    nonce:           String          -- OIDC nonce bound to this session
}
```

---

## BP-7: Component Design

### Axum Middleware Chain

```
Request
  │
  ▼
┌──────────────────────────────────────────────┐
│  auth_layer (COMP-AUTH-001)                  │
│  1. Extract Bearer token from Authorization  │
│     header or session cookie                │
│  2. validate_token() -> Claims               │
│  3. On failure: return 401                   │
│  4. Insert Claims into request extensions    │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│  cedar_layer (COMP-CEDAR-002)                │
│  1. Extract route info (action, resource)    │
│     from request extensions                 │
│  2. Build Cedar principal from Claims        │
│  3. Load required entities from EntityStore  │
│  4. is_authorized() -> AuthResponse          │
│  5. On Deny: return 403                      │
│  6. On Allow: insert AuthResponse into       │
│     request extensions for audit logging     │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
              Route Handler
```

### JWKS Cache Invalidation Strategy

```
                    ┌───────────────────────────┐
                    │     JWKS Cache            │
                    │   (Arc<RwLock<JWKS>>)     │
                    │                           │
                    │  TTL: 24h (configurable)  │
                    │  Max keys: 10             │
                    └──────────┬────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
        TTL expired     Unknown kid      Explicit
        (background     (on-demand       refresh
         refresh)       fetch)           (admin API)
```

- **TTL-based**: Background task refreshes JWKS every 24h (DC-AUTH-JWKS-001).
- **On-demand**: When `validate_token()` encounters a `kid` not in the cache, it fetches fresh JWKS from the OP.
- **Explicit**: Admin API endpoint triggers immediate JWKS refresh.
- **HTTPS enforcement**: JWKS URI must use `https://` scheme (DC-AUTH-JWKS-002).

### Policy Hot-Reload Mechanism

```
                    ┌───────────────────────────────────┐
                    │    PolicyStore                     │
                    │                                   │
                    │  ┌─────────────┐                  │
                    │  │  DB / File  │                  │
                    │  └──────┬──────┘                  │
                    │         │                         │
                    │         ▼                         │
                    │  ┌─────────────────────┐          │
                    │  │  validate + parse   │          │
                    │  │  against schema     │          │
                    │  └──────┬──────────────┘          │
                    │         │                         │
                    │         ▼                         │
                    │  ┌─────────────────────┐          │
                    │  │  Arc<RwLock<PolicySet>>        │
                    │  │  (atomic swap)       │          │
                    │  └─────────────────────┘          │
                    └───────────────────────────────────┘

    DB-backed:  LISTEN/NOTIFY or polling interval
    File-backed: notify crate (inotify/kqueue) + debounced reload
```

- **Atomic swap**: The active `PolicySet` is behind `Arc<RwLock<PolicySet>>`. Updates construct a new `PolicySet`, validate it fully, then swap the pointer atomically.
- **Validation before activation**: Schema validation (DC-AUTH-CEDAR-VALIDATE-001) runs on the entire policy set before the swap. If validation fails, the old policy set remains active.
- **Rollback**: If the new policy set causes evaluation errors, the previous version can be restored from the policy store's version history.
- **Size enforcement**: Individual policies are rejected if they exceed 256 KiB (DC-AUTH-CEDAR-SIZE-001). The total policy count is capped at 10,000 (DC-AUTH-CEDAR-MAX-001).

---

## BP-8: Deployment Design

### OIDC Provider Configuration

```toml
[oidc]
issuer = "https://auth.ferro.local"
client_id = "ferro-client"
client_secret = "${OIDC_CLIENT_SECRET}"  # env var injection
redirect_uri = "https://ferro.local/auth/callback"
scopes = ["openid", "profile", "email"]
pkce_method = "S256"
clock_skew_seconds = 60
max_token_ttl_seconds = 600
jwks_cache_ttl_seconds = 86400
allowed_algorithms = ["RS256", "RS384", "RS512", "ES256", "ES384", "ES512"]
```

### Policy Store Configuration

```toml
[policy_store]
# Backend: "database" | "file"
backend = "database"

[policy_store.database]
url = "${DATABASE_URL}"
# Schema validation on every policy load
validate_on_load = true

[policy_store.file]
path = "/etc/ferro/policies/"
watch = true  # enable filesystem watch for hot-reload
poll_interval_ms = 5000  # fallback if inotify unavailable
```

### Session Configuration

```toml
[session]
idle_timeout_seconds = 1800      # 30 minutes
absolute_lifetime_seconds = 86400 # 24 hours
refresh_token_rotation = true
store_backend = "redis"          # "redis" | "memory"
redis_url = "${REDIS_URL}"
```

### Deployment Topology

```
┌─────────────┐     ┌─────────────┐
│  Load       │────>│  Ferro      │ (2+ replicas)
│  Balancer   │     │  Instance   │
└─────────────┘     └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ OIDC OP  │ │ Postgres │ │  Redis   │
        │(Keycloak)│ │ (policies│ │(sessions)│
        └──────────┘ │ + data)  │ └──────────┘
                    └──────────┘
```

- **Stateless auth**: OIDC validation and Cedar evaluation are stateless. Any replica can process any request.
- **Shared policy store**: All replicas read policies from the same database. Hot-reload propagates via LISTEN/NOTIFY.
- **Shared session store**: Redis provides consistent session state across replicas.

---

## BP-9: Formal Verification

### PROP-AUTH-001: Token Validation Security

**Property**: No forged or tampered OIDC token can be accepted as valid given a correctly provisioned JWKS.

**Formal statement**:
```
forall token: Token, jwks: JWKS, config: OIDCConfig,
    validate_token(token, jwks, config) = Ok(claims) =>
    exists key in jwks where verify_signature(token, key) = true
    AND claims.iss == config.expected_issuer
    AND config.client_id in claims.aud
    AND claims.exp > now - config.clock_skew
```

**Argument**: By construction of ALG-OIDC-VALIDATE-001, steps 2-3 verify the JWS signature against a key from the trusted JWKS. Steps 5-6 validate issuer and audience. Steps 7-9 validate temporal claims. Any violation causes immediate rejection. See THM-OIDC-SEC-001 in the Yellow Paper.

**Lean 4 proof**: See `proofs/proof_auth.lean` -- `theorem token_validation_soundness`.

### PROP-AUTH-002: Deny-by-Default

**Property**: An empty policy set produces `Deny` for any authorization request.

**Formal statement**:
```
forall request: Request,
    authorize(request, PolicySet.empty(), Entities.empty()).decision = Deny
```

**Argument**: Direct corollary of THM-CEDAR-001 Property 1. With `permits = {}` and `forbids = {}`, the decision algorithm falls through to the default case: `D = Deny`.

**Lean 4 proof**: See `proofs/proof_auth.lean` -- `theorem default_deny`.

### PROP-AUTH-003: Policy Evaluation Termination

**Property**: Cedar policy evaluation terminates for all bounded inputs.

**Formal statement**:
```
forall request: Request, policy_set: PolicySet, entities: Entities,
    |policy_set| <= 10000  AND
    forall p in policy_set, |p| <= 262144 AND
    |entities| <= MAX_ENTITIES AND
    forall e in entities, |e.attrs| <= MAX_ATTRS =>
    exists result: Response where authorize(request, policy_set, entities) = result
    (i.e., the function returns in finite time)
```

**Argument**: Cedar's evaluation semantics are defined over a finite AST with no recursion or unbounded loops. Entity attribute lookups are bounded by the entity store size. Policy conditions are finite conjunctions of boolean expressions over primitive types. Therefore, the evaluation function is a total function over bounded inputs.

**Lean 4 proof**: See `proofs/proof_auth.lean` -- `theorem evaluation_terminates`.

---

## BP-10: Security Considerations

### Threat Model

| Threat | Component | Mitigation |
|--------|-----------|------------|
| Token forgery | COMP-AUTH-001 | JWS signature verification against trusted JWKS |
| Algorithm substitution | COMP-AUTH-001 | Algorithm allowlist (DC-AUTH-ALG-001); `none` forbidden |
| Token replay | COMP-AUTH-001 | `exp` + `nonce` + short TTL (DC-AUTH-TOKEN-TTL-001) |
| Issuer confusion | COMP-AUTH-001 | Exact `iss` match (ALG-OIDC-VALIDATE-001 step 5) |
| Audience confusion | COMP-AUTH-001 | `aud` contains `client_id` (step 6) |
| Policy injection | COMP-POLICY-004 | Schema validation (DC-AUTH-CEDAR-VALIDATE-001); size limit (DC-AUTH-CEDAR-SIZE-001) |
| Authorization bypass | Middleware chain | No route exists outside the middleware chain; all paths require auth |
| Session hijacking | COMP-SESSION-003 | HTTP-only, Secure, SameSite=Strict cookies; refresh token rotation |
| Memory exhaustion | COMP-POLICY-004 | Max policy count (10K) + max policy size (256 KiB) |
| Timing side-channel | COMP-AUTH-001 | Constant-time signature comparison (HMAC/RSA) |

### Error Response Security

| Error Type | HTTP Status | Response Body | Logged |
|------------|-------------|---------------|--------|
| Invalid/expired token | 401 | `{"error": "unauthenticated"}` | WARN |
| Cedar Deny | 403 | `{"error": "forbidden"}` | INFO |
| Cedar evaluation error | 403 | `{"error": "forbidden"}` | ERROR |
| Internal error | 500 | `{"error": "internal_error"}` | ERROR |

No internal details (policy text, claim values, stack traces) are exposed to clients per DC-AUTH-ERROR-001.

---

## BP-11: Evolution and Extensibility

### Future Considerations

| Area | Potential Extension | Impact |
|------|-------------------|--------|
| Multi-OP support | Multiple OIDC providers with issuer-based routing | COMP-AUTH-001: multi-JWKS cache keyed by issuer |
| Attribute providers | External attribute sources (LDAP, SCIM) | New component: AttributeProvider behind IF-ATTR-001 |
| Policy templates | Admin UI for template-based policy creation | COMP-POLICY-004: template CRUD + slot management |
| Policy versioning | A/B testing of policy sets | COMP-POLICY-004: versioned policy sets with gradual rollout |
| OPA migration path | Hybrid Cedar+OPA evaluation | COMP-CEDAR-002: adapter trait for alternative evaluators |

### Extension Points

- `OidcValidator` trait: alternative implementations for different OIDC libraries or custom validation logic.
- `CedarAuthorizer` trait: alternative evaluators (OPA, Casbin) behind a common interface.
- `PolicyStore` trait: database, file, or remote policy server backends.
- `SessionManager` trait: Redis, PostgreSQL, or in-memory session stores.

---

## BP-12: Bibliography

1. **IEEE 1016-2009** -- Standard for Information Technology -- Systems Design -- Software Design Descriptions.

2. **OpenID Connect Core 1.0** (incorporating errata set 2). N. Sakimura, J. Bradley, M. Jones, B. de Medeiros, C. Mortimore. December 2023.

3. **OAuth 2.0 Authorization Framework** (RFC 6749). D. Hardt. October 2012.

4. **JSON Web Token (JWT)** (RFC 7519). M. Jones, J. Bradley, N. Sakimura. May 2015.

5. **JSON Web Signature (JWS)** (RFC 7515). M. Jones, J. Bradley, N. Sakimura. May 2015.

6. **Proof Key for Code Exchange by OAuth Public Clients** (RFC 7636). N. Sakimura, J. Bradley. September 2015.

7. **Cedar Policy Language Reference Guide**. AWS Cedar Team. https://docs.cedarpolicy.com

8. **cedar-policy crate v4.9.1**. AWS. https://docs.rs/cedar-policy/4.9.1/cedar_policy/

9. **How we built Cedar with automated reasoning and differential testing**. Amazon Science Blog.

10. **OpenID Connect Discovery 1.0** (incorporating errata set 1). N. Sakimura, J. Bradley, M. Jones, E. Jay. December 2023.

11. **Axum Web Framework Documentation**. Tokio Contributors. https://docs.rs/axum

12. **FERRO-REQ-001**: Ferro System Requirements -- EARS Specification.

13. **YP-AUTH-OIDC-CEDAR-001**: Authentication (OIDC) & Authorization (Cedar Policy Language) -- Yellow Paper.
