//! Session management integration.
//!
//! Provides helpers for creating and validating user sessions.

use ferro_session_manager::{SessionManager, SessionConfig};

pub fn create_session_manager() -> SessionManager {
    SessionManager::new(SessionConfig::default())
}

pub fn create_session_manager_with_config(config: SessionConfig) -> SessionManager {
    SessionManager::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_session_manager::DeviceInfo;

    #[test]
    fn test_create_manager() {
        let mgr = create_session_manager();
        assert_eq!(mgr.active_count("user-1"), 0);
    }

    #[test]
    fn test_create_session() {
        let mgr = create_session_manager();
        let device = DeviceInfo::new("device-1".to_string(), "test-agent".to_string())
            .with_name("Test Device".to_string());
        let result = mgr.create_session("user-1", device, None, std::time::Duration::from_secs(3600));
        assert!(result.is_ok());
        let (session_id, _token) = result.unwrap();
        assert!(!session_id.is_empty());
        let session = mgr.get_session(&session_id);
        assert!(session.is_some());
    }
}
