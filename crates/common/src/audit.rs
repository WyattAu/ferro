use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

const MAX_AUDIT_ENTRIES: usize = 10_000;

// ── Persisted types ────────────────────────────────────────────────────

/// A single audit log entry stored in a persistence backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub content_length: Option<u64>,
    pub chain_hash: Option<String>,
}

/// Report from verifying audit log chain hash integrity.
#[derive(Debug, Clone, Serialize)]
pub struct ChainVerificationReport {
    pub total_entries: usize,
    pub verified: usize,
    pub mismatches: usize,
    pub skipped_no_hash: usize,
    pub findings: Vec<ChainMismatch>,
}

/// A single chain hash mismatch found during verification.
#[derive(Debug, Clone, Serialize)]
pub struct ChainMismatch {
    pub entry_id: i64,
    pub stored_hash: String,
    pub computed_hash: String,
    pub description: String,
}

// ── Persistence trait ──────────────────────────────────────────────────

/// Persistence backend for audit log entries (e.g. SQLite).
#[async_trait]
pub trait AuditLogPersistence: Send + Sync {
    async fn log(&self, entry: PersistedAuditEntry) -> Result<(), String>;
    async fn count(&self) -> usize;
    async fn recent(&self, limit: usize) -> Vec<PersistedAuditEntry>;
    async fn verify_audit_chain(&self) -> Option<ChainVerificationReport>;
}

/// HTTP audit log entry.
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

/// Minimal audit log trait for handlers that need to record audit events.
#[async_trait]
pub trait AuditLogTrait: Send + Sync {
    async fn log(&self, entry: AuditEntry);
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

// ── AuditLog struct ────────────────────────────────────────────────────

/// In-memory audit log with optional persistence backend.
pub struct AuditLog {
    entries: Arc<RwLock<VecDeque<AuditEntry>>>,
    persistence: Option<Arc<dyn AuditLogPersistence>>,
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLog")
            .field("persistence", &self.persistence.is_some())
            .finish()
    }
}

impl AuditLog {
    /// Create a new in-memory audit log.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::new())),
            persistence: None,
        }
    }

    /// Add an optional persistence backend.
    pub fn with_persistence(mut self, persistence: Arc<dyn AuditLogPersistence>) -> Self {
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
                .log(PersistedAuditEntry {
                    id: 0,
                    timestamp: entry.timestamp,
                    method: entry.method,
                    path: entry.path,
                    user: entry.user,
                    status: entry.status,
                    client_ip: entry.client_ip,
                    user_agent: entry.user_agent,
                    content_length: entry.content_length,
                    chain_hash: None,
                })
                .await
        {
            warn!(error = %e, "audit log persistence failed");
        }
    }

    /// Return all in-memory audit entries.
    pub async fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().await.iter().cloned().collect()
    }

    /// Return the most recent audit entries from in-memory buffer.
    pub async fn recent_entries(&self, limit: usize) -> Vec<AuditEntry> {
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

    /// Return the total number of audit entries (from persistence if available,
    /// otherwise from in-memory buffer).
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
    /// Returns `None` if persistence is not configured.
    pub async fn verify_chain(&self) -> Option<ChainVerificationReport> {
        if let Some(ref p) = self.persistence {
            p.verify_audit_chain().await
        } else {
            None
        }
    }

    /// Return audit entries with pagination offset.
    pub async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<AuditEntry> {
        if let Some(ref p) = self.persistence {
            let persisted = p.recent(limit).await;
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

/// Audit event level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub level: AuditLevel,
    pub event: String,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resource: String,
    pub action: String,
    pub status: String,
    pub details: HashMap<String, serde_json::Value>,
}

/// Audit logger
pub struct AuditLogger {
    enabled: bool,
    events: Vec<AuditEvent>,
}

impl AuditLogger {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            events: Vec::new(),
        }
    }

    /// Log an event
    pub fn log_event(&mut self, event: AuditEvent) {
        if self.enabled {
            self.events.push(event);
        }
    }

    /// Create an info event
    pub fn info(&mut self, event: &str, user_id: Option<String>, resource: &str, action: &str) {
        self.log_event(AuditEvent {
            timestamp: Utc::now(),
            level: AuditLevel::Info,
            event: event.to_string(),
            user_id,
            ip_address: None,
            user_agent: None,
            resource: resource.to_string(),
            action: action.to_string(),
            status: "success".to_string(),
            details: HashMap::new(),
        });
    }

    /// Create a warning event
    pub fn warning(
        &mut self,
        event: &str,
        user_id: Option<String>,
        resource: &str,
        action: &str,
        details: HashMap<String, serde_json::Value>,
    ) {
        self.log_event(AuditEvent {
            timestamp: Utc::now(),
            level: AuditLevel::Warning,
            event: event.to_string(),
            user_id,
            ip_address: None,
            user_agent: None,
            resource: resource.to_string(),
            action: action.to_string(),
            status: "warning".to_string(),
            details,
        });
    }

    /// Create an error event
    pub fn error(&mut self, event: &str, user_id: Option<String>, resource: &str, action: &str, error: &str) {
        let mut details = HashMap::new();
        details.insert("error".to_string(), serde_json::Value::String(error.to_string()));

        self.log_event(AuditEvent {
            timestamp: Utc::now(),
            level: AuditLevel::Error,
            event: event.to_string(),
            user_id,
            ip_address: None,
            user_agent: None,
            resource: resource.to_string(),
            action: action.to_string(),
            status: "error".to_string(),
            details,
        });
    }

    /// Get all events
    pub fn get_events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Get events by level
    pub fn get_events_by_level(&self, level: &AuditLevel) -> Vec<&AuditEvent> {
        self.events.iter().filter(|e| e.level == *level).collect()
    }

    /// Get events by user
    pub fn get_events_by_user(&self, user_id: &str) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .collect()
    }

    /// Clear events
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event() {
        let mut logger = AuditLogger::new(true);

        logger.info("user.login", Some("user1".to_string()), "/auth/login", "POST");

        let events = logger.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, AuditLevel::Info);
    }

    #[test]
    fn test_get_events_by_level() {
        let mut logger = AuditLogger::new(true);

        logger.info("user.login", Some("user1".to_string()), "/auth/login", "POST");

        logger.warning(
            "user.login_failed",
            Some("user1".to_string()),
            "/auth/login",
            "POST",
            HashMap::new(),
        );

        let info_events = logger.get_events_by_level(&AuditLevel::Info);
        assert_eq!(info_events.len(), 1);

        let warning_events = logger.get_events_by_level(&AuditLevel::Warning);
        assert_eq!(warning_events.len(), 1);
    }

    #[test]
    fn test_get_events_by_user() {
        let mut logger = AuditLogger::new(true);

        logger.info("user.login", Some("user1".to_string()), "/auth/login", "POST");

        logger.info("user.login", Some("user2".to_string()), "/auth/login", "POST");

        let user1_events = logger.get_events_by_user("user1");
        assert_eq!(user1_events.len(), 1);

        let user2_events = logger.get_events_by_user("user2");
        assert_eq!(user2_events.len(), 1);
    }
}
