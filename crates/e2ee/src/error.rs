use thiserror::Error;

#[derive(Debug, Error)]
pub enum E2eeError {
    #[error("Key generation failed: {message}")]
    KeyGeneration { message: String },
    #[error("Encryption failed: {message}")]
    Encryption { message: String },
    #[error("Decryption failed: {message}")]
    Decryption { message: String },
    #[error("Invalid key: {reason}")]
    InvalidKey { reason: String },
    #[error("Serialization failed: {message}")]
    Serialization { message: String },
}

impl From<aes_gcm::Error> for E2eeError {
    fn from(e: aes_gcm::Error) -> Self {
        E2eeError::Encryption {
            message: e.to_string(),
        }
    }
}

impl From<std::io::Error> for E2eeError {
    fn from(e: std::io::Error) -> Self {
        E2eeError::Encryption {
            message: e.to_string(),
        }
    }
}
