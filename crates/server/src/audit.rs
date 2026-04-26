use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

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
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn log(&self, entry: AuditEntry) {
        info!(
            method = %entry.method,
            path = %entry.path,
            user = %entry.user,
            status = entry.status,
            "AUDIT"
        );
        self.entries.write().await.push(entry);
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
        self.entries.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    pub async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        let len = entries.len();
        if offset >= len {
            return vec![];
        }
        let end = (offset + limit).min(len);
        entries[offset..end].to_vec()
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
