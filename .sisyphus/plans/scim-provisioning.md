# SCIM 2.0 Provisioning Endpoint

## Context

Build a new `crates/scim` crate implementing a SCIM 2.0 (System for Cross-domain Identity Management) provisioning endpoint. This enables identity providers (Okta, Azure AD, etc.) to provision users and groups into Ferro via a standard protocol.

## Files to Create

### 1. `crates/scim/Cargo.toml`

```toml
[package]
name = "ferro-scim"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[dependencies]
axum = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }
tracing = "0.1"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
tokio = { workspace = true, features = ["full", "test-util"] }
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

### 2. `crates/scim/src/lib.rs`

Public API re-exports. Modules: `error`, `schema`, `user`, `group`, `handler`.

### 3. `crates/scim/src/error.rs`

SCIM-compliant error types using `thiserror`. Must include:
- `ScimError` enum with variants: `NotFound`, `MethodNotAllowed`, `InvalidSchema`, `InvalidSyntax`, `TooMany`, `Uniqueness`, `Mutability`, `InvalidValue`, `Internal`, `BadRequest`
- `ScimErrorResponse` struct (serializable) with `schemas`, `scimType`, `detail`, `status` fields
- `impl IntoResponse for ScimError` to convert errors to proper SCIM JSON error responses with correct HTTP status codes

### 4. `crates/scim/src/schema.rs`

SCIM schema URI constants and schema definition types:
- Schema URIs: `USER_SCHEMA_URI`, `GROUP_SCHEMA_URI`, `SCIM_CORE_SCHEMA_URI`, `ENTERPRISE_USER_SCHEMA_URI`
- `ScimSchema` struct with `id`, `name`, `description`, `attributes`, `meta` fields
- `ScimSchemaAttribute` struct with `name`, `type`, `description`, `required`, `mutability`, `returned`, `uniqueness` fields
- `fn user_schema() -> ScimSchema` - returns the User resource schema definition
- `fn group_schema() -> ScimSchema` - returns the Group resource schema definition
- `fn sp_config_schema() -> ScimSchema` - returns ServiceProviderConfig schema
- `fn resource_type_schemas() -> Vec<ScimSchema>` - returns all resource type schemas

### 5. `crates/scim/src/user.rs`

Core SCIM user types:
- `ScimUser` struct with fields: `id`, `external_id`, `user_name`, `display_name`, `name` (Option<ScimName>), `emails` (Vec<ScimEmail>), `phone_numbers` (Vec<ScimPhoneNumber>), `active`, `groups` (Vec<ScimGroupRef>), `meta` (ScimMeta)
- `ScimName` struct with `formatted`, `family_name`, `given_name`
- `ScimEmail` struct with `value`, `type_` (serde rename), `primary`
- `ScimPhoneNumber` struct with `value`, `type_`, `primary`
- `ScimGroupRef` struct with `value`, `$ref`, `display`
- `ScimUserCreateRequest` struct with `schemas`, `external_id`, `user_name`, `display_name`, `name`, `emails`, `active`
- `ScimUserPatchRequest` struct with `schemas`, `Operations` (Vec<PatchOperation>)
- `PatchOperation` struct with `op`, `path`, `value`
- All structs derive `Serialize, Deserialize, Clone, Debug`

### 6. `crates/scim/src/group.rs`

Core SCIM group types:
- `ScimGroup` struct with `id`, `display_name`, `members` (Vec<ScimUserRef>), `meta`
- `ScimUserRef` struct with `value`, `$ref`, `display`
- `ScimGroupCreateRequest` with `schemas`, `display_name`, `members`
- All structs derive `Serialize, Deserialize, Clone, Debug`

### 7. `crates/scim/src/handler.rs`

HTTP handler functions using axum extractors. This is the main integration point.

#### Handler functions (all async):

```rust
// User endpoints
pub async fn handle_users(State(state): State<AppState>, method: Method, body: Bytes) -> impl IntoResponse
pub async fn handle_user(State(state): State<AppState>, method: Method, Path(id): Path<String>, body: Bytes) -> impl IntoResponse

