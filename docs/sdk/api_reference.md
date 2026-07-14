# API Reference

## Authentication

### User Types (`ferro-auth::users`)

#### User

```rust
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub status: UserStatus,
    pub storage_quota_bytes: Option<u64>,
    pub storage_used_bytes: u64,
    pub is_ldap: bool,
    pub password_hash: Option<ZeroizeString>,
    pub totp_secret: Option<ZeroizeString>,
    pub totp_enabled: bool,
}
```

A registered user in the system.

**Methods:**

- `is_active() -> bool` - Check whether the user account is active
- `is_admin() -> bool` - Check whether the user has admin privileges
- `has_read_write() -> bool` - Check whether the user can read and write

#### UserRole

```rust
pub enum UserRole {
    Admin,
    User,
    ReadOnly,
}
```

Role assigned to a user, controlling their access level.

#### UserStatus

```rust
pub enum UserStatus {
    Active,
    Disabled,
    Locked,
}
```

Account status of a user.

#### UserInfo

```rust
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
}
```

Lightweight user identity attached to authenticated requests.

#### InMemoryUserStore

```rust
pub struct InMemoryUserStore { /* private fields */ }
```

In-memory user store backed by concurrent hash maps, with optional SQLite persistence.

**Methods:**

- `new() -> Self` - Create a new empty in-memory user store
- `with_db(self, db: DbHandle) -> Self` - Attach a SQLite database handle for persistent storage
- `create_admin(username: &str, password: &str) -> Option<User>` - Create a pre-configured admin user
- `load_from_db(&self, user: User)` - Load a user into the in-memory store
- `load_all_from_db(conn: &Connection) -> Result<Vec<User>, Error>` - Load all users from SQLite

#### UserStoreTrait

```rust
#[async_trait]
pub trait UserStoreTrait: Send + Sync {
    async fn create_user(&self, user: User) -> Result<User, UserError>;
    async fn get_user(&self, id: &str) -> Result<User, UserError>;
    async fn get_user_by_username(&self, username: &str) -> Result<User, UserError>;
    async fn get_user_by_email(&self, email: &str) -> Result<User, UserError>;
    async fn list_users(&self) -> Vec<User>;
    async fn update_user(&self, id: &str, updates: UpdateUserRequest) -> Result<User, UserError>;
    async fn delete_user(&self, id: &str) -> Result<(), UserError>;
    async fn update_last_login(&self, id: &str);
    async fn set_password(&self, id: &str, password_hash: &str) -> Result<(), UserError>;
    async fn authenticate(&self, username: &str, password: &str) -> Result<User, UserError>;
}
```

Async interface for persisting and retrieving user accounts.

#### hash_password

```rust
pub fn hash_password(password: &str) -> Result<String, UserError>
```

Hash a password using bcrypt with the default cost factor.

### Request Types

#### CreateUserRequest

```rust
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password: ZeroizeString,
    pub role: UserRole,
    pub storage_quota_bytes: Option<u64>,
}
```

#### UpdateUserRequest

```rust
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub storage_quota_bytes: Option<Option<u64>>,
}
```

#### UpdateSelfRequest

```rust
pub struct UpdateSelfRequest {
    pub display_name: Option<String>,
    pub password: Option<ZeroizeString>,
}
```

#### ResetPasswordRequest

```rust
pub struct ResetPasswordRequest {
    pub new_password: ZeroizeString,
}
```

### Error Types

#### UserError

```rust
pub struct UserError {
    pub kind: UserErrorKind,
    pub message: String,
}
```

**Methods:**

- `not_found(msg: impl Into<String>) -> Self` - Create a "not found" error
- `conflict(msg: impl Into<String>) -> Self` - Create a "conflict" (duplicate) error
- `forbidden(msg: impl Into<String>) -> Self` - Create a "forbidden" error
- `bad_request(msg: impl Into<String>) -> Self` - Create a "bad request" error

#### UserErrorKind

```rust
pub enum UserErrorKind {
    NotFound,
    Conflict,
    Forbidden,
    BadRequest,
}
```

### OIDC Types (`ferro-auth::oidc`)

#### OidcConfig

```rust
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub audience: String,
    pub jwks_uri: Option<String>,
}
```

OIDC provider configuration.

#### OidcValidator

```rust
pub struct OidcValidator { /* private fields */ }
```

Validates OIDC tokens and manages PKCE sessions.

**Methods:**

- `new(config: OidcConfig) -> Self` - Create a new validator
- `config(&self) -> &OidcConfig` - Return a reference to the OIDC configuration
- `store_pkce_session(&self, state: &str, code_verifier: &str, redirect_uri: &str, callback_url: &str)` - Store a PKCE session
- `consume_pkce_session(&self, state: &str) -> Option<PkceSession>` - Consume and return a PKCE session
- `exchange_code(&self, code: &str, code_verifier: &str, redirect_uri: &str) -> Result<Value>` - Exchange an authorization code for tokens
- `refresh_access_token(&self, refresh_token: &str) -> Result<Value>` - Refresh an access token
- `validate_token(&self, token: &str) -> Result<Claims>` - Validate a JWT access token

#### PkceSession

```rust
pub struct PkceSession {
    pub code_verifier: String,
    pub redirect_uri: String,
    pub callback_url: String,
    pub created_at: Instant,
}
```

## DAV Operations

### Store Types (`ferro-dav::store`)

#### CalendarInfo

