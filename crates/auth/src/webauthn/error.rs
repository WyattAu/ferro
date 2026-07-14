/// Error type for `WebAuthn` operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum WebAuthnError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Invalid challenge: {0}")]
    InvalidChallenge(String),
    #[error("Credential not found: {0}")]
    CredentialNotFound(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Duplicate credential: {0}")]
    DuplicateCredential(String),
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("Challenge expired")]
    ChallengeExpired,
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(i32),
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Attestation error: {0}")]
    AttestationError(String),
}
