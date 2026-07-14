use chrono::Utc;
use ferro_core::persistence::AuditLogStore;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing::warn;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::AppState;

const MAX_AUDIT_ENTRIES: usize = 10_000;

/// A single audit log entry.
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

/// In-memory audit log with optional SQLite persistence.
#[derive(Debug)]
pub struct AuditLog {
    entries: Arc<RwLock<VecDeque<AuditEntry>>>,
    persistence: Option<Arc<ferro_core::persistence::SqlitePersistence>>,
}

impl AuditLog {
    /// Create a new in-memory audit log.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            persistence: None,
        }
    }

    /// Add optional SQLite persistence to this audit log.
    pub fn with_persistence(mut self, persistence: Arc<ferro_core::persistence::SqlitePersistence>) -> Self {
        self.persistence = Some(persistence);
        self
    }

    /// Record an audit entry.
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
                    chain_hash: None, // Computed by persistence layer
                })
                .await
        {
            warn!(error = %e, "audit log persistence failed");
        }
    }

    /// Return all audit entries.
    pub async fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().await.iter().cloned().collect()
    }

    /// Return the most recent audit entries.
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

    /// Return the total number of audit entries.
    pub async fn len(&self) -> usize {
        if let Some(ref p) = self.persistence {
            p.count().await
        } else {
            self.entries.read().await.len()
        }
    }

    /// Check whether the audit log is empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Verify the chain hash integrity of persisted audit entries.
    ///
    /// Returns `None` if persistence is not configured (in-memory-only mode).
    pub async fn verify_chain(&self) -> Option<ferro_core::persistence::ChainVerificationReport> {
        if let Some(ref p) = self.persistence {
            Some(p.verify_audit_chain().await)
        } else {
            None
        }
    }

    /// Return audit entries with pagination offset.
    pub async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<AuditEntry> {
        if let Some(ref p) = self.persistence {
            let persisted = p.recent(limit).await.unwrap_or_default();
            persisted
                .into_iter()
                .skip(offset)
                .map(|e| AuditEntry {
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
        } else {
            let entries = self.entries.read().await;
            entries.iter().skip(offset).take(limit).cloned().collect()
        }
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Build an audit entry from request details.
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

pub async fn audit_middleware(State(state): State<AppState>, req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let user = req
        .headers()
        .get("X-Ferro-User")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();
    let client_ip = req
        .headers()
        .get("X-Forwarded-For")
        .or_else(|| req.headers().get("X-Real-Ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(req).await;
    let status = response.status().as_u16();

    state
        .audit_log
        .log(build_audit_entry(&method, &path, &user, status, client_ip, user_agent))
        .await;

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_log_new_is_empty() {
        let log = AuditLog::new();
        assert!(log.is_empty().await);
        assert_eq!(log.len().await, 0);
    }

    #[tokio::test]
    async fn test_audit_log_entries_empty() {
        let log = AuditLog::new();
        let entries = log.entries().await;
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_audit_log_recent_empty() {
        let log = AuditLog::new();
        let entries = log.recent(10).await;
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_audit_log_log_and_entries() {
        let log = AuditLog::new();
        let entry = AuditEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            method: "PUT".to_string(),
            path: "/test.txt".to_string(),
            user: "admin".to_string(),
            status: 200,
            client_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            content_length: Some(100),
        };
        log.log(entry.clone()).await;
        assert!(!log.is_empty().await);
        assert_eq!(log.len().await, 1);

        let entries = log.entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].method, "PUT");
        assert_eq!(entries[0].path, "/test.txt");
        assert_eq!(entries[0].user, "admin");
    }

    #[tokio::test]
    async fn test_audit_log_recent_limit() {
        let log = AuditLog::new();
        for i in 0..20 {
            log.log(AuditEntry {
                timestamp: format!("2025-01-01T00:00:{:02}Z", i),
                method: "GET".to_string(),
                path: format!("/file-{}.txt", i),
                user: "user".to_string(),
                status: 200,
                client_ip: None,
                user_agent: None,
                content_length: None,
            })
            .await;
        }

        let recent = log.recent(5).await;
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].path, "/file-15.txt");
    }

    #[tokio::test]
    async fn test_audit_log_recent_with_offset() {
        let log = AuditLog::new();
        for i in 0..10 {
            log.log(AuditEntry {
                timestamp: format!("2025-01-01T00:00:{:02}Z", i),
                method: "GET".to_string(),
                path: format!("/file-{}.txt", i),
                user: "user".to_string(),
                status: 200,
                client_ip: None,
                user_agent: None,
                content_length: None,
            })
            .await;
        }

        let entries = log.recent_with_offset(3, 2).await;
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "/file-2.txt");
    }

    #[tokio::test]
    async fn test_audit_log_max_entries_eviction() {
        let log = AuditLog::new();
        for i in 0..10_010 {
            log.log(AuditEntry {
                timestamp: format!("2025-01-01T00:00:{:02}Z", i % 60),
                method: "PUT".to_string(),
                path: format!("/file-{}.txt", i),
                user: "user".to_string(),
                status: 200,
                client_ip: None,
                user_agent: None,
                content_length: None,
            })
            .await;
        }

        let len = log.len().await;
        assert!(len <= 10_000);
    }

    #[test]
    fn test_build_audit_entry() {
        let entry = build_audit_entry(
            "PUT",
            "/test.txt",
            "admin",
            200,
            Some("10.0.0.1".to_string()),
            Some("curl/7.68".to_string()),
        );
        assert_eq!(entry.method, "PUT");
        assert_eq!(entry.path, "/test.txt");
        assert_eq!(entry.user, "admin");
        assert_eq!(entry.status, 200);
        assert_eq!(entry.client_ip.as_deref(), Some("10.0.0.1"));
        assert_eq!(entry.user_agent.as_deref(), Some("curl/7.68"));
        assert!(entry.content_length.is_none());
    }

    #[test]
    fn test_build_audit_entry_none_optionals() {
        let entry = build_audit_entry("GET", "/", "anon", 404, None, None);
        assert_eq!(entry.method, "GET");
        assert!(entry.client_ip.is_none());
        assert!(entry.user_agent.is_none());
    }

    #[test]
    fn test_default_is_empty() {
        let log = AuditLog::default();
        assert!(log.entries.try_read().unwrap().is_empty());
    }
}
