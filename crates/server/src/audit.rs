use chrono::Utc;
use ferro_core::persistence::AuditLogStore;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

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
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    persistence: Option<Arc<ferro_core::persistence::SqlitePersistence>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            persistence: None,
        }
    }

    pub fn with_persistence(mut self, persistence: Arc<ferro_core::persistence::SqlitePersistence>) -> Self {
        self.persistence = Some(persistence);
        self
    }

    pub async fn log(&self, entry: AuditEntry) {
        info!(
            method = %entry.method,
            path = %entry.path,
            user = %entry.user,
            status = entry.status,
            "AUDIT"
        );
        {
            let mut entries = self.entries.write().await;
            entries.push(entry.clone());
            if entries.len() > MAX_AUDIT_ENTRIES {
                let excess = entries.len() - MAX_AUDIT_ENTRIES;
                entries.drain(..excess);
            }
        }

        if let Some(ref p) = self.persistence {
            let _ = p.log(ferro_core::persistence::PersistedAuditEntry {
                id: 0,
                timestamp: entry.timestamp.clone(),
                method: entry.method.clone(),
                path: entry.path.clone(),
                user: entry.user.clone(),
                status: entry.status,
                client_ip: entry.client_ip.clone(),
                user_agent: entry.user_agent.clone(),
                content_length: entry.content_length,
            }).await;
        }
    }

    pub async fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().await.clone()
    }

    pub async fn recent(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        let len = entries.len();
        let start = len.saturating_sub(limit);
        entries[start..].to_vec()
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

    pub async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<AuditEntry> {
        if let Some(ref p) = self.persistence {
            let persisted = p.recent(limit).await.unwrap_or_default();
            persisted.into_iter().skip(offset).map(|e| AuditEntry {
                timestamp: e.timestamp,
                method: e.method,
                path: e.path,
                user: e.user,
                status: e.status,
                client_ip: e.client_ip,
                user_agent: e.user_agent,
                content_length: e.content_length,
            }).collect()
        } else {
            let entries = self.entries.read().await;
            let len = entries.len();
            if offset >= len {
                return vec![];
            }
            let end = (offset + limit).min(len);
            entries[offset..end].to_vec()
        }
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
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
