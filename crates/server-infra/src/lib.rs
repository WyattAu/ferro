pub mod api_federation;
pub mod circuit_breaker;
pub mod federation_sync;
pub mod metadata_replication;
#[cfg(feature = "pg")]
pub mod pg_state;
#[cfg(feature = "redis")]
pub mod redis_lock;

use common::storage::StorageEngine;
use std::sync::Arc;

pub use ferro_server_security::error::ApiError;

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

pub trait InfraState: Clone + Send + Sync + 'static {
    fn federation_secret(&self) -> &str;
    fn external_url(&self) -> &str;
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore>;
}

impl<S: InfraState> InfraState for Arc<S> {
    fn federation_secret(&self) -> &str {
        (**self).federation_secret()
    }
    fn external_url(&self) -> &str {
        (**self).external_url()
    }
    fn storage(&self) -> &Arc<dyn StorageEngine> {
        (**self).storage()
    }
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore> {
        (**self).activity_store()
    }
}
