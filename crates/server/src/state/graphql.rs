use crate::state::AppState;

impl AppState {
    /// Build a [`ferro_graphql::GraphQLContext`] from this state.
    pub fn graphql_context(&self) -> ferro_graphql::GraphQLContext {
        let storage = self.storage.clone();
        let share_store = self.share_store.clone();
        let audit_log = self.audit_log.clone();
        let storage2 = storage.clone();
        let storage3 = storage.clone();
        let storage4 = storage.clone();
        ferro_graphql::GraphQLContext {
            list_files: Box::new(move |prefix: &str| {
                let storage = storage.clone();
                let prefix = prefix.to_string();
                Box::pin(async move { storage.list(&prefix).await.map_err(|e| e.to_string()) })
            }),
            head_file: Box::new(move |path: &str| {
                let storage = storage2.clone();
                let path = path.to_string();
                Box::pin(async move { storage.head(&path).await.map_err(|e| e.to_string()) })
            }),
            create_collection: Box::new(move |path: &str, owner: &str| {
                let storage = storage3.clone();
                let path = path.to_string();
                let owner = owner.to_string();
                Box::pin(async move {
                    storage
                        .create_collection(&path, &owner)
                        .await
                        .map_err(|e| e.to_string())
                })
            }),
            delete_file: Box::new(move |path: &str| {
                let storage = storage4.clone();
                let path = path.to_string();
                Box::pin(async move { storage.delete(&path).await.map_err(|e| e.to_string()) })
            }),
            list_shares: Box::new(move || {
                let share_store = share_store.clone();
                Box::pin(async move {
                    share_store
                        .list()
                        .await
                        .into_iter()
                        .map(|l| ferro_graphql::ShareEntry {
                            token: l.token,
                            path: l.path,
                            expires_at: l.expires_at.to_string(),
                            password_protected: l.password.is_some(),
                            max_downloads: l.max_downloads,
                            download_count: l.download_count,
                            created_by: l.created_by,
                        })
                        .collect()
                })
            }),
            recent_audit: Box::new(move |limit: usize, offset: usize| {
                let audit_log = audit_log.clone();
                Box::pin(async move {
                    audit_log
                        .recent_with_offset(limit, offset)
                        .await
                        .into_iter()
                        .map(|e| ferro_graphql::AuditEntry {
                            method: e.method,
                            path: e.path,
                            user: e.user,
                            status: e.status,
                            timestamp: e.timestamp,
                        })
                        .collect()
                })
            }),
            current_user: None,
        }
    }
}
