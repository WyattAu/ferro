//! WARNING: WebAuthn stub implementation. Does NOT perform cryptographic verification. NOT suitable for production use.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A registered WebAuthn credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredential {
    pub credential_id: String,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
    pub device_name: String,
    pub registered_at: i64,
    pub last_used_at: i64,
}

/// WebAuthn configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnConfig {
    pub enabled: bool,
    pub rp_id: String,
    pub rp_name: String,
    pub rp_origin: String,
}

impl Default for WebAuthnConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rp_id: "localhost".to_string(),
            rp_name: "Ferro".to_string(),
            rp_origin: "http://localhost:8080".to_string(),
        }
    }
}

/// In-memory credential store. WARNING: credentials are lost on restart. NOT suitable for production.
pub struct WebAuthnStore {
    credentials: HashMap<String, Vec<WebAuthnCredential>>,
    challenges: HashMap<String, String>,
}

impl WebAuthnStore {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            challenges: HashMap::new(),
        }
    }

    pub fn register_credential(&mut self, username: &str, cred: WebAuthnCredential) {
        self.credentials
            .entry(username.to_string())
            .or_default()
            .push(cred);
    }

    pub fn get_credentials(&self, username: &str) -> Vec<WebAuthnCredential> {
        self.credentials.get(username).cloned().unwrap_or_default()
    }

    pub fn store_challenge(&mut self, challenge_id: &str, challenge: &str) {
        self.challenges
            .insert(challenge_id.to_string(), challenge.to_string());
    }

    pub fn get_and_remove_challenge(&mut self, challenge_id: &str) -> Option<String> {
        self.challenges.remove(challenge_id)
    }
}

impl Default for WebAuthnStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webauthn_config_default() {
        let config = WebAuthnConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rp_id, "localhost");
        assert_eq!(config.rp_name, "Ferro");
        assert_eq!(config.rp_origin, "http://localhost:8080");
    }

    #[test]
    fn test_store_register_and_get_credentials() {
        let mut store = WebAuthnStore::new();
        let cred = WebAuthnCredential {
            credential_id: "cred-1".to_string(),
            public_key: vec![1, 2, 3, 4],
            sign_count: 0,
            device_name: "YubiKey 5".to_string(),
            registered_at: 1700000000,
            last_used_at: 1700000000,
        };
        store.register_credential("alice", cred.clone());
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-2".to_string(),
                public_key: vec![5, 6, 7, 8],
                sign_count: 0,
                device_name: "Touch ID".to_string(),
                registered_at: 1700000001,
                last_used_at: 1700000001,
            },
        );

        let creds = store.get_credentials("alice");
        assert_eq!(creds.len(), 2);
        assert_eq!(creds[0].credential_id, "cred-1");
        assert_eq!(creds[1].credential_id, "cred-2");

        let empty = store.get_credentials("bob");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_store_challenge_flow() {
        let mut store = WebAuthnStore::new();
        store.store_challenge("ch-1", "random-challenge-bytes");
        assert_eq!(
            store.get_and_remove_challenge("ch-1"),
            Some("random-challenge-bytes".to_string())
        );
        assert!(store.get_and_remove_challenge("ch-1").is_none());
    }
}
