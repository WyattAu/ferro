//! WebAuthn/FIDO2 passwordless authentication framework.
//!
//! Protocol and cryptographic verification are delegated to the published
//! `webauthn-kit` crate (CTAP2 authenticator data parsing, COSE ES256/RS256
//! signature verification via `ring`, challenge freshness/replay protection).
//! This module keeps what the kit deliberately leaves to the integrator:
//! credential storage ([`WebAuthnStore`]) and the re-exported kit API.
//!
//! Supports:
//! - ES256 (ECDSA P-256 + SHA-256) — the most common `WebAuthn` algorithm
//! - RS256 (RSA PKCS#1 v1.5 + SHA-256) — for broader authenticator compatibility
//!
//! ## Security
//!
//! Verification (performed by `webauthn-kit`) covers full cryptographic
//! checking of `WebAuthn` ceremonies:
//! - COSE public key parsing and signature verification via `ring`
//! - Authenticator data parsing (rpIdHash, flags, signCount, credential data)
//! - Client data JSON hash verification
//! - Origin and RP-ID validation
//! - Challenge freshness and replay protection

mod credential;

pub use credential::WebAuthnStore;
pub use webauthn_kit::{
    AllowCredential, AuthenticationOptions, AuthenticationParams, AuthenticationResponse, AuthenticationResult,
    AuthenticatorSelection, ExcludeCredential, PubKeyCredParam, RegistrationOptions, RegistrationResponse,
    RegistrationResult, RelyingParty, WebauthnConfig as WebAuthnConfig, WebauthnCredential as WebAuthnCredential,
    WebauthnError as WebAuthnError, WebauthnUser as WebAuthnUser, verify_authentication, verify_registration,
};

#[cfg(test)]
mod tests {
    use super::*;
    use credential::WebAuthnStore;

    fn test_config() -> WebAuthnConfig {
        WebAuthnConfig {
            rp_id: "localhost".to_string(),
            rp_name: "Ferro Test".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
            allowed_algorithms: vec![-7, -257],
            challenge_timeout_secs: 300,
        }
    }

    #[test]
    fn test_webauthn_config_default() {
        let config = WebAuthnConfig::default();
        assert_eq!(config.rp_id, "localhost");
        assert_eq!(config.rp_origins, vec!["http://localhost:8080"]);
        assert_eq!(config.allowed_algorithms, vec![-7, -257]);
        assert_eq!(config.challenge_timeout_secs, 300);
    }

    #[test]
    fn test_store_register_and_get_credentials() {
        let mut store = WebAuthnStore::new();
        let cred = WebAuthnCredential {
            credential_id: "cred-1".to_string(),
            public_key_cose: vec![1, 2, 3, 4],
            sign_count: 0,
            device_name: "YubiKey 5".to_string(),
            registered_at: 1700000000,
            last_used_at: 1700000000,
            attestation_format: "packed".to_string(),
            user_verified: true,
        };
        store.register_credential("alice", cred.clone());

        let creds = store.get_credentials("alice");
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, "cred-1");

        let empty = store.get_credentials("bob");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_challenge_registration_flow() {
        let mut store = WebAuthnStore::new();
        let config = test_config();

        let (challenge_id, _options) = store.generate_registration_challenge(&config, "alice", "Alice", &[]);

        store.store_registration_challenge(&challenge_id, "alice", challenge_id.as_bytes().into());

        let (username, bytes) = store.consume_registration_challenge(&challenge_id, 300).unwrap();
        assert_eq!(username, "alice");
        assert_eq!(bytes, challenge_id.as_bytes());

        assert!(store.consume_registration_challenge(&challenge_id, 300).is_err());
    }

    #[test]
    fn test_challenge_expiration() {
        // Expiry semantics live in webauthn-kit's ChallengeStore; exercise it
        // directly via its public `_at` constructor.
        let mut store = webauthn_kit::ChallengeStore::new();
        store.store_registration_challenge_at("ch-1", "alice", vec![0u8; 32], chrono::Utc::now().timestamp() - 301);

        let result = store.consume_registration_challenge("ch-1", 300);
        assert!(matches!(result, Err(WebAuthnError::ChallengeExpired)));
    }

