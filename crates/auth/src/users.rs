use async_trait::async_trait;
use chrono::Utc;
pub use common::zeroize::ZeroizeString;
use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// # Safety
/// The wrapped `rusqlite::Connection` is only accessed via short-lived lock guards
/// that never cross an `.await` point. `SQLite` operations are synchronous
/// and complete in microseconds, well below the threshold for async poisoning.
pub type DbHandle = std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>;

/// Role assigned to a user, controlling their access level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum UserRole {
    #[default]
    Admin,
    User,
    ReadOnly,
}

/// Account status of a user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    Active,
    Disabled,
    Locked,
}

/// A registered user in the system.
#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub status: UserStatus,
    pub storage_quota_bytes: Option<u64>,
    pub storage_used_bytes: u64,
    pub is_ldap: bool,
    #[serde(skip_serializing)]
    pub password_hash: Option<ZeroizeString>,
    /// Base32-encoded TOTP secret. Present when TOTP is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp_secret: Option<ZeroizeString>,
    /// Whether TOTP two-factor authentication is enabled.
    #[serde(default)]
    pub totp_enabled: bool,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("username", &self.username)
            .field("display_name", &self.display_name)
            .field("email", &self.email)
            .field("role", &self.role)
            .field("created_at", &self.created_at)
            .field("last_login", &self.last_login)
            .field("status", &self.status)
            .field("storage_quota_bytes", &self.storage_quota_bytes)
            .field("storage_used_bytes", &self.storage_used_bytes)
            .field("is_ldap", &self.is_ldap)
            .field("password_hash", &self.password_hash.as_ref().map(|_| "[REDACTED]"))
            .field("totp_secret", &self.totp_secret.as_ref().map(|_| "[REDACTED]"))
            .field("totp_enabled", &self.totp_enabled)
            .finish()
    }
}

impl User {
    /// Check whether the user account is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// Check whether the user has admin privileges.
    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    /// Check whether the user can read and write (admin or user role).
    #[must_use]
    pub fn has_read_write(&self) -> bool {
        self.role == UserRole::Admin || self.role == UserRole::User
    }
}

/// Lightweight user identity attached to authenticated requests.
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
}

impl From<&User> for UserInfo {
    fn from(u: &User) -> Self {
        Self {
            user_id: u.id.clone(),
            username: u.username.clone(),
            role: u.role.clone(),
        }
    }
}

/// Request body for creating a new user.
#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password: ZeroizeString,
    #[serde(default)]
    pub role: UserRole,
    pub storage_quota_bytes: Option<u64>,
}

impl std::fmt::Debug for CreateUserRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateUserRequest")
            .field("username", &self.username)
            .field("display_name", &self.display_name)
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .field("role", &self.role)
            .field("storage_quota_bytes", &self.storage_quota_bytes)
            .finish()
    }
}

/// Request body for updating an existing user (admin-only fields).
#[derive(Debug, Deserialize, Default)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub storage_quota_bytes: Option<Option<u64>>,
}

/// Request body for a user updating their own profile.
#[derive(Deserialize)]
pub struct UpdateSelfRequest {
    pub display_name: Option<String>,
    pub password: Option<ZeroizeString>,
}

impl std::fmt::Debug for UpdateSelfRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateSelfRequest")
            .field("display_name", &self.display_name)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// Request body for resetting a user's password.
#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: ZeroizeString,
}

impl std::fmt::Debug for ResetPasswordRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResetPasswordRequest")
            .field("new_password", &"[REDACTED]")
            .finish()
    }
}

/// Error returned by user store operations.
#[derive(Debug)]
pub struct UserError {
    pub kind: UserErrorKind,
    pub message: String,
}

/// Kind of user store error.
#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum UserErrorKind {
    NotFound,
    Conflict,
    Forbidden,
    BadRequest,
}

impl UserError {
    /// Create a "not found" error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            kind: UserErrorKind::NotFound,
            message: msg.into(),
        }
    }
    /// Create a "conflict" (duplicate) error.
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            kind: UserErrorKind::Conflict,
            message: msg.into(),
        }
    }
    /// Create a "forbidden" error.
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            kind: UserErrorKind::Forbidden,
            message: msg.into(),
        }
    }
    /// Create a "bad request" error.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            kind: UserErrorKind::BadRequest,
            message: msg.into(),
        }
    }
}

