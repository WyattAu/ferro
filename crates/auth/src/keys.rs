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
    #[must_use]
    pub fn key_id_from_public(public_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        hex::encode(hasher.finalize())[..16].to_string()
    }
}

/// E2EE configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct E2eeConfig {
    pub enabled: bool,
    pub admin_recovery_key_id: Option<String>,
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

    #[test]
    fn test_e2ee_key_meta_serialization_roundtrip() {
        let meta = E2eeKeyMeta {
            public_key: "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE".to_string(),
            key_id: "key-123".to_string(),
            created_at: 1700000000,
            algorithm: "ES256".to_string(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let deser: E2eeKeyMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.public_key, meta.public_key);
        assert_eq!(deser.key_id, meta.key_id);
        assert_eq!(deser.created_at, meta.created_at);
        assert_eq!(deser.algorithm, meta.algorithm);
    }

    #[test]
    fn test_e2ee_config_serialization_roundtrip() {
        let config = E2eeConfig {
            enabled: true,
            admin_recovery_key_id: Some("recovery-key-1".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deser: E2eeConfig = serde_json::from_str(&json).unwrap();
        assert!(deser.enabled);
        assert_eq!(deser.admin_recovery_key_id.as_deref(), Some("recovery-key-1"));
    }

    #[test]
    fn test_e2ee_key_meta_debug_format() {
        let meta = E2eeKeyMeta {
            public_key: "key".to_string(),
            key_id: "id".to_string(),
            created_at: 0,
            algorithm: "ES256".to_string(),
        };
        let debug = format!("{:?}", meta);
        assert!(debug.contains("E2eeKeyMeta"));
    }

    #[test]
    fn test_e2ee_config_debug_format() {
        let config = E2eeConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("E2eeConfig"));
    }

    #[test]
    fn test_key_id_from_empty_string() {
        let id = E2eeKeyMeta::key_id_from_public("");
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_key_id_from_long_key() {
        let long_key = "x".repeat(10000);
        let id = E2eeKeyMeta::key_id_from_public(&long_key);
        assert_eq!(id.len(), 16);
    }
}