    #[test]
    fn test_challenge_expiry_with_injected_clock() {
        let t0 = 1_700_000_000i64;
        let mut store = webauthn_kit::ChallengeStore::with_clock(std::sync::Arc::new(move || t0 + 301));
        store.store_registration_challenge_at("ch", "alice", vec![0u8; 32], t0);

        let result = store.consume_registration_challenge("ch", 300);
        assert!(matches!(result, Err(WebAuthnError::ChallengeExpired)));

        // Just inside the freshness window succeeds.
        store.store_registration_challenge_at("ch-ok", "alice", vec![0u8; 32], t0 + 1);
        assert!(store.consume_registration_challenge("ch-ok", 300).is_ok());
    }

    #[test]
    fn test_find_credential_across_users() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "shared-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        let (username, _cred) = store.find_credential("shared-1").unwrap();
        assert_eq!(username, "alice");

        assert!(store.find_credential("nonexistent").is_none());
    }

    #[test]
    fn test_update_credential_usage() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        store.update_credential_usage("alice", "cred-1", 42).unwrap();
        let creds = store.get_credentials("alice");
        assert_eq!(creds[0].sign_count, 42);
        assert!(creds[0].last_used_at > 0);
    }

    #[test]
    fn test_remove_credential() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        assert!(store.remove_credential("alice", "cred-1"));
        assert!(store.get_credentials("alice").is_empty());
        assert!(!store.remove_credential("alice", "cred-1"));
    }

    #[test]
    fn test_duplicate_detection() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        assert!(store.is_credential_registered("cred-1"));
        assert!(!store.is_credential_registered("cred-2"));
    }

    #[test]
    fn test_authentication_challenge_flow() {
        let mut store = WebAuthnStore::new();
        let config = test_config();

        let (challenge_id, _options) = store.generate_authentication_challenge(&config, vec!["cred-1".to_string()]);

        store.store_authentication_challenge(&challenge_id, "alice", vec![0u8; 32], vec!["cred-1".to_string()]);

        let (username, _bytes, allowed) = store.consume_authentication_challenge(&challenge_id, 300).unwrap();
        assert_eq!(username, "alice");
        assert_eq!(allowed, vec!["cred-1".to_string()]);
    }

    #[test]
    fn test_registration_options_serialization() {
        let store = WebAuthnStore::new();
        let config = test_config();
        let (_, options) =
            store.generate_registration_challenge(&config, "alice", "Alice Johnson", &["existing-1".to_string()]);

        let json = serde_json::to_string(&options).unwrap();
        let deser: RegistrationOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.rp.id, "localhost");
        assert_eq!(deser.user.display_name, "alice");
        assert_eq!(deser.exclude_credentials.len(), 1);
        assert_eq!(deser.pub_key_cred_params.len(), 2);
    }

    #[test]
    fn test_authentication_options_serialization() {
        let store = WebAuthnStore::new();
        let config = test_config();
        let (_, options) = store.generate_authentication_challenge(&config, vec!["cred-1".to_string()]);

        let json = serde_json::to_string(&options).unwrap();
        let deser: AuthenticationOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.rp_id, "localhost");
        assert_eq!(deser.allow_credentials.len(), 1);
    }

    #[test]
    fn test_webauthn_error_display_variants() {
        let errors = vec![
            WebAuthnError::Config("test".to_string()),
            WebAuthnError::InvalidChallenge("test".to_string()),
            WebAuthnError::CredentialNotFound("test".to_string()),
            WebAuthnError::VerificationFailed("test".to_string()),
            WebAuthnError::DuplicateCredential("test".to_string()),
            WebAuthnError::UserNotFound("test".to_string()),
            WebAuthnError::ChallengeExpired,
            WebAuthnError::UnsupportedAlgorithm(-7),
            WebAuthnError::SignatureVerificationFailed,
            WebAuthnError::AttestationError("test".to_string()),
        ];
        for err in errors {
            let display = format!("{}", err);
            assert!(!display.is_empty());
            let debug = format!("{:?}", err);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_webauthn_store_default() {
        let store = WebAuthnStore::default();
        assert!(store.get_credentials("nobody").is_empty());
    }

    #[test]
    fn test_consume_authentication_challenge_not_found() {
        let mut store = WebAuthnStore::new();
        let result = store.consume_authentication_challenge("nonexistent", 300);
        assert!(matches!(result, Err(WebAuthnError::InvalidChallenge(_))));
    }

    #[test]
    fn test_consume_authentication_challenge_expired() {
        let mut store = webauthn_kit::ChallengeStore::new();
        store.store_authentication_challenge_at(
            "ch-1",
            "alice",
            vec![0u8; 32],
            vec!["cred-1".to_string()],
            chrono::Utc::now().timestamp() - 400,
        );
        let result = store.consume_authentication_challenge("ch-1", 300);
        assert!(matches!(result, Err(WebAuthnError::ChallengeExpired)));
    }

    #[test]
    fn test_update_credential_usage_missing_user() {
        let mut store = WebAuthnStore::new();
        let result = store.update_credential_usage("nobody", "cred-1", 1);
        assert!(matches!(result, Err(WebAuthnError::CredentialNotFound(_))));
    }

    #[test]
    fn test_update_credential_usage_missing_credential() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );
        let result = store.update_credential_usage("alice", "cred-2", 1);
        assert!(matches!(result, Err(WebAuthnError::CredentialNotFound(_))));
    }

    #[test]
    fn test_remove_credential_wrong_user() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );
        assert!(!store.remove_credential("bob", "cred-1"));
    }

    #[test]
    fn test_multiple_credentials_per_user() {
        let mut store = WebAuthnStore::new();
        for i in 0..3 {
            store.register_credential(
                "alice",
                WebAuthnCredential {
                    credential_id: format!("cred-{}", i),
                    public_key_cose: vec![],
                    sign_count: 0,
                    device_name: format!("Device {}", i),
                    registered_at: 0,
                    last_used_at: 0,
                    attestation_format: "none".to_string(),
                    user_verified: false,
                },
            );
        }
        assert_eq!(store.get_credentials("alice").len(), 3);
        assert!(store.is_credential_registered("cred-0"));
        assert!(store.is_credential_registered("cred-1"));
        assert!(store.is_credential_registered("cred-2"));
    }

    #[test]
    fn test_find_credential_across_multiple_users() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "alice-cred".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );
        store.register_credential(
            "bob",
            WebAuthnCredential {
                credential_id: "bob-cred".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        let (user, _) = store.find_credential("alice-cred").unwrap();
        assert_eq!(user, "alice");
        let (user, _) = store.find_credential("bob-cred").unwrap();
        assert_eq!(user, "bob");
        assert!(store.find_credential("nobody-cred").is_none());
    }

    #[test]
    fn test_registration_options_configurable() {
        let store = WebAuthnStore::new();
        let config = WebAuthnConfig {
            rp_id: "custom.example.com".to_string(),
            rp_name: "Custom App".to_string(),
            rp_origins: vec!["https://custom.example.com".to_string()],
            allowed_algorithms: vec![-7, -257],
            challenge_timeout_secs: 600,
        };
        let (_, options) =
            store.generate_registration_challenge(&config, "alice", "Alice", &["existing-cred".to_string()]);
        assert_eq!(options.rp.id, "custom.example.com");
        assert_eq!(options.rp.name, "Custom App");
        assert_eq!(options.timeout, 600_000);
        assert_eq!(options.exclude_credentials.len(), 1);
    }

    #[test]
    fn test_authentication_options_configurable() {
        let store = WebAuthnStore::new();
        let config = WebAuthnConfig {
            rp_id: "custom.example.com".to_string(),
            rp_name: "Custom App".to_string(),
            rp_origins: vec![],
            allowed_algorithms: vec![],
            challenge_timeout_secs: 120,
        };
        let (_, options) = store.generate_authentication_challenge(&config, vec!["c1".to_string(), "c2".to_string()]);
        assert_eq!(options.rp_id, "custom.example.com");
        assert_eq!(options.timeout, 120_000);
        assert_eq!(options.allow_credentials.len(), 2);
    }
}
