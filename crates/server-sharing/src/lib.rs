pub mod api_error;
pub mod comments;
pub mod favorites;
pub mod federation;
pub mod guests;
pub mod shares;
pub mod shares_ext;
pub mod tags;

use std::sync::Arc;

use comments::CommentStore;
use common::storage::StorageEngine;
use favorites::FavoriteStore;
use shares::ShareStoreTrait;
use tags::TagStore;

#[async_trait::async_trait]
pub trait AuditLogTrait: Send + Sync {
    async fn log_audit(&self, entry: audit::AuditEntry);
    async fn recent_audit(&self, limit: usize) -> Vec<audit::AuditEntry>;
}

pub mod audit {
    use chrono::Utc;
    use ferro_core::persistence::AuditLogStore;
    use serde::Serialize;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use tracing::warn;

    const MAX_AUDIT_ENTRIES: usize = 10_000;

    #[derive(Debug, Clone, Serialize)]
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

    pub struct AuditLog {
        entries: Arc<RwLock<VecDeque<AuditEntry>>>,
        persistence: Option<Arc<ferro_core::persistence::SqlitePersistence>>,
    }

    impl AuditLog {
        pub fn new() -> Self {
            Self {
                entries: Arc::new(RwLock::new(VecDeque::new())),
                persistence: None,
            }
        }

        pub fn with_persistence(
            mut self,
            persistence: Arc<ferro_core::persistence::SqlitePersistence>,
        ) -> Self {
            self.persistence = Some(persistence);
            self
        }

        pub async fn log(&self, entry: AuditEntry) {
            {
                let mut entries = self.entries.write().await;
                entries.push_back(entry.clone());
                if entries.len() > MAX_AUDIT_ENTRIES {
                    let excess = entries.len() - MAX_AUDIT_ENTRIES;
                    entries.drain(..excess);
                }
            }

            if let Some(ref p) = self.persistence
                && let Err(e) = p
                    .log(ferro_core::persistence::PersistedAuditEntry {
                        id: 0,
                        timestamp: entry.timestamp.clone(),
                        method: entry.method.clone(),
                        path: entry.path.clone(),
                        user: entry.user.clone(),
                        status: entry.status,
                        client_ip: entry.client_ip.clone(),
                        user_agent: entry.user_agent.clone(),
                        content_length: entry.content_length,
                        chain_hash: None,
                    })
                    .await
            {
                warn!(error = %e, "audit log persistence failed");
            }
        }

        pub async fn recent(&self, limit: usize) -> Vec<AuditEntry> {
            let entries = self.entries.read().await;
            entries
                .iter()
                .rev()
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        }

        pub async fn len(&self) -> usize {
            if let Some(ref p) = self.persistence {
                p.count().await
            } else {
                self.entries.read().await.len()
            }
        }

        pub async fn is_empty(&self) -> bool {
            self.len().await == 0
        }
    }

    impl Default for AuditLog {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait::async_trait]
    impl crate::AuditLogTrait for AuditLog {
        async fn log_audit(&self, entry: AuditEntry) {
            self.log(entry).await;
        }

        async fn recent_audit(&self, limit: usize) -> Vec<AuditEntry> {
            self.recent(limit).await
        }
    }

    pub fn build_audit_entry(
        method: &str,
        path: &str,
        user: &str,
        status: u16,
        client_ip: Option<String>,
        user_agent: Option<String>,
    ) -> AuditEntry {
        AuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            method: method.to_string(),
            path: path.to_string(),
            user: user.to_string(),
            status,
            client_ip,
            user_agent,
            content_length: None,
        }
    }
}

pub mod db {
    use rusqlite::Connection;
    use std::sync::Arc;

    pub type DbHandle = Arc<std::sync::Mutex<Connection>>;
}

pub mod security {
    pub fn sanitize_control_chars(input: &str) -> String {
        input
            .chars()
            .map(|c| if c.is_control() || c == '\0' { ' ' } else { c })
            .collect()
    }