/// Async interface for persisting and retrieving user accounts.
#[async_trait]
pub trait UserStoreTrait: Send + Sync {
    /// Register a new user.
    async fn create_user(&self, user: User) -> Result<User, UserError>;
    /// Look up a user by their unique ID.
    async fn get_user(&self, id: &str) -> Result<User, UserError>;
    /// Look up a user by their username.
    async fn get_user_by_username(&self, username: &str) -> Result<User, UserError>;
    /// Look up a user by their email address.
    async fn get_user_by_email(&self, email: &str) -> Result<User, UserError>;
    /// List all registered users.
    async fn list_users(&self) -> Vec<User>;
    /// Update mutable fields of an existing user.
    async fn update_user(&self, id: &str, updates: UpdateUserRequest) -> Result<User, UserError>;
    /// Remove a user by ID.
    async fn delete_user(&self, id: &str) -> Result<(), UserError>;
    /// Record the current time as the user's last login.
    async fn update_last_login(&self, id: &str);
    /// Set a user's password hash directly.
    async fn set_password(&self, id: &str, password_hash: &str) -> Result<(), UserError>;
    /// Authenticate a username and password, returning the user on success.
    async fn authenticate(&self, username: &str, password: &str) -> Result<User, UserError>;

    /// Blocking wrapper around [`Self::get_user_by_username`].
    fn get_user_by_username_blocking(&self, username: &str) -> Result<User, UserError> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.get_user_by_username(username))
    }
}

/// Hash a password using bcrypt with the default cost factor.
///
/// Returns the bcrypt hash string on success, or a `UserError` if bcrypt
/// fails (e.g., memory exhaustion). This avoids panicking on external
/// library failures.
pub fn hash_password(password: &str) -> Result<String, UserError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| UserError::bad_request(format!("Password hashing failed: {e}")))
}

fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

const MAX_USERS: usize = 10_000;

/// In-memory user store backed by concurrent hash maps, with optional `SQLite` persistence.
pub struct InMemoryUserStore {
    users: DashMap<String, User>,
    username_index: DashMap<String, String>,
    email_index: DashMap<String, String>,
    db: Option<DbHandle>,
}

impl InMemoryUserStore {
    /// Create a new empty in-memory user store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
            username_index: DashMap::new(),
            email_index: DashMap::new(),
            db: None,
        }
    }

    /// Attach a `SQLite` database handle for persistent storage.
    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    /// Create a pre-configured admin user with the given credentials.
    ///
    /// Returns `None` if password hashing fails.
    #[must_use]
    pub fn create_admin(username: &str, password: &str) -> Option<User> {
        let password_hash = hash_password(password).ok()?;
        Some(User {
            id: uuid::Uuid::new_v4().to_string(),
            username: username.to_string(),
            display_name: username.to_string(),
            email: String::new(),
            role: UserRole::Admin,
            created_at: Utc::now(),
            last_login: None,
            status: UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ZeroizeString::new(password_hash)),
            totp_secret: None,
            totp_enabled: false,
        })
    }

    /// Load a user into the in-memory store (used during DB restore).
    pub fn load_from_db(&self, user: User) {
        let username = user.username.clone();
        let email = user.email.clone();
        let id = user.id.clone();
        if !self.username_index.contains_key(&username) {
            self.username_index.insert(username, id.clone());
        }
        if !email.is_empty() && !self.email_index.contains_key(&email) {
            self.email_index.insert(email, id.clone());
        }
        self.users.insert(id, user);
    }

    fn persist_user(&self, user: &User) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO users (id, username, display_name, email, role, created_at, last_login, status, storage_quota_bytes, storage_used_bytes, is_ldap, password_hash, totp_secret, totp_enabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    user.id,
                    user.username,
                    user.display_name,
                    user.email,
                    format!("{:?}", user.role),
                    user.created_at.to_rfc3339(),
                    user.last_login.map(|l| l.to_rfc3339()),
                    format!("{:?}", user.status),
                    user.storage_quota_bytes.unwrap_or(0) as i64,
                    user.storage_used_bytes as i64,
                    i32::from(user.is_ldap),
                    user.password_hash.as_ref().map(|s| s.as_str()),
                    user.totp_secret.as_ref().map(|s| s.as_str()),
                    i32::from(user.totp_enabled),
                ],
            ) {
                warn!("Failed to persist user to SQLite: {}", e);
            }
        }
    }

    fn delete_user_from_db(&self, id: &str) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = conn.execute("DELETE FROM users WHERE id = ?1", params![id]) {
                warn!("Failed to delete user from SQLite: {}", e);
            }
        }
    }

    /// Load all users from a `SQLite` connection into a vector.
    pub fn load_all_from_db(conn: &rusqlite::Connection) -> Result<Vec<User>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, username, display_name, email, role, created_at, last_login, status, storage_quota_bytes, storage_used_bytes, is_ldap, password_hash FROM users",
        )?;
        let rows = stmt.query_map([], |row| {
            let role_str: String = row.get(4)?;
            let role = match role_str.as_str() {
                "Admin" => UserRole::Admin,
                "User" => UserRole::User,
                "ReadOnly" => UserRole::ReadOnly,
                _ => UserRole::User,
            };
            let status_str: String = row.get(7)?;
            let status = match status_str.as_str() {
                "Active" => UserStatus::Active,
                "Disabled" => UserStatus::Disabled,
                "Locked" => UserStatus::Locked,
                _ => UserStatus::Active,
            };
            let created_at_str: String = row.get(5)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
            let last_login: Option<String> = row.get(6)?;
            let last_login = last_login.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            });
            let quota: i64 = row.get(8)?;
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                email: row.get(3)?,
                role,
                created_at,
                last_login,
                status,
                storage_quota_bytes: if quota == 0 { None } else { Some(quota as u64) },
                storage_used_bytes: row.get::<_, i64>(9)? as u64,
                is_ldap: row.get::<_, i32>(10)? != 0,
                password_hash: row.get::<_, Option<String>>(11)?.map(ZeroizeString::new),
                totp_secret: row.get::<_, Option<String>>(12).unwrap_or(None).map(ZeroizeString::new),
                totp_enabled: row.get::<_, i32>(13).unwrap_or(0) != 0,
            })
        })?;
        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }
}

