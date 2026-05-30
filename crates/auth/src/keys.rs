//! End-to-end encryption key management.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A user's E2EE key pair metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2eeKeyMeta {
    pub public_key: String,
    pub key_id: String,
    pub created_at: i64,
    pub algorithm: String,
}

impl E2eeKeyMeta {
    /// Generate a key ID from the public key.
    pub fn key_id_from_public(public_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        hex::encode(hasher.finalize())[..16].to_string()
    }
}

/// E2EE configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2eeConfig {
    pub enabled: bool,
    pub admin_recovery_key_id: Option<String>,
}

impl Default for E2eeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            admin_recovery_key_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_id_from_public_deterministic() {
        let id1 = E2eeKeyMeta::key_id_from_public("test-public-key");
        let id2 = E2eeKeyMeta::key_id_from_public("test-public-key");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 16);
    }

    #[test]
    fn test_key_id_from_public_different_keys() {
        let id1 = E2eeKeyMeta::key_id_from_public("key-a");
        let id2 = E2eeKeyMeta::key_id_from_public("key-b");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_e2ee_config_default() {
        let config = E2eeConfig::default();
        assert!(!config.enabled);
        assert!(config.admin_recovery_key_id.is_none());
    }
}