```rust
pub struct CalendarInfo {
    pub id: String,
    pub principal: String,
    pub name: String,
    pub color: String,
    pub ctag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Information about a calendar collection.

#### EventInfo

```rust
pub struct EventInfo {
    pub uid: String,
    pub calendar_id: String,
    pub ical_data: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Information about a calendar event.

#### CalFilter

```rust
pub struct CalFilter {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}
```

Time-range filter for calendar event queries.

#### CalendarStore

```rust
#[async_trait]
pub trait CalendarStore: Send + Sync {
    async fn list_calendars(&self, principal: &str) -> Vec<CalendarInfo>;
    async fn get_calendar(&self, principal: &str, calendar_id: &str) -> Option<CalendarInfo>;
    async fn create_calendar(&self, principal: &str, name: &str, color: &str) -> StoreResult<CalendarInfo>;
    async fn delete_calendar(&self, principal: &str, calendar_id: &str) -> StoreResult<()>;
    async fn list_events(&self, calendar_id: &str) -> Vec<EventInfo>;
    async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<EventInfo>;
    async fn create_event(&self, calendar_id: &str, ical: &str) -> StoreResult<EventInfo>;
    async fn update_event(&self, calendar_id: &str, event_uid: &str, ical: &str) -> StoreResult<EventInfo>;
    async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> StoreResult<()>;
    async fn query_events(&self, calendar_id: &str, filter: &CalFilter) -> Vec<EventInfo>;
}
```

Trait for calendar data storage backends.

#### AddressBookInfo

```rust
pub struct AddressBookInfo {
    pub id: String,
    pub principal: String,
    pub name: String,
    pub ctag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Information about an address book collection.

#### ContactInfo

```rust
pub struct ContactInfo {
    pub uid: String,
    pub address_book_id: String,
    pub vcard_data: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Information about a contact (vCard).

#### AddressBookStore

```rust
#[async_trait]
pub trait AddressBookStore: Send + Sync {
    async fn list_address_books(&self, principal: &str) -> Vec<AddressBookInfo>;
    async fn get_address_book(&self, principal: &str, book_id: &str) -> Option<AddressBookInfo>;
    async fn create_address_book(&self, principal: &str, name: &str) -> StoreResult<AddressBookInfo>;
    async fn delete_address_book(&self, principal: &str, book_id: &str) -> StoreResult<()>;
    async fn list_contacts(&self, book_id: &str) -> Vec<ContactInfo>;
    async fn get_contact(&self, book_id: &str, contact_uid: &str) -> Option<ContactInfo>;
    async fn create_contact(&self, book_id: &str, vcard: &str) -> StoreResult<ContactInfo>;
    async fn update_contact(&self, book_id: &str, contact_uid: &str, vcard: &str) -> StoreResult<ContactInfo>;
    async fn delete_contact(&self, book_id: &str, contact_uid: &str) -> StoreResult<()>;
}
```

Trait for address book data storage backends.

### iCalendar Types (`ferro-dav::ical`)

#### IcalComponent

```rust
pub struct IcalComponent {
    pub name: String,
    pub properties: HashMap<String, Vec<IcalProperty>>,
    pub children: Vec<IcalComponent>,
}
```

A parsed iCalendar component (e.g. VCALENDAR, VEVENT, VTODO).

#### IcalProperty

```rust
pub struct IcalProperty {
    pub name: String,
    pub params: HashMap<String, String>,
    pub value: String,
}
```

A single property within an iCalendar component.

**Functions:**

- `parse_ical(input: &str) -> Result<Vec<IcalComponent>, String>` - Parse RFC 5545 iCalendar string
- `serialize_ical(components: &[IcalComponent]) -> String` - Serialize components to iCalendar string
- `get_first_prop<'a>(component: &'a IcalComponent, name: &str) -> Option<&'a IcalProperty>` - Get first property by name
- `get_all_props<'a>(component: &'a IcalComponent, name: &str) -> Vec<&'a IcalProperty>` - Get all properties by name

### vCard Types (`ferro-dav::vcard`)

#### Vcard

```rust
pub struct Vcard {
    pub uid: Option<String>,
    pub fn_name: String,
    pub family_name: String,
    pub given_name: String,
    pub additional_names: String,
    pub prefix: String,
    pub suffix: String,
    pub emails: Vec<VcardValue>,
    pub phones: Vec<VcardValue>,
    pub addresses: Vec<VcardAddress>,
    pub org: Option<String>,
    pub title: Option<String>,
    pub role: Option<String>,
    pub photo: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
    pub properties: HashMap<String, Vec<VcardProperty>>,
}
```

A parsed vCard (RFC 6350) contact.

#### VcardProperty

```rust
pub struct VcardProperty {
    pub name: String,
    pub params: HashMap<String, String>,
    pub value: String,
}
```

A single property line in a vCard.

#### VcardValue

```rust
pub struct VcardValue {
    pub value: String,
    pub types: Vec<String>,
    pub pref: Option<u32>,
}
```

A typed value with TYPE parameters (used for emails, phones).

#### VcardAddress

```rust
pub struct VcardAddress {
    pub po_box: String,
    pub extended: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
    pub types: Vec<String>,
}
```

A structured postal address from a vCard ADR property.

**Functions:**

- `parse_vcard(input: &str) -> Result<Vcard, String>` - Parse RFC 6350 vCard string
- `serialize_vcard(vcard: &Vcard) -> String` - Serialize contact to vCard string

## Common Types (`ferro-common`)

### Claims

```rust
pub struct Claims {
    pub sub: String,
    pub aud: String,
    pub iss: String,
    pub exp: u64,
    pub iat: u64,
    pub nonce: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub groups: Option<Vec<String>>,
}
```

JWT claims structure used for OIDC token validation.

### FerroError

```rust
pub enum FerroError {
    NotFound,
    Unauthorized,
    Forbidden,
    Conflict(String),
    BadRequest(String),
    Internal(String),
    Storage(String),
}
```

Application error type used across the Ferro ecosystem.

### ZeroizeString

```rust
pub struct ZeroizeString(String);
```

Secure string that zeros its memory on drop. Used for passwords and secrets.