    pub fn contains_html(input: &str) -> bool {
        let lower = input.to_lowercase();
        lower.contains("<script")
            || lower.contains("</script")
            || lower.contains("onerror=")
            || lower.contains("onload=")
            || lower.contains("onclick=")
            || lower.contains("onmouseover=")
            || lower.contains("javascript:")
            || lower.contains("<iframe")
            || lower.contains("<img")
            || lower.contains("<svg")
            || lower.contains("<object")
            || lower.contains("<embed")
            || lower.contains("<link")
            || lower.contains("<style")
            || lower.contains("alert(")
            || lower.contains("document.")
            || lower.contains("window.")
    }
}

/// Trait for state that the sharing crate handlers need access to.
///
/// The ferro-server crate implements this for its `AppState`, allowing the
/// sharing crate to remain decoupled from concrete server types.
#[async_trait::async_trait]
pub trait SharingStateTrait: Send + Sync {
    fn share_store(&self) -> &Arc<dyn ShareStoreTrait>;
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn audit_log(&self) -> &Arc<dyn AuditLogTrait>;
    fn db(&self) -> &Option<db::DbHandle>;
    fn max_body_size(&self) -> u64;
    fn tags(&self) -> &Arc<TagStore>;
    fn comments(&self) -> &Arc<CommentStore>;
    fn favorites(&self) -> &Arc<dyn FavoriteStore>;
    fn admin_user(&self) -> Option<&str>;
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore>;
    fn external_url(&self) -> &str;
    fn federation_secret(&self) -> &str;
    #[allow(clippy::type_complexity)]
    fn on_share_created(
        &self,
    ) -> &Option<Arc<dyn Fn(&str, &str) -> futures::future::BoxFuture<'static, ()> + Send + Sync>>;
}

#[derive(Clone)]
pub struct SharingState {
    pub share_store: Arc<dyn ShareStoreTrait>,
    pub storage: Arc<dyn StorageEngine>,
    pub audit_log: Arc<dyn AuditLogTrait>,
    pub db: Option<db::DbHandle>,
    pub max_body_size: u64,
    pub tags: Arc<TagStore>,
    pub comments: Arc<CommentStore>,
    pub favorites: Arc<dyn FavoriteStore>,
    pub admin_user: Option<String>,
    pub activity_store: Arc<ferro_server_activitypub::store::ActivityStore>,
    pub external_url: String,
    pub federation_secret: String,
    #[allow(clippy::type_complexity)]
    pub on_share_created:
        Option<Arc<dyn Fn(&str, &str) -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
}

#[async_trait::async_trait]
impl SharingStateTrait for SharingState {
    fn share_store(&self) -> &Arc<dyn ShareStoreTrait> {
        &self.share_store
    }
    fn storage(&self) -> &Arc<dyn StorageEngine> {
        &self.storage
    }
    fn audit_log(&self) -> &Arc<dyn AuditLogTrait> {
        &self.audit_log
    }
    fn db(&self) -> &Option<db::DbHandle> {
        &self.db
    }
    fn max_body_size(&self) -> u64 {
        self.max_body_size
    }
    fn tags(&self) -> &Arc<TagStore> {
        &self.tags
    }
    fn comments(&self) -> &Arc<CommentStore> {
        &self.comments
    }
    fn favorites(&self) -> &Arc<dyn FavoriteStore> {
        &self.favorites
    }
    fn admin_user(&self) -> Option<&str> {
        self.admin_user.as_deref()
    }
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore> {
        &self.activity_store
    }
    fn external_url(&self) -> &str {
        &self.external_url
    }
    fn federation_secret(&self) -> &str {
        &self.federation_secret
    }
    fn on_share_created(
        &self,
    ) -> &Option<Arc<dyn Fn(&str, &str) -> futures::future::BoxFuture<'static, ()> + Send + Sync>>
    {
        &self.on_share_created
    }
}
