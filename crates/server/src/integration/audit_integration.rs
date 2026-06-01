use std::collections::HashMap;

use chrono::Utc;
use ferro_audit_log::{AuditAction, AuditLog, ResourceType, audit_log::AuditEntry};

pub fn create_audit_log() -> AuditLog {
    AuditLog::new_in_memory().expect("Failed to create audit log")
}

pub fn record_file_op(
    log: &AuditLog,
    action: AuditAction,
    actor_id: &str,
    path: &str,
    details: HashMap<String, serde_json::Value>,
) {
    let mut entry = AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now(),
        action,
        actor_id: actor_id.to_string(),
        resource_type: ResourceType::File,
        resource_id: path.to_string(),
        details,
        ip_address: None,
        user_agent: None,
        previous_hash: String::new(),
        hash: String::new(),
    };
    let _ = log.record(&mut entry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_audit_log() {
        let log = create_audit_log();
        assert!(log.count().unwrap() == 0);
    }

    #[test]
    fn test_record_and_count() {
        let log = create_audit_log();
        record_file_op(
            &log,
            AuditAction::FileCreate,
            "user-1",
            "/docs/report.pdf",
            HashMap::new(),
        );
        assert_eq!(log.count().unwrap(), 1);
    }

    #[test]
    fn test_chain_integrity() {
        let log = create_audit_log();
        record_file_op(
            &log,
            AuditAction::FileCreate,
            "user-1",
            "/a.txt",
            HashMap::new(),
        );
        record_file_op(
            &log,
            AuditAction::FileDelete,
            "user-1",
            "/a.txt",
            HashMap::new(),
        );
        let result = log.verify_chain().unwrap();
        assert!(result.valid);
        assert_eq!(result.total, 2);
    }
}
