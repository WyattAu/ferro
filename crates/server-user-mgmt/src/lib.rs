pub mod account_api;
pub mod guests;
pub mod totp_api;
pub mod user_api;

use std::sync::Arc;

use ferro_auth::users::{UserInfo, UserStoreTrait};

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

// Re-export types needed by handlers
pub use ferro_auth::users::{
    CreateUserRequest, ResetPasswordRequest, UpdateSelfRequest, UpdateUserRequest, User, UserError,
    UserErrorKind, UserRole, UserStatus, hash_password,
};
pub use ferro_server_security::ApiError;

// Blanket impl: Arc<S> delegates to S when S: UserMgmtState
impl<S: UserMgmtState> UserMgmtState for Arc<S> {
    fn user_info(&self, username: &str) -> Option<ferro_auth::users::UserInfo> {
        (**self).user_info(username)
    }
    fn admin_user(&self) -> &Option<String> {
        (**self).admin_user()
    }
    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait> {
        (**self).user_store()
    }
    fn db(&self) -> &Option<DbHandle> {
        (**self).db()
    }
    fn audit_log(&self) -> &Arc<dyn AuditLog> {
        (**self).audit_log()
    }
    fn push_notification_store(
        &self,
    ) -> &Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    > {
        (**self).push_notification_store()
    }
    fn push_notification_config(
        &self,
    ) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
        (**self).push_notification_config()
    }
}

/// A single audit log entry (subset of fields used by user management).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub content_length: Option<u64>,
}

/// Adapter trait for audit logging that doesn't pull in the full server AuditLog.
pub trait AuditLog: Send + Sync {
    fn log(
        &self,
        entry: AuditEntry,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>;
}

/// Trait abstracting AppState for user management handlers.
/// The server crate implements this for its AppState.
pub trait UserMgmtState: Clone + Send + Sync + 'static {
    fn user_info(&self, username: &str) -> Option<UserInfo>;
    fn admin_user(&self) -> &Option<String>;
    fn user_store(&self) -> &Arc<dyn UserStoreTrait>;
    fn db(&self) -> &Option<DbHandle>;
    fn audit_log(&self) -> &Arc<dyn AuditLog>;
    fn push_notification_store(
        &self,
    ) -> &Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    >;
    fn push_notification_config(
        &self,
    ) -> &ferro_server_integrations::push_notifications::PushNotificationConfig;
}
