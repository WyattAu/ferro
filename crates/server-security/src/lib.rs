pub mod api_keys;
pub mod clamav;
pub mod e2ee;
pub mod encryption;
pub mod error;
pub mod ransomware;
pub mod security;
pub mod totp;
#[cfg(feature = "webauthn")]
pub mod webauthn;

pub use error::ApiError;
pub use security::{AuthAttemptTracker, LoginRateLimiter};

use ferro_auth::api_keys::ApiKeyStoreTrait;
use ferro_auth::users::UserStoreTrait;
use ferro_common::storage::StorageEngine;
use std::sync::Arc;

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

pub trait SecurityAppState: Clone + Send + Sync + 'static {
    fn auth_attempt_tracker(&self) -> &Arc<AuthAttemptTracker>;
    fn login_rate_limiter(&self) -> &Arc<LoginRateLimiter>;
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn api_key_store(&self) -> &Arc<dyn ApiKeyStoreTrait>;
    fn admin_user(&self) -> &Option<String>;
    fn admin_password(&self) -> &Option<String>;
    fn user_store(&self) -> &Arc<dyn UserStoreTrait>;
    fn db(&self) -> &Option<DbHandle>;

    #[cfg(feature = "webauthn")]
    fn webauthn_store(&self) -> &Arc<tokio::sync::RwLock<ferro_auth::webauthn::WebAuthnStore>>;
}
