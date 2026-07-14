use age::secrecy::SecretString;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::io::{Read, Write};

use crate::SecurityAppState;
use crate::error::ApiError;
use crate::security::validate_path;

pub async fn encrypt_content(content: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    let passphrase = SecretString::from(passphrase.to_string());
    let encryptor = age::Encryptor::with_user_passphrase(passphrase);

    let encrypted = tokio::task::spawn_blocking({
        let content = content.to_vec();
        move || {
            let mut binary = Vec::new();
            let mut writer = encryptor
                .wrap_output(&mut binary)
                .map_err(|e| format!("wrap error: {e}"))?;
            writer.write_all(&content).map_err(|e| format!("write error: {e}"))?;
            writer.finish().map_err(|e| format!("finish error: {e}"))?;

            let mut armored = Vec::new();
            let mut armor_writer = age::armor::ArmoredWriter::wrap_output(&mut armored, age::armor::Format::AsciiArmor)
                .map_err(|e| format!("armor writer error: {e}"))?;
            armor_writer
                .write_all(&binary)
                .map_err(|e| format!("armor write error: {e}"))?;
            armor_writer.finish().map_err(|e| format!("armor finish error: {e}"))?;

            Ok::<Vec<u8>, String>(armored)
        }
    })
    .await
    .map_err(|e| format!("task error: {e}"))??;

    Ok(encrypted)
}

pub async fn decrypt_content(encrypted: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    let passphrase = SecretString::from(passphrase.to_string());

    let decrypted = tokio::task::spawn_blocking({
        let encrypted = encrypted.to_vec();
        move || {
            let armor_reader = age::armor::ArmoredReader::new(&encrypted[..]);
            let decryptor = age::Decryptor::new(armor_reader).map_err(|e| format!("decryptor error: {e}"))?;
            let identity = age::scrypt::Identity::new(passphrase);
            let dyn_identity: &dyn age::Identity = &identity;
            let mut reader = decryptor
                .decrypt(std::iter::once(dyn_identity))
                .map_err(|e| format!("decrypt error: {e}"))?;
            let mut output = Vec::new();
            reader
                .read_to_end(&mut output)
                .map_err(|e| format!("read error: {e}"))?;
            Ok::<Vec<u8>, String>(output)
        }
    })
    .await
    .map_err(|e| format!("task error: {e}"))??;

    Ok(decrypted)
}

#[must_use]
pub fn is_age_encrypted(content: &[u8]) -> bool {
    if let Ok(s) = std::str::from_utf8(content) {
        s.starts_with("-----BEGIN AGE ENCRYPTED FILE-----")
    } else {
        false
    }
}

#[derive(Deserialize)]
pub struct EncryptRequest {
    pub path: String,
    pub passphrase: String,
}

pub async fn encrypt_file<S: SecurityAppState>(
    State(state): State<S>,
    axum::Json(req): axum::Json<EncryptRequest>,
) -> Response {
    if let Err(e) = validate_path(&req.path) {
        return ApiError::bad_request(ApiError::PATH_INVALID, format!("Invalid path: {e}"));
    }

    let content = match state.storage().get(&req.path).await {
        Ok(c) => c,
        Err(_) => {
            return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found");
        }
    };

    match encrypt_content(&content, &req.passphrase).await {
        Ok(encrypted) => match state.storage().put(&req.path, encrypted.into(), "admin").await {
            Ok(meta) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "path": meta.path,
                    "size": meta.size,
                    "encrypted": true,
                })),
            )
                .into_response(),
            Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, format!("Failed to write encrypted file: {e}")),
        },
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, format!("Encryption failed: {e}")),
    }
}

#[derive(Deserialize)]
pub struct DecryptRequest {
    pub path: String,
    pub passphrase: String,
}

pub async fn decrypt_file<S: SecurityAppState>(
    State(state): State<S>,
    axum::Json(req): axum::Json<DecryptRequest>,
) -> Response {
    if let Err(e) = validate_path(&req.path) {
        return ApiError::bad_request(ApiError::PATH_INVALID, format!("Invalid path: {e}"));
    }

    let content = match state.storage().get(&req.path).await {
        Ok(c) => c,
        Err(_) => {
            return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found");
        }
    };

    if !is_age_encrypted(&content) {
        return ApiError::bad_request(ApiError::NOT_ENCRYPTED, "File is not encrypted");
    }

    match decrypt_content(&content, &req.passphrase).await {
        Ok(decrypted) => match state.storage().put(&req.path, decrypted.into(), "admin").await {
            Ok(meta) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "path": meta.path,
                    "size": meta.size,
                    "encrypted": false,
                })),
            )
                .into_response(),
            Err(e) => ApiError::internal(ApiError::DECRYPT_FAILED, format!("Failed to write decrypted file: {e}")),
        },
        Err(e) => ApiError::bad_request(
            ApiError::DECRYPT_FAILED,
            format!("Decryption failed: wrong passphrase? {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let original = b"Hello, this is a secret message!";
        let passphrase = "test-password-123";

        let encrypted = encrypt_content(original, passphrase).await.unwrap();
        assert!(is_age_encrypted(&encrypted));
        assert_ne!(encrypted, original.to_vec());

        let decrypted = decrypt_content(&encrypted, passphrase).await.unwrap();
        assert_eq!(decrypted, original);
    }

    #[tokio::test]
    async fn test_wrong_passphrase() {
        let original = b"secret data";
        let encrypted = encrypt_content(original, "correct-pass").await.unwrap();
        let result = decrypt_content(&encrypted, "wrong-pass").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_is_age_encrypted() {
        let armored = b"-----BEGIN AGE ENCRYPTED FILE-----\ndata\n-----END AGE ENCRYPTED FILE-----";
        assert!(is_age_encrypted(armored));
        assert!(!is_age_encrypted(b"not encrypted"));
        assert!(!is_age_encrypted(b""));
    }

    #[test]
    fn test_validate_path_rejects_traversal() {
        assert!(validate_path("/../../../etc/passwd").is_err());
        assert!(validate_path("/good/path.txt").is_ok());
    }

    #[tokio::test]
    async fn test_large_file() {
        let data = vec![0u8; 1024 * 1024];
        let encrypted = encrypt_content(&data, "pass").await.unwrap();
        let decrypted = decrypt_content(&encrypted, "pass").await.unwrap();
        assert_eq!(decrypted.len(), data.len());
    }
}
