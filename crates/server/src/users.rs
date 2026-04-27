use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum UserRole {
    #[default]
    Admin,
    User,
    ReadOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserStatus {
    Active,
    Disabled,
    Locked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub password_hash: Option<String>,
}

impl User {
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    pub fn has_read_write(&self) -> bool {
        self.role == UserRole::Admin || self.role == UserRole::User
    }
}

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

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub role: UserRole,
    pub storage_quota_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub storage_quota_bytes: Option<Option<u64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSelfRequest {
    pub display_name: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug)]
pub struct UserError {
    pub kind: UserErrorKind,
    pub message: String,
}

#[derive(Debug, PartialEq)]
pub enum UserErrorKind {
    NotFound,
    Conflict,
    Forbidden,
    BadRequest,
}

impl UserError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { kind: UserErrorKind::NotFound, message: msg.into() }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self { kind: UserErrorKind::Conflict, message: msg.into() }
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self { kind: UserErrorKind::Forbidden, message: msg.into() }
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self { kind: UserErrorKind::BadRequest, message: msg.into() }
    }
}

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

    fn get_user_by_username_blocking(&self, username: &str) -> Result<User, UserError> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.get_user_by_username(username))
    }
}

pub fn hash_password(password: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(b"ferro-salt-v1-");
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

pub struct InMemoryUserStore {
    users: DashMap<String, User>,
    username_index: DashMap<String, String>,
    email_index: DashMap<String, String>,
}

impl InMemoryUserStore {
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
            username_index: DashMap::new(),
            email_index: DashMap::new(),
        }
    }

    pub fn create_admin(username: &str, password: &str) -> User {
        User {
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
            password_hash: Some(hash_password(password)),
        }
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
        if self.username_index.contains_key(&user.username) {
            return Err(UserError::conflict(format!("Username '{}' already exists", user.username)));
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
        Ok(user)
    }

    async fn get_user(&self, id: &str) -> Result<User, UserError> {
        self.users.get(id)
            .map(|r| r.value().clone())
            .ok_or_else(|| UserError::not_found(format!("User '{}' not found", id)))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User, UserError> {
        let id = self.username_index.get(username)
            .map(|r| r.value().clone())
            .ok_or_else(|| UserError::not_found(format!("User '{}' not found", username)))?;
        self.get_user(&id).await
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User, UserError> {
        let id = self.email_index.get(email)
            .map(|r| r.value().clone())
            .ok_or_else(|| UserError::not_found(format!("No user with email '{}'", email)))?;
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
                    return Err(UserError::conflict(format!("Email '{}' already in use", new_email)));
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
        Ok(user)
    }

    async fn delete_user(&self, id: &str) -> Result<(), UserError> {
        let user = self.get_user(id).await?;
        self.users.remove(id);
        self.username_index.remove(&user.username);
        if !user.email.is_empty() {
            self.email_index.remove(&user.email);
        }
        Ok(())
    }

    async fn update_last_login(&self, id: &str) {
        if let Some(mut user) = self.users.get_mut(id) {
            user.last_login = Some(Utc::now());
        }
    }

    async fn set_password(&self, id: &str, password_hash: &str) -> Result<(), UserError> {
        let mut user = self.get_user(id).await?;
        user.password_hash = Some(password_hash.to_string());
        self.users.insert(id.to_string(), user);
        Ok(())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<User, UserError> {
        let user = self.get_user_by_username(username).await?;
        if !user.is_active() {
            return Err(UserError::forbidden("User account is not active"));
        }
        let hash = user.password_hash.as_deref()
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
            password_hash: Some(hash_password("testpass")),
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
        s.create_user(make_user("bob", "bob@example.com", UserRole::User)).await.unwrap();

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
        s.create_user(make_user("charlie", "c1@example.com", UserRole::User)).await.unwrap();
        let err = s.create_user(make_user("charlie", "c2@example.com", UserRole::User)).await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::Conflict);
    }

    #[tokio::test]
    async fn test_duplicate_email_rejected() {
        let s = store();
        s.create_user(make_user("dave", "dave@example.com", UserRole::User)).await.unwrap();
        let err = s.create_user(make_user("dave2", "dave@example.com", UserRole::User)).await.unwrap_err();
        assert_eq!(err.kind, UserErrorKind::Conflict);
    }

    #[tokio::test]
    async fn test_update_user_role() {
        let s = store();
        let user = make_user("eve", "eve@example.com", UserRole::ReadOnly);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        let updated = s.update_user(&id, UpdateUserRequest {
            role: Some(UserRole::Admin),
            ..Default::default()
        }).await.unwrap();

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
        s.create_user(make_user("grace", "grace@example.com", UserRole::User)).await.unwrap();

        let result = s.authenticate("grace", "testpass").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authenticate_wrong_password() {
        let s = store();
        s.create_user(make_user("heidi", "heidi@example.com", UserRole::User)).await.unwrap();

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
        s.update_user(&id, UpdateUserRequest {
            status: Some(UserStatus::Disabled),
            ..Default::default()
        }).await.unwrap();

        let result = s.authenticate("ivan", "testpass").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_users() {
        let s = store();
        s.create_user(make_user("u1", "u1@example.com", UserRole::User)).await.unwrap();
        s.create_user(make_user("u2", "u2@example.com", UserRole::Admin)).await.unwrap();
        s.create_user(make_user("u3", "u3@example.com", UserRole::ReadOnly)).await.unwrap();

        let users = s.list_users().await;
        assert_eq!(users.len(), 3);
    }

    #[tokio::test]
    async fn test_update_email_clears_old_index() {
        let s = store();
        let user = make_user("judy", "old@example.com", UserRole::User);
        let id = user.id.clone();
        s.create_user(user).await.unwrap();

        s.update_user(&id, UpdateUserRequest {
            email: Some("new@example.com".to_string()),
            ..Default::default()
        }).await.unwrap();

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

        s.set_password(&id, &hash_password("newpass")).await.unwrap();
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
        let admin = InMemoryUserStore::create_admin("admin", "secret");
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
}