// Group endpoints
pub async fn handle_groups(State(state): State<AppState>, method: Method, body: Bytes) -> impl IntoResponse
pub async fn handle_group(State(state): State<AppState>, method: Method, Path(id): Path<String>, body: Bytes) -> impl IntoResponse

// Discovery endpoints
pub async fn handle_sp_config() -> impl IntoResponse
pub async fn handle_schemas() -> impl IntoResponse
pub async fn handle_resource_types() -> impl IntoResponse
```

#### Internal handler logic:

Each CRUD handler dispatches on `Method`:
- **GET (list)**: `ScimListResponse<ScimUser>` / `ScimListResponse<ScimGroup>` with pagination via `startIndex` and `count` query params
- **POST (create)**: Parse body as `ScimUserCreateRequest`, validate schemas, return 201 with created resource
- **GET (by id)**: Return single resource
- **PUT (replace)**: Full replace of resource
- **DELETE**: Remove resource, return 204
- **PATCH**: Apply operations from `ScimUserPatchRequest`

#### Response helpers:
- `fn scim_json_response<T: Serialize>(status: StatusCode, data: &T) -> Response` - wraps response with `Content-Type: application/scim+json`
- `fn scim_error_response(error: ScimError) -> Response` - wraps error into SCIM JSON error format

#### AppState reference:
The handler needs to reference `crate::AppState` from the server crate. To avoid circular dependency, the handler module will define a `ScimState` trait that the server's `AppState` will implement:

```rust
#[async_trait]
pub trait ScimUserStore: Send + Sync {
    async fn list_users(&self, start: u32, count: u32) -> Result<(Vec<User>, u32), ScimError>;
    async fn get_user(&self, id: &str) -> Result<User, ScimError>;
    async fn create_user(&self, user: ScimUserCreateRequest) -> Result<User, ScimError>;
    async fn update_user(&self, id: &str, user: ScimUserCreateRequest) -> Result<User, ScimError>;
    async fn delete_user(&self, id: &str) -> Result<(), ScimError>;
    async fn patch_user(&self, id: &str, ops: Vec<PatchOperation>) -> Result<User, ScimError>;
}

#[async_trait]
pub trait ScimGroupStore: Send + Sync {
    async fn list_groups(&self, start: u32, count: u32) -> Result<(Vec<ScimGroup>, u32), ScimError>;
    async fn get_group(&self, id: &str) -> Result<ScimGroup, ScimError>;
    async fn create_group(&self, group: ScimGroupCreateRequest) -> Result<ScimGroup, ScimError>;
    async fn update_group(&self, id: &str, group: ScimGroupCreateRequest) -> Result<ScimGroup, ScimError>;
    async fn delete_group(&self, id: &str) -> Result<(), ScimError>;
}
```

## Files to Modify

### 8. `Cargo.toml` (workspace root)

Add `"crates/scim"` to the `[workspace] members` list.

### 9. `crates/server/Cargo.toml`

Add dependency: `ferro-scim = { path = "../scim" }`

### 10. `crates/server/src/lib.rs`

Add `pub mod scim_integration;` module declaration, and add SCIM routes to `build_router_with_static`:

```rust
.route("/scim/v2/Users", any(ferro_scim::handler::handle_users))
.route("/scim/v2/Users/{id}", any(ferro_scim::handler::handle_user))
.route("/scim/v2/Groups", any(ferro_scim::handler::handle_groups))
.route("/scim/v2/Groups/{id}", any(ferro_scim::handler::handle_group))
.route("/scim/v2/ServiceProviderConfig", get(ferro_scim::handler::handle_sp_config))
.route("/scim/v2/Schemas", get(ferro_scim::handler::handle_schemas))
.route("/scim/v2/ResourceTypes", get(ferro_scim::handler::handle_resource_types))
```

### 11. `crates/server/src/scim_integration.rs` (new file)

Implements `ScimUserStore` and `ScimGroupStore` traits for `AppState`, bridging to the existing `user_store` field.

## Verification

1. `cargo check -p ferro-scim` - passes type checking
2. `cargo test -p ferro-scim` - unit tests pass (serialization round-trips, error responses, schema generation)
3. `cargo check -p ferro-server` - integration compiles
4. `cargo clippy -p ferro-scim` - no warnings
