use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
