//! Credential storage for `WebAuthn` — the persistence half that
//! `webauthn-kit` deliberately leaves to the integrator. Challenge
//! generation, single-use consumption (replay protection), and expiry are
//! delegated to [`webauthn_kit::ChallengeStore`].

use std::collections::HashMap;

use webauthn_kit::{
    AuthenticationOptions, ChallengeStore, RegistrationOptions, WebauthnConfig, WebauthnCredential, WebauthnError,
};

/// In-memory credential store with challenge management delegated to
/// [`webauthn_kit::ChallengeStore`].
pub struct WebAuthnStore {
    /// Per-user credential lists.
    credentials: HashMap<String, Vec<WebauthnCredential>>,
    /// Pending challenge management (registration + authentication).
    challenges: ChallengeStore,
}

impl WebAuthnStore {
    /// Create a new empty credential store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            challenges: ChallengeStore::new(),
        }
    }

    /// Store a pending registration challenge.
    pub fn store_registration_challenge(&mut self, challenge_id: &str, username: &str, challenge_bytes: Vec<u8>) {
        self.challenges
            .store_registration_challenge(challenge_id, username, challenge_bytes);
    }

    /// Consume a registration challenge, returning the associated username and bytes.
    pub fn consume_registration_challenge(
        &mut self,
        challenge_id: &str,
        timeout_secs: u64,
    ) -> Result<(String, Vec<u8>), WebauthnError> {
        self.challenges
            .consume_registration_challenge(challenge_id, timeout_secs)
    }

    /// Store a pending authentication challenge.
    pub fn store_authentication_challenge(
        &mut self,
        challenge_id: &str,
        username: &str,
        challenge_bytes: Vec<u8>,
        allowed_credential_ids: Vec<String>,
    ) {
        self.challenges
            .store_authentication_challenge(challenge_id, username, challenge_bytes, allowed_credential_ids);
    }

    /// Consume an authentication challenge.
    pub fn consume_authentication_challenge(
        &mut self,
        challenge_id: &str,
        timeout_secs: u64,
    ) -> Result<(String, Vec<u8>, Vec<String>), WebauthnError> {
        self.challenges
            .consume_authentication_challenge(challenge_id, timeout_secs)
    }

    /// Register a credential for a user.
    pub fn register_credential(&mut self, username: &str, cred: WebauthnCredential) {
        self.credentials.entry(username.to_string()).or_default().push(cred);
    }

    /// Look up credentials by credential ID across all users.
    #[must_use]
    pub fn find_credential(&self, credential_id: &str) -> Option<(&str, &WebauthnCredential)> {
        for (username, creds) in &self.credentials {
            for cred in creds {
                if cred.credential_id == credential_id {
                    return Some((username, cred));
                }
            }
        }
        None
    }

    /// Get all credentials for a user.
    #[must_use]
    pub fn get_credentials(&self, username: &str) -> Vec<WebauthnCredential> {
        self.credentials.get(username).cloned().unwrap_or_default()
    }

    /// Update the sign count and `last_used_at` for a credential.
    pub fn update_credential_usage(
        &mut self,
        username: &str,
        credential_id: &str,
        new_sign_count: u32,
    ) -> Result<(), WebauthnError> {
        let creds = self
            .credentials
            .get_mut(username)
            .ok_or_else(|| WebauthnError::CredentialNotFound(format!("user '{username}' has no credentials")))?;

        let cred = creds
            .iter_mut()
            .find(|c| c.credential_id == credential_id)
            .ok_or_else(|| WebauthnError::CredentialNotFound(credential_id.to_string()))?;

        cred.sign_count = new_sign_count;
        cred.last_used_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Remove a credential.
    pub fn remove_credential(&mut self, username: &str, credential_id: &str) -> bool {
        if let Some(creds) = self.credentials.get_mut(username) {
            let before = creds.len();
            creds.retain(|c| c.credential_id != credential_id);
            creds.len() < before
        } else {
            false
        }
    }

    /// Check if a credential ID is already registered (for duplicate detection).
    #[must_use]
    pub fn is_credential_registered(&self, credential_id: &str) -> bool {
        self.find_credential(credential_id).is_some()
    }

    /// Generate a new registration challenge (delegated to
    /// [`webauthn_kit::ChallengeStore`]).
    #[must_use]
    pub fn generate_registration_challenge(
        &self,
        config: &WebauthnConfig,
        username: &str,
        display_name: &str,
        existing_credential_ids: &[String],
    ) -> (String, RegistrationOptions) {
        self.challenges
            .generate_registration_challenge(config, username, display_name, existing_credential_ids)
    }

    /// Generate a new authentication challenge (delegated to
    /// [`webauthn_kit::ChallengeStore`]).
    #[must_use]
    pub fn generate_authentication_challenge(
        &self,
        config: &WebauthnConfig,
        credential_ids: Vec<String>,
    ) -> (String, AuthenticationOptions) {
        self.challenges
            .generate_authentication_challenge(config, credential_ids)
    }
}

impl Default for WebAuthnStore {
    fn default() -> Self {
        Self::new()
    }
}