impl Default for InMemoryUserStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserStoreTrait for InMemoryUserStore {
    async fn create_user(&self, user: User) -> Result<User, UserError> {
        if self.users.len() >= MAX_USERS {
            return Err(UserError::bad_request("MAX_USERS_REACHED".to_string()));
        }
        if self.username_index.contains_key(&user.username) {
            return Err(UserError::conflict(format!(
                "Username '{}' already exists",
                user.username
            )));
        }
        if !user.email.is_empty() && self.email_index.contains_key(&user.email) {
            return Err(UserError::conflict(format!("Email '{}' already in use", user.email)));
        }
        let id = user.id.clone();
        let username = user.username.clone();
        let email = user.email.clone();
        self.username_index.insert(username, id.clone());
        if !email.is_empty() {
            self.email_index.insert(email, id.clone());
        }
        self.users.insert(id, user.clone());
        self.persist_user(&user);
        Ok(user)
    }

    async fn get_user(&self, id: &str) -> Result<User, UserError> {
        self.users
            .get(id)
            .map(|r| r.value().clone())
            .ok_or_else(|| UserError::not_found(format!("User '{id}' not found")))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User, UserError> {
        let id = self
            .username_index
            .get(username)
            .map(|r| r.value().clone())
            .ok_or_else(|| UserError::not_found(format!("User '{username}' not found")))?;
        self.get_user(&id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, UserError> {
        let id = self
            .email_index
            .get(email)
            .map(|r| r.value().clone())
            .ok_or_else(|| UserError::not_found(format!("No user with email '{email}'")))?;
        self.get_user(&id).await
    }

    async fn list_users(&self) -> Vec<User> {
        self.users.iter().map(|r| r.value().clone()).collect()
    }

    async fn update_user(&self, id: &str, updates: UpdateUserRequest) -> Result<User, UserError> {
        let mut user = self.get_user(id).await?;

        if let Some(ref new_name) = updates.display_name {
            user.display_name = new_name.clone();
        }
        if let Some(ref new_email) = updates.email {
            if !new_email.is_empty() {
                if let Some(existing_id) = self.email_index.get(new_email)
                    && existing_id.value() != id
                {
                    return Err(UserError::conflict(format!("Email '{new_email}' already in use")));
                }
                if !user.email.is_empty() {
                    self.email_index.remove(&user.email);
                }
                self.email_index.insert(new_email.clone(), id.to_string());
            } else if !user.email.is_empty() {
                self.email_index.remove(&user.email);
            }
            user.email = new_email.clone();
        }
        if let Some(ref role) = updates.role {
            user.role = role.clone();
        }
        if let Some(ref status) = updates.status {
            user.status = status.clone();
        }
        if let Some(ref quota) = updates.storage_quota_bytes {
            user.storage_quota_bytes = *quota;
        }

        self.users.insert(id.to_string(), user.clone());
        self.persist_user(&user);
        Ok(user)
    }

    async fn delete_user(&self, id: &str) -> Result<(), UserError> {
        let user = self.get_user(id).await?;
        self.users.remove(id);
        self.username_index.remove(&user.username);
        if !user.email.is_empty() {
            self.email_index.remove(&user.email);
        }
        self.delete_user_from_db(id);
        Ok(())
    }

    async fn update_last_login(&self, id: &str) {
        if let Some(mut user) = self.users.get_mut(id) {
            user.last_login = Some(Utc::now());
            let u = user.clone();
            drop(user);
            if let Some(ref db) = self.db {
                let conn = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Err(e) = conn.execute(
                    "UPDATE users SET last_login = ?1 WHERE id = ?2",
                    params![u.last_login.map(|l| l.to_rfc3339()), u.id],
                ) {
                    warn!("Failed to persist last_login to SQLite: {}", e);
                }
            }
        }
    }

    async fn set_password(&self, id: &str, password_hash: &str) -> Result<(), UserError> {
        let mut user = self.get_user(id).await?;
        user.password_hash = Some(ZeroizeString::new(password_hash.to_string()));
        self.users.insert(id.to_string(), user.clone());
        self.persist_user(&user);
        Ok(())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<User, UserError> {
        let user = self.get_user_by_username(username).await?;
        if !user.is_active() {
            return Err(UserError::forbidden("User account is not active"));
        }
        let hash = user
            .password_hash
            .as_ref()
            .map(|s| s.as_str())
            .ok_or_else(|| UserError::forbidden("No password set for this user"))?;
        if !verify_password(password, hash) {
            return Err(UserError::forbidden("Invalid password"));
        }
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_user(username: &str, email: &str, role: UserRole) -> User {
        User {
            id: uuid::Uuid::new_v4().to_string(),
            username: username.to_string(),
            display_name: username.to_string(),
            email: email.to_string(),
            role,
            created_at: Utc::now(),
            last_login: None,
            status: UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ZeroizeString::new(hash_password("testpass").unwrap())),
            totp_secret: None,
            totp_enabled: false,
        }
    }

    fn store() -> Arc<InMemoryUserStore> {
        Arc::new(InMemoryUserStore::new())
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let s = store();
        let user = make_user("alice", "alice@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        let fetched = s.get_user(&id).await.unwrap();
        assert_eq!(fetched.username, "alice");
        assert_eq!(fetched.email, "alice@example.com");
    }

    #[tokio::test]
    async fn test_get_user_by_username() {
        let s = store();
        s.create_user(make_user("bob", "bob@example.com", UserRole::User))
            .await
            .unwrap();

        let user = s.get_user_by_username("bob").await.unwrap();
        assert_eq!(user.username, "bob");
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let s = store();
        let err = s.get_user("nonexistent").await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_duplicate_username_rejected() {
        let s = store();
        s.create_user(make_user("charlie", "c1@example.com", UserRole::User))
            .await
            .unwrap();
        let err = s
            .create_user(make_user("charlie", "c2@example.com", UserRole::User))
            .await
            .unwrap_err();
        assert_eq!(err.kind, UserErrorKind::Conflict);
    }

    #[tokio::test]
    async fn test_duplicate_email_rejected() {
        let s = store();
        s.create_user(make_user("dave", "dave@example.com", UserRole::User))
            .await
            .unwrap();
        let err = s
            .create_user(make_user("dave2", "dave@example.com", UserRole::User))
            .await
            .unwrap_err();
        assert_eq!(err.kind, UserErrorKind::Conflict);
    }

    #[tokio::test]
    async fn test_update_user_role() {
        let s = store();
        let user = make_user("eve", "eve@example.com", UserRole::ReadOnly);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        let updated = s
            .update_user(
                &id,
                UpdateUserRequest {
                    role: Some(UserRole::Admin),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.role, UserRole::Admin);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let s = store();
        let user = make_user("frank", "frank@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        s.delete_user(&id).await.unwrap();
        assert!(s.get_user(&id).await.is_err());
        assert!(s.get_user_by_username("frank").await.is_err());
    }

    #[tokio::test]
    async fn test_authenticate_success() {
        let s = store();
        s.create_user(make_user("grace", "grace@example.com", UserRole::User))
            .await
            .unwrap();

        let result = s.authenticate("grace", "testpass").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let s = store();
        s.create_user(make_user("heidi", "heidi@example.com", UserRole::User))
            .await
            .unwrap();

        let result = s.authenticate("heidi", "wrong").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, UserErrorKind::Forbidden);
    }

    #[tokio::test]
    async fn test_authenticate_disabled_user() {
        let s = store();
        let user = make_user("ivan", "ivan@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();
        s.update_user(
            &id,
            UpdateUserRequest {
                status: Some(UserStatus::Disabled),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let result = s.authenticate("ivan", "testpass").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_users() {
        let s = store();
        s.create_user(make_user("u1", "u1@example.com", UserRole::User))
            .await
            .unwrap();
        s.create_user(make_user("u2", "u2@example.com", UserRole::Admin))
            .await
            .unwrap();
        s.create_user(make_user("u3", "u3@example.com", UserRole::ReadOnly))
            .await
            .unwrap();

        let users = s.list_users().await;
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_update_email_clears_old_index() {
        let s = store();
        let user = make_user("judy", "old@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        s.update_user(
            &id,
            UpdateUserRequest {
                email: Some("new@example.com".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert!(s.get_user_by_email("old@example.com").await.is_err());
        let updated = s.get_user_by_email("new@example.com").await.unwrap();
        assert_eq!(updated.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_set_password() {
        let s = store();
        let user = make_user("kate", "kate@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        s.set_password(&id, &hash_password("newpass").unwrap()).await.unwrap();
        assert!(s.authenticate("kate", "newpass").await.is_ok());
        assert!(s.authenticate("kate", "testpass").await.is_err());
    }

    #[tokio::test]
    async fn test_update_last_login() {
        let s = store();
        let user = make_user("leo", "leo@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        assert!(s.get_user(&id).await.unwrap().last_login.is_none());
        s.update_last_login(&id).await;
        assert!(s.get_user(&id).await.unwrap().last_login.is_some());
    }

    #[tokio::test]
    async fn test_create_admin_user() {
        let admin = InMemoryUserStore::create_admin("admin", "secret").unwrap();
        assert_eq!(admin.username, "admin");
        assert_eq!(admin.role, UserRole::Admin);
        assert!(admin.is_active());
        assert!(admin.is_admin());
    }

    #[tokio::test]
    async fn test_user_has_read_write() {
        let admin = make_user("a", "a@x.com", UserRole::Admin);
        let user = make_user("u", "u@x.com", UserRole::User);
        let readonly = make_user("r", "r@x.com", UserRole::ReadOnly);

        assert!(admin.has_read_write());
        assert!(user.has_read_write());
        assert!(!readonly.has_read_write());
    }

    #[test]
    fn test_user_error_constructors() {
        let e = UserError::not_found("missing");
        assert_eq!(e.kind, UserErrorKind::NotFound);
        assert_eq!(e.message, "missing");

        let e = UserError::conflict("dup");
        assert_eq!(e.kind, UserErrorKind::Conflict);
        assert_eq!(e.message, "dup");

        let e = UserError::forbidden("no");
        assert_eq!(e.kind, UserErrorKind::Forbidden);
        assert_eq!(e.message, "no");

        let e = UserError::bad_request("bad");
        assert_eq!(e.kind, UserErrorKind::BadRequest);
        assert_eq!(e.message, "bad");
    }

    #[test]
    fn test_user_error_debug_format() {
        let e = UserError::not_found("test");
        let debug = format!("{:?}", e);
        assert!(debug.contains("UserError"));
    }

    #[test]
    fn test_user_is_active() {
        let mut user = make_user("a", "a@x.com", UserRole::User);
        user.status = UserStatus::Active;
        assert!(user.is_active());
        user.status = UserStatus::Disabled;
        assert!(!user.is_active());
        user.status = UserStatus::Locked;
        assert!(!user.is_active());
    }

    #[test]
    fn test_user_is_admin() {
        let user = make_user("a", "a@x.com", UserRole::Admin);
        assert!(user.is_admin());
        let user = make_user("u", "u@x.com", UserRole::User);
        assert!(!user.is_admin());
    }

    #[test]
    fn test_user_info_from_user() {
        let user = make_user("alice", "alice@x.com", UserRole::Admin);
        let info = UserInfo::from(&user);
        assert_eq!(info.user_id, user.id);
        assert_eq!(info.username, "alice");
        assert_eq!(info.role, UserRole::Admin);
    }

    #[test]
    fn test_user_role_default() {
        let role = UserRole::default();
        assert_eq!(role, UserRole::Admin);
    }

    #[test]
    fn test_user_role_serialization_roundtrip() {
        let roles = vec![UserRole::Admin, UserRole::User, UserRole::ReadOnly];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let deser: UserRole = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, role);
        }
    }

    #[test]
    fn test_user_status_serialization_roundtrip() {
        let statuses = vec![UserStatus::Active, UserStatus::Disabled, UserStatus::Locked];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deser: UserStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, status);
        }
    }

    #[test]
    fn test_user_serialization_roundtrip() {
        let user = make_user("test", "test@x.com", UserRole::User);
        let json = serde_json::to_string(&user).unwrap();
        let deser: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.username, "test");
        assert_eq!(deser.role, UserRole::User);
    }

    #[test]
    fn test_hash_password_deterministic_per_call() {
        let h1 = hash_password("test123").unwrap();
        let h2 = hash_password("test123").unwrap();
        assert_ne!(h1, h2); // bcrypt uses random salt
        assert!(verify_password("test123", &h1));
        assert!(verify_password("test123", &h2));
    }

    #[test]
    fn test_hash_password_empty() {
        let h = hash_password("").unwrap();
        assert!(verify_password("", &h));
    }

    #[test]
    fn test_verify_password_wrong() {
        let h = hash_password("correct").unwrap();
        assert!(!verify_password("wrong", &h));
    }

    #[tokio::test]
    async fn test_get_user_by_username_not_found() {
        let s = store();
        let err = s.get_user_by_username("nobody").await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_get_user_by_email_not_found() {
        let s = store();
        let err = s.get_user_by_email("nobody@x.com").await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let s = store();
        s.create_user(make_user("bob", "bob@x.com", UserRole::User))
            .await
            .unwrap();
        let user = s.get_user_by_email("bob@x.com").await.unwrap();
        assert_eq!(user.username, "bob");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let s = store();
        let err = s.delete_user("nonexistent").await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_update_nonexistent_user() {
        let s = store();
        let err = s
            .update_user("nonexistent", UpdateUserRequest::default())
            .await
            .unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_set_password_nonexistent_user() {
        let s = store();
        let err = s.set_password("nonexistent", "hash").await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_authenticate_nonexistent_user() {
        let s = store();
        let err = s.authenticate("nobody", "pass").await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_update_user_email_conflict() {
        let s = store();
        s.create_user(make_user("u1", "u1@x.com", UserRole::User))
            .await
            .unwrap();
        s.create_user(make_user("u2", "u2@x.com", UserRole::User))
            .await
            .unwrap();
        let u2 = s.get_user_by_username("u2").await.unwrap();
        let err = s
            .update_user(
                &u2.id,
                UpdateUserRequest {
                    email: Some("u1@x.com".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.kind, UserErrorKind::Conflict);
    }

    #[tokio::test]
    async fn test_update_user_email_to_empty() {
        let s = store();
        s.create_user(make_user("u1", "u1@x.com", UserRole::User))
            .await
            .unwrap();
        let u1 = s.get_user_by_username("u1").await.unwrap();
        s.update_user(
            &u1.id,
            UpdateUserRequest {
                email: Some("".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(s.get_user_by_email("u1@x.com").await.is_err());
    }

    #[tokio::test]
    async fn test_update_user_display_name() {
        let s = store();
        s.create_user(make_user("u1", "u1@x.com", UserRole::User))
            .await
            .unwrap();
        let u1 = s.get_user_by_username("u1").await.unwrap();
        let updated = s
            .update_user(
                &u1.id,
                UpdateUserRequest {
                    display_name: Some("New Name".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.display_name, "New Name");
    }

    #[tokio::test]
    async fn test_update_user_status() {
        let s = store();
        s.create_user(make_user("u1", "u1@x.com", UserRole::User))
            .await
            .unwrap();
        let u1 = s.get_user_by_username("u1").await.unwrap();
        let updated = s
            .update_user(
                &u1.id,
                UpdateUserRequest {
                    status: Some(UserStatus::Locked),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.status, UserStatus::Locked);
    }

    #[tokio::test]
    async fn test_update_user_storage_quota() {
        let s = store();
        s.create_user(make_user("u1", "u1@x.com", UserRole::User))
            .await
            .unwrap();
        let u1 = s.get_user_by_username("u1").await.unwrap();
        let updated = s
            .update_user(
                &u1.id,
                UpdateUserRequest {
                    storage_quota_bytes: Some(Some(1024 * 1024)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.storage_quota_bytes, Some(1024 * 1024));
    }

    #[tokio::test]
    async fn test_update_user_clear_storage_quota() {
        let s = store();
        let mut user = make_user("u1", "u1@x.com", UserRole::User);
        user.storage_quota_bytes = Some(1024);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();
        let updated = s
            .update_user(
                &id,
                UpdateUserRequest {
                    storage_quota_bytes: Some(None),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.storage_quota_bytes, None);
    }

    #[tokio::test]
    async fn test_create_user_empty_email() {
        let s = store();
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "noemail".to_string(),
            display_name: "No Email".to_string(),
            email: String::new(),
            role: UserRole::User,
            created_at: Utc::now(),
            last_login: None,
            status: UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ZeroizeString::new(hash_password("pass").unwrap())),
            totp_secret: None,
            totp_enabled: false,
        };
        let result = s.create_user(user).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_load_from_db() {
        let s = InMemoryUserStore::new();
        let user = make_user("loaded", "loaded@x.com", UserRole::Admin);
        s.load_from_db(user.clone());
        let fetched = s.get_user_by_username("loaded").await.unwrap();
        assert_eq!(fetched.id, user.id);
        assert!(s.get_user_by_email("loaded@x.com").await.is_ok());
    }

    #[tokio::test]
    async fn test_load_from_db_empty_email() {
        let s = InMemoryUserStore::new();
        let mut user = make_user("noemail", "", UserRole::User);
        user.email = String::new();
        s.load_from_db(user);
        let user = s.get_user_by_username("noemail").await.unwrap();
        assert!(user.email.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_user_store_default() {
        let s = InMemoryUserStore::default();
        assert!(s.list_users().await.is_empty());
    }

    #[test]
    fn test_user_debug_format() {
        let user = make_user("test", "test@x.com", UserRole::User);
        let debug = format!("{:?}", user);
        assert!(debug.contains("User"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_user_info_debug_format() {
        let user = make_user("test", "test@x.com", UserRole::User);
        let info = UserInfo::from(&user);
        let debug = format!("{:?}", info);
        assert!(debug.contains("UserInfo"));
    }

    #[test]
    fn test_create_user_request_deserialization() {
        let json = r#"{"username":"alice","display_name":"Alice","email":"alice@x.com","password":"pass123"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.username, "alice");
        assert_eq!(req.email, "alice@x.com");
    }

    #[test]
    fn test_update_user_request_default() {
        let req = UpdateUserRequest::default();
        assert!(req.display_name.is_none());
        assert!(req.email.is_none());
        assert!(req.role.is_none());
        assert!(req.status.is_none());
        assert!(req.storage_quota_bytes.is_none());
    }

    #[test]
    fn test_user_serialization_skips_password_hash() {
        let user = make_user("test", "test@x.com", UserRole::User);
        let json = serde_json::to_string(&user).unwrap();
        assert!(!json.contains("password_hash"));
    }

    #[test]
    fn test_user_with_totp_secret() {
        let mut user = make_user("test", "test@x.com", UserRole::User);
        user.totp_secret = Some(ZeroizeString::new("secret123".to_string()));
        user.totp_enabled = true;
        let json = serde_json::to_string(&user).unwrap();
        // totp_secret is serialized (skip_serializing_if = Option::is_none)
        assert!(json.contains("totp_enabled"));
    }
}
