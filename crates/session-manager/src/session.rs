use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::device::DeviceInfo;
use crate::error::SessionError;
use crate::token::{SessionToken, hash_token, verify_token_hash};

pub type SessionId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub device_info: DeviceInfo,
    pub ip_address: Option<String>,
    pub is_elevated: bool,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_concurrent_sessions: usize,
    pub default_ttl: Duration,
    pub max_ttl: Duration,
    pub rotation_required: bool,
    pub inactive_timeout: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 5,
            default_ttl: Duration::from_secs(24 * 60 * 60),
            max_ttl: Duration::from_secs(30 * 24 * 60 * 60),
            rotation_required: false,
            inactive_timeout: Duration::from_secs(7 * 24 * 60 * 60),
        }
    }
}

pub struct SessionManager {
    sessions: DashMap<String, Session>,
    user_sessions: DashMap<String, Vec<String>>,
    config: SessionConfig,
}

impl SessionManager {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            sessions: DashMap::new(),
            user_sessions: DashMap::new(),
            config,
        }
    }

    pub fn create_session(
        &self,
        user_id: &str,
        device: DeviceInfo,
        ip: Option<&str>,
        ttl: Duration,
    ) -> Result<(SessionId, SessionToken), SessionError> {
        let effective_ttl = if ttl > self.config.max_ttl {
            self.config.max_ttl
        } else {
            ttl
        };

        self.enforce_limit(user_id)?;

        let session_id = Uuid::new_v4().to_string();
        let token = SessionToken::new();
        let token_hash = hash_token(&token);
        let now = Utc::now();

        let session = Session {
            id: session_id.clone(),
            user_id: user_id.to_string(),
            token_hash,
            created_at: now,
            last_active: now,
            expires_at: now + TimeDelta::from_std(effective_ttl).unwrap_or(TimeDelta::days(30)),
            device_info: device,
            ip_address: ip.map(String::from),
            is_elevated: false,
        };

        let sid = session.id.clone();
        self.sessions.insert(sid.clone(), session);

        self.user_sessions
            .entry(user_id.to_string())
            .or_default()
            .push(sid);

        Ok((session_id, token))
    }

    pub fn validate_token(
        &self,
        user_id: &str,
        token: &SessionToken,
    ) -> Result<Session, SessionError> {
        let _token_hex = token.to_hex();

        let user_ids = match self.user_sessions.get(user_id) {
            Some(ids) => ids.clone(),
            None => return Err(SessionError::NotFound),
        };

        for sid in &user_ids {
            if let Some(mut session) = self.sessions.get_mut(sid)
                && verify_token_hash(token, &session.token_hash)
            {
                if session.is_expired() {
                    return Err(SessionError::Expired);
                }

                session.last_active = Utc::now();

                if self.config.inactive_timeout.as_secs() > 0 {
                    let inactive = Utc::now() - session.last_active;
                    if inactive
                        > TimeDelta::from_std(self.config.inactive_timeout)
                            .unwrap_or(TimeDelta::days(30))
                    {
                        return Err(SessionError::Expired);
                    }
                }

                if self.config.rotation_required && !session.is_elevated {
                    return Err(SessionError::TokenRotationRequired);
                }

                let clone = session.clone();
                drop(session);
                return Ok(clone);
            }
        }

        Err(SessionError::InvalidToken)
    }

    pub fn rotate_token(&self, session_id: &str) -> Result<SessionToken, SessionError> {
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or(SessionError::NotFound)?;

        if session.is_expired() {
            return Err(SessionError::Expired);
        }

        let new_token = SessionToken::new();
        session.token_hash = hash_token(&new_token);
        session.last_active = Utc::now();
        session.is_elevated = true;

        drop(session);
        Ok(new_token)
    }

    pub fn invalidate_session(&self, session_id: &str) -> Result<(), SessionError> {
        let (_session, user_id) = {
            let session = self
                .sessions
                .remove(session_id)
                .ok_or(SessionError::NotFound)?;
            let user_id = session.1.user_id.clone();
            (session, user_id)
        };

        if let Some(mut ids) = self.user_sessions.get_mut(&user_id) {
            ids.retain(|id| id != session_id);
            if ids.is_empty() {
                drop(ids);
                self.user_sessions.remove(&user_id);
            }
        }

        Ok(())
    }

    pub fn invalidate_all_user_sessions(&self, user_id: &str) -> Result<usize, SessionError> {
        let session_ids = match self.user_sessions.remove(user_id) {
            Some((_, ids)) => ids,
            None => return Ok(0),
        };

        let count = session_ids.len();
        for sid in &session_ids {
            self.sessions.remove(sid);
        }

        Ok(count)
    }

    pub fn invalidate_all_except(
        &self,
        user_id: &str,
        except_session_id: &str,
    ) -> Result<usize, SessionError> {
        let mut session_ids = match self.user_sessions.get_mut(user_id) {
            Some(ids) => ids.clone(),
            None => return Ok(0),
        };

        session_ids.retain(|id| id != except_session_id);

        let count = session_ids.len();

        for sid in &session_ids {
            self.sessions.remove(sid);
        }

        if let Some(mut ids) = self.user_sessions.get_mut(user_id) {
            ids.retain(|id| id == except_session_id);
        }

        Ok(count)
    }

    pub fn get_user_sessions(&self, user_id: &str) -> Vec<Session> {
        let ids = match self.user_sessions.get(user_id) {
            Some(ids) => ids.clone(),
            None => return Vec::new(),
        };

        ids.iter()
            .filter_map(|id| self.sessions.get(id).map(|s| s.clone()))
            .collect()
    }

    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    pub fn cleanup_expired(&self) -> usize {
        let expired_ids: Vec<(String, String)> = self
            .sessions
            .iter()
            .filter(|entry| entry.value().is_expired())
            .map(|entry| (entry.key().clone(), entry.value().user_id.clone()))
            .collect();

        let count = expired_ids.len();

        for (session_id, user_id) in &expired_ids {
            self.sessions.remove(session_id);
            if let Some(mut ids) = self.user_sessions.get_mut(user_id) {
                ids.retain(|id| id != session_id);
                if ids.is_empty() {
                    drop(ids);
                    self.user_sessions.remove(user_id);
                }
            }
        }

        count
    }

    pub fn active_count(&self, user_id: &str) -> usize {
        match self.user_sessions.get(user_id) {
            Some(ids) => ids
                .iter()
                .filter(|id| self.sessions.get(*id).is_some_and(|s| !s.is_expired()))
                .count(),
            None => 0,
        }
    }

    pub fn enforce_limit(&self, user_id: &str) -> Result<(), SessionError> {
        let active = self.active_count(user_id);
        if active >= self.config.max_concurrent_sessions {
            let mut oldest_id: Option<String> = None;
            let mut oldest_time = Utc::now();

            if let Some(ids) = self.user_sessions.get(user_id) {
                for sid in ids.iter() {
                    if let Some(session) = self.sessions.get(sid)
                        && session.created_at < oldest_time
                    {
                        oldest_time = session.created_at;
                        oldest_id = Some(sid.clone());
                    }
                }
            }

            if let Some(oldest) = oldest_id {
                let _ = self.invalidate_session(&oldest);
            } else {
                return Err(SessionError::MaxSessionsExceeded(
                    self.config.max_concurrent_sessions,
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device(device_id: &str) -> DeviceInfo {
        DeviceInfo::new(device_id.to_string(), "test-agent".to_string())
    }

    fn test_config() -> SessionConfig {
        SessionConfig::default()
    }

    fn short_ttl_config() -> SessionConfig {
        SessionConfig {
            default_ttl: Duration::from_millis(50),
            max_ttl: Duration::from_secs(1),
            max_concurrent_sessions: 2,
            rotation_required: false,
            inactive_timeout: Duration::from_secs(1),
        }
    }

    #[test]
    fn test_create_and_validate_session() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (session_id, token) = mgr
            .create_session(
                "user1",
                device,
                Some("127.0.0.1"),
                Duration::from_secs(3600),
            )
            .unwrap();

        let session = mgr.validate_token("user1", &token).unwrap();
        assert_eq!(session.id, session_id);
        assert_eq!(session.user_id, "user1");
        assert_eq!(session.ip_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(session.device_info.device_id, "dev1");
    }

    #[test]
    fn test_token_rotation() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (session_id, token) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        let new_token = mgr.rotate_token(&session_id).unwrap();
        assert_ne!(token.to_hex(), new_token.to_hex());

        let session = mgr.get_session(&session_id).unwrap();
        let rotated_session = mgr.validate_token("user1", &new_token).unwrap();
        assert_eq!(rotated_session.id, session_id);
        assert!(rotated_session.is_elevated);
        assert!(session.is_elevated);
    }

    #[test]
    fn test_old_token_invalid_after_rotation() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (session_id, token) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        let _new_token = mgr.rotate_token(&session_id).unwrap();

        let result = mgr.validate_token("user1", &token);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_expiry() {
        let mgr = SessionManager::new(short_ttl_config());
        let device = test_device("dev1");
        let (session_id, _token) = mgr
            .create_session("user1", device, None, Duration::from_millis(50))
            .unwrap();

        std::thread::sleep(Duration::from_millis(80));

        let session = mgr.get_session(&session_id).unwrap();
        assert!(session.is_expired());
    }

    #[test]
    fn test_validate_expired_session_fails() {
        let mgr = SessionManager::new(short_ttl_config());
        let device = test_device("dev1");
        let (_session_id, token) = mgr
            .create_session("user1", device, None, Duration::from_millis(50))
            .unwrap();

        std::thread::sleep(Duration::from_millis(80));

        let result = mgr.validate_token("user1", &token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::Expired));
    }

    #[test]
    fn test_concurrent_session_limit() {
        let mut config = test_config();
        config.max_concurrent_sessions = 2;
        let mgr = SessionManager::new(config);
        let device = test_device("dev1");

        let (_, t1) = mgr
            .create_session("user1", device.clone(), None, Duration::from_secs(3600))
            .unwrap();
        let (_, t2) = mgr
            .create_session("user1", device.clone(), None, Duration::from_secs(3600))
            .unwrap();

        let (_, t3) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        assert_eq!(mgr.active_count("user1"), 2);

        let valid_tokens = [&t1, &t2, &t3]
            .iter()
            .any(|t| mgr.validate_token("user1", t).is_ok());
        assert!(valid_tokens);
    }

    #[test]
    fn test_invalidate_single_session() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (session_id, token) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        mgr.invalidate_session(&session_id).unwrap();
        assert!(mgr.get_session(&session_id).is_none());
        assert!(mgr.validate_token("user1", &token).is_err());
        assert_eq!(mgr.active_count("user1"), 0);
    }

    #[test]
    fn test_invalidate_all_user_sessions() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (id1, t1) = mgr
            .create_session("user1", device.clone(), None, Duration::from_secs(3600))
            .unwrap();
        let (id2, t2) = mgr
            .create_session("user1", device.clone(), None, Duration::from_secs(3600))
            .unwrap();
        let (id3, _t3) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        let count = mgr.invalidate_all_user_sessions("user1").unwrap();
        assert_eq!(count, 3);

        assert!(mgr.get_session(&id1).is_none());
        assert!(mgr.get_session(&id2).is_none());
        assert!(mgr.get_session(&id3).is_none());
        assert!(mgr.validate_token("user1", &t1).is_err());
        assert!(mgr.validate_token("user1", &t2).is_err());
    }

    #[test]
    fn test_invalidate_all_except() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (keep_id, keep_token) = mgr
            .create_session("user1", device.clone(), None, Duration::from_secs(3600))
            .unwrap();
        let (remove_id, remove_token) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        let count = mgr.invalidate_all_except("user1", &keep_id).unwrap();
        assert_eq!(count, 1);

        assert!(mgr.get_session(&keep_id).is_some());
        assert!(mgr.get_session(&remove_id).is_none());
        assert!(mgr.validate_token("user1", &keep_token).is_ok());
        assert!(mgr.validate_token("user1", &remove_token).is_err());
    }

    #[test]
    fn test_cleanup_expired() {
        let mgr = SessionManager::new(short_ttl_config());
        let device = test_device("dev1");
        mgr.create_session("user1", device.clone(), None, Duration::from_millis(10))
            .unwrap();
        mgr.create_session("user1", device.clone(), None, Duration::from_millis(10))
            .unwrap();

        std::thread::sleep(Duration::from_millis(30));

        let cleaned = mgr.cleanup_expired();
        assert_eq!(cleaned, 2);
        assert_eq!(mgr.active_count("user1"), 0);
    }

    #[test]
    fn test_device_tracking_across_sessions() {
        let mgr = SessionManager::new(test_config());
        let device1 = DeviceInfo::new("phone-1".to_string(), "iOS/16".to_string())
            .with_os("iOS".to_string())
            .with_name("iPhone 15".to_string());
        let device2 = DeviceInfo::new("laptop-1".to_string(), "Chrome/120".to_string())
            .with_os("macOS".to_string())
            .with_browser("Chrome".to_string())
            .with_name("MacBook Pro".to_string());

        let (id1, _) = mgr
            .create_session("user1", device1, Some("1.2.3.4"), Duration::from_secs(3600))
            .unwrap();
        let (id2, _) = mgr
            .create_session("user1", device2, Some("5.6.7.8"), Duration::from_secs(3600))
            .unwrap();

        let sessions = mgr.get_user_sessions("user1");
        assert_eq!(sessions.len(), 2);

        let s1 = sessions.iter().find(|s| s.id == id1).unwrap();
        let s2 = sessions.iter().find(|s| s.id == id2).unwrap();

        assert_eq!(s1.device_info.device_id, "phone-1");
        assert_eq!(s1.device_info.os.as_deref(), Some("iOS"));
        assert_eq!(s1.ip_address.as_deref(), Some("1.2.3.4"));

        assert_eq!(s2.device_info.device_id, "laptop-1");
        assert_eq!(s2.device_info.browser.as_deref(), Some("Chrome"));
        assert_eq!(s2.device_info.os.as_deref(), Some("macOS"));
        assert_eq!(s2.ip_address.as_deref(), Some("5.6.7.8"));
    }

    #[test]
    fn test_wrong_token_rejected() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let _ = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        let fake_token = SessionToken::new();
        let result = mgr.validate_token("user1", &fake_token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::InvalidToken));
    }

    #[test]
    fn test_configurable_ttl() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");

        let (id1, _) = mgr
            .create_session("user1", device.clone(), None, Duration::from_secs(60))
            .unwrap();
        let (id2, _) = mgr
            .create_session("user2", device, None, Duration::from_secs(7200))
            .unwrap();

        let s1 = mgr.get_session(&id1).unwrap();
        let s2 = mgr.get_session(&id2).unwrap();

        let ttl1 = s1.expires_at - s1.created_at;
        let ttl2 = s2.expires_at - s2.created_at;

        assert!(ttl2.num_seconds() > ttl1.num_seconds());
    }

    #[test]
    fn test_ttl_capped_at_max() {
        let mut config = test_config();
        config.max_ttl = Duration::from_secs(100);
        let mgr = SessionManager::new(config);
        let device = test_device("dev1");

        let (id, _) = mgr
            .create_session("user1", device, None, Duration::from_secs(99999))
            .unwrap();

        let session = mgr.get_session(&id).unwrap();
        let ttl = (session.expires_at - session.created_at).num_seconds();
        assert_eq!(ttl, 100);
    }

    #[test]
    fn test_nonexistent_session_get() {
        let mgr = SessionManager::new(test_config());
        assert!(mgr.get_session("nonexistent").is_none());
    }

    #[test]
    fn test_double_invalidate() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (session_id, _) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        mgr.invalidate_session(&session_id).unwrap();
        let result = mgr.invalidate_session(&session_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::NotFound));
    }

    #[test]
    fn test_validate_nonexistent_user() {
        let mgr = SessionManager::new(test_config());
        let token = SessionToken::new();
        let result = mgr.validate_token("noone", &token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::NotFound));
    }

    #[test]
    fn test_invalidate_all_empty_user() {
        let mgr = SessionManager::new(test_config());
        let count = mgr.invalidate_all_user_sessions("nobody").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_invalidate_all_except_empty_user() {
        let mgr = SessionManager::new(test_config());
        let count = mgr.invalidate_all_except("nobody", "some-id").unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_active_count_no_sessions() {
        let mgr = SessionManager::new(test_config());
        assert_eq!(mgr.active_count("nobody"), 0);
    }

    #[test]
    fn test_rotate_nonexistent_session() {
        let mgr = SessionManager::new(test_config());
        let result = mgr.rotate_token("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::NotFound));
    }

    #[test]
    fn test_rotate_expired_session() {
        let mgr = SessionManager::new(short_ttl_config());
        let device = test_device("dev1");
        let (session_id, _) = mgr
            .create_session("user1", device, None, Duration::from_millis(10))
            .unwrap();

        std::thread::sleep(Duration::from_millis(30));

        let result = mgr.rotate_token(&session_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::Expired));
    }

    #[test]
    fn test_multiple_users_independent() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");

        let (_, t1) = mgr
            .create_session("alice", device.clone(), None, Duration::from_secs(3600))
            .unwrap();
        let (_, t2) = mgr
            .create_session("bob", device, None, Duration::from_secs(3600))
            .unwrap();

        assert!(mgr.validate_token("alice", &t1).is_ok());
        assert!(mgr.validate_token("bob", &t2).is_ok());
        assert!(mgr.validate_token("alice", &t2).is_err());
        assert!(mgr.validate_token("bob", &t1).is_err());
    }

    #[test]
    fn test_enforce_limit_direct_call() {
        let mut config = test_config();
        config.max_concurrent_sessions = 1;
        let mgr = SessionManager::new(config);
        let device = test_device("dev1");

        mgr.create_session("user1", device.clone(), None, Duration::from_secs(3600))
            .unwrap();
        mgr.enforce_limit("user1").unwrap();
    }

    #[test]
    fn test_session_is_elevated_false_by_default() {
        let mgr = SessionManager::new(test_config());
        let device = test_device("dev1");
        let (session_id, _token) = mgr
            .create_session("user1", device, None, Duration::from_secs(3600))
            .unwrap();

        let session = mgr.get_session(&session_id).unwrap();
        assert!(!session.is_elevated);
    }

    #[test]
    fn test_token_zeroize_debug_redaction() {
        let token = SessionToken::new();
        assert_eq!(format!("{:?}", token), "SessionToken([redacted])");
        let _hex = token.to_hex();
    }

    #[test]
    fn test_hash_token_consistency() {
        let token = SessionToken::new();
        let h1 = hash_token(&token);
        let h2 = hash_token(&token);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
