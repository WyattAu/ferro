use crate::audit::AuditLog;
use std::sync::Arc;

/// Adapter to bridge the server's AuditLog to the collaboration crate's AuditLogTrait.
pub(super) struct CollaborationAuditLogAdapter(pub Arc<AuditLog>);

#[async_trait::async_trait]
impl ferro_server_collaboration::AuditLogTrait for CollaborationAuditLogAdapter {
    async fn log(&self, entry: ferro_server_collaboration::AuditEntry) {
        self.0
            .log(crate::audit::AuditEntry {
                timestamp: entry.timestamp,
                method: entry.method,
                path: entry.path,
                user: entry.user,
                status: entry.status,
                client_ip: entry.client_ip,
                user_agent: entry.user_agent,
                content_length: entry.content_length,
            })
            .await;
    }
}

/// Adapter to bridge the server's AuditLog to the user-mgmt crate's AuditLog trait.
pub(super) struct UserMgmtAuditLogAdapter(pub Arc<AuditLog>);

impl ferro_server_user_mgmt::AuditLog for UserMgmtAuditLogAdapter {
    fn log(
        &self,
        entry: ferro_server_user_mgmt::AuditEntry,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let audit = self.0.clone();
        Box::pin(async move {
            audit
                .log(crate::audit::AuditEntry {
                    timestamp: entry.timestamp,
                    method: entry.method,
                    path: entry.path,
                    user: entry.user,
                    status: entry.status,
                    client_ip: entry.client_ip,
                    user_agent: entry.user_agent,
                    content_length: entry.content_length,
                })
                .await;
        })
    }
}

/// Adapter to bridge the server's AuditLog to the compliance crate's AuditLogTrait.
pub(super) struct AuditLogAdapter(pub Arc<AuditLog>);

#[async_trait::async_trait]
impl ferro_server_compliance::AuditLogTrait for AuditLogAdapter {
    async fn log(&self, entry: ferro_server_compliance::AuditEntry) {
        self.0
            .log(crate::audit::AuditEntry {
                timestamp: entry.timestamp,
                method: entry.method,
                path: entry.path,
                user: entry.user,
                status: entry.status,
                client_ip: entry.client_ip,
                user_agent: entry.user_agent,
                content_length: entry.content_length,
            })
            .await;
    }
}

/// Adapter to bridge the server's AuditLog to the admin crate's AuditLogTrait.
pub(super) struct AdminAuditLogAdapter(pub Arc<AuditLog>);

#[async_trait::async_trait]
impl ferro_server_admin_api::AuditLogTrait for AdminAuditLogAdapter {
    async fn log(&self, entry: ferro_server_admin_api::AuditEntry) {
        self.0
            .log(crate::audit::AuditEntry {
                timestamp: entry.timestamp,
                method: entry.method,
                path: entry.path,
                user: entry.user,
                status: entry.status,
                client_ip: entry.client_ip,
                user_agent: entry.user_agent,
                content_length: entry.content_length,
            })
            .await;
    }

    async fn entries(&self) -> Vec<ferro_server_admin_api::AuditEntry> {
        self.0
            .recent_with_offset(10000, 0)
            .await
            .into_iter()
            .map(|e| ferro_server_admin_api::AuditEntry {
                timestamp: e.timestamp,
                method: e.method,
                path: e.path,
                user: e.user,
                status: e.status,
                client_ip: e.client_ip,
                user_agent: e.user_agent,
                content_length: e.content_length,
            })
            .collect()
    }

    async fn verify_chain(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Adapter to bridge the server's ShareStoreTrait to the admin crate's AdminShareStoreTrait.
pub(super) struct AdminShareStoreAdapter(pub Arc<dyn crate::shares::ShareStoreTrait>);

#[async_trait::async_trait]
impl ferro_server_admin_api::AdminShareStoreTrait for AdminShareStoreAdapter {
    async fn list(&self) -> Vec<ferro_server_admin_api::AdminShareLink> {
        self.0
            .list()
            .await
            .into_iter()
            .map(|s| ferro_server_admin_api::AdminShareLink {
                token: s.token,
                path: s.path,
                expires_at: s.expires_at.to_rfc3339(),
                max_downloads: s.max_downloads,
                download_count: s.download_count,
                created_by: s.created_by,
                allow_download: s.allow_download,
                allow_upload: s.allow_upload,
            })
            .collect()
    }

    async fn delete(&self, token: &str) -> bool {
        self.0.delete(token).await
    }
}

/// Adapter to bridge the server's FavoriteStore to the admin crate's AdminFavoriteStoreTrait.
pub(super) struct AdminFavoriteStoreAdapter(pub Arc<dyn crate::favorites::FavoriteStore>);

#[async_trait::async_trait]
impl ferro_server_admin_api::AdminFavoriteStoreTrait for AdminFavoriteStoreAdapter {
    async fn list(&self) -> Vec<String> {
        self.0.list().await
    }

    async fn remove(&self, path: &str) {
        self.0.remove(path).await
    }
}

/// Adapter to bridge the server's TagStore to the admin crate's AdminTagStoreTrait.
pub(super) struct AdminTagStoreAdapter(pub Arc<ferro_server_collaboration::tags::TagStore>);

impl ferro_server_admin_api::AdminTagStoreTrait for AdminTagStoreAdapter {
    fn all_tags(&self) -> Vec<(String, Vec<String>)> {
        self.0
            .entries
            .iter()
            .map(|entry| {
                let (path, tags) = entry.pair();
                (path.clone(), tags.iter().cloned().collect())
            })
            .collect()
    }

    fn all_tag_pairs(&self) -> Vec<(String, String)> {
        self.0
            .entries
            .iter()
            .flat_map(|entry| {
                let (path, tags) = entry.pair();
                tags.iter().map(|tag| (path.clone(), tag.clone())).collect::<Vec<_>>()
            })
            .collect()
    }

    fn remove_tag(&self, path: &str, tag: &str) -> bool {
        self.0.remove_tag(path, tag)
    }
}
