use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, AeadCore, Nonce};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::E2eeError;
use crate::key::E2eeKeyPair;

const FILE_KEY_INFO: &[u8] = b"ferro-e2ee";
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
}

#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    pub algorithm: EncryptionAlgorithm,
    pub chunk_size: usize,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            chunk_size: 64 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedChunk {
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct EncryptedFile {
    pub version: u8,
    pub algorithm: EncryptionAlgorithm,
    pub nonce: [u8; NONCE_LEN],
    pub key_id: [u8; 32],
    pub chunks: Vec<EncryptedChunk>,
    pub original_size: u64,
}

fn derive_file_key(key: &E2eeKeyPair) -> Result<[u8; 32], E2eeError> {
    let ikm = key.private_key_bytes();
    let salt = key.key_id();

    let hk = Hkdf::<Sha256>::new(Some(&salt), ikm);

    let mut file_key = [0u8; 32];
    hk.expand(FILE_KEY_INFO, &mut file_key)
        .map_err(|e| E2eeError::Encryption {
            message: e.to_string(),
        })?;
    Ok(file_key)
}

fn encrypt_chunk(cipher: &Aes256Gcm, plaintext: &[u8]) -> Result<EncryptedChunk, E2eeError> {
    let nonce = Aes256Gcm::generate_nonce(rand::rngs::OsRng);
    let ciphertext = cipher.encrypt(&nonce, plaintext)?;

    let mut nonce_arr = [0u8; NONCE_LEN];
    nonce_arr.copy_from_slice(&nonce);

    Ok(EncryptedChunk {
        nonce: nonce_arr,
        ciphertext,
    })
}

fn decrypt_chunk(cipher: &Aes256Gcm, chunk: &EncryptedChunk) -> Result<Vec<u8>, E2eeError> {
    let nonce = Nonce::from_slice(&chunk.nonce);
    cipher
        .decrypt(nonce, chunk.ciphertext.as_ref())
        .map_err(|e| E2eeError::Decryption {
            message: e.to_string(),
        })
}

pub fn encrypt_file(
    key: &E2eeKeyPair,
    data: &[u8],
    config: &EncryptionConfig,
) -> Result<EncryptedFile, E2eeError> {
    let file_key = derive_file_key(key)?;
    let cipher = Aes256Gcm::new_from_slice(&file_key).map_err(|e| E2eeError::Encryption {
        message: e.to_string(),
    })?;

    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce);

    let mut chunks = Vec::new();
    for chunk_data in data.chunks(config.chunk_size) {
        chunks.push(encrypt_chunk(&cipher, chunk_data)?);
    }

    if chunks.is_empty() {
        chunks.push(encrypt_chunk(&cipher, &[])?);
    }

    Ok(EncryptedFile {
        version: 1,
        algorithm: config.algorithm,
        nonce,
        key_id: key.key_id(),
        chunks,
        original_size: data.len() as u64,
    })
}

pub fn decrypt_file(key: &E2eeKeyPair, encrypted: &EncryptedFile) -> Result<Vec<u8>, E2eeError> {
    if encrypted.version != 1 {
        return Err(E2eeError::Decryption {
            message: format!("Unsupported version: {}", encrypted.version),
        });
    }

    let file_key = derive_file_key(key)?;
    let cipher = Aes256Gcm::new_from_slice(&file_key).map_err(|e| E2eeError::Decryption {
        message: e.to_string(),
    })?;

    let mut output = Vec::with_capacity(encrypted.original_size as usize);
    for chunk in &encrypted.chunks {
        let plaintext = decrypt_chunk(&cipher, chunk)?;
        output.extend_from_slice(&plaintext);
    }

    Ok(output)
}

pub async fn stream_encrypt<R, W>(
    key: &E2eeKeyPair,
    mut reader: R,
    mut writer: W,
    config: &EncryptionConfig,
) -> Result<u64, E2eeError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let file_key = derive_file_key(key)?;
    let cipher = Aes256Gcm::new_from_slice(&file_key).map_err(|e| E2eeError::Encryption {
        message: e.to_string(),
    })?;

    let version: u8 = 1;
    let algorithm_byte: u8 = match config.algorithm {
        EncryptionAlgorithm::Aes256Gcm => 0,
    };

    writer.write_all(&[version]).await?;
    writer.write_all(&[algorithm_byte]).await?;

    let nonce = Aes256Gcm::generate_nonce(rand::rngs::OsRng);
    writer.write_all(&nonce).await?;

    let key_id = key.key_id();
    writer.write_all(&key_id).await?;

    let chunk_size = config.chunk_size;
    let mut buf = vec![0u8; chunk_size];
    let mut total_written = 0u64;

    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        let chunk = encrypt_chunk(&cipher, &buf[..n])?;

        let nonce_len = chunk.nonce.len() as u32;
        writer.write_all(&nonce_len.to_le_bytes()).await?;
        writer.write_all(&chunk.nonce).await?;

        let ct_len = chunk.ciphertext.len() as u32;
        writer.write_all(&ct_len.to_le_bytes()).await?;
        writer.write_all(&chunk.ciphertext).await?;

        total_written += n as u64;
    }

    let original_size = total_written.to_le_bytes();
    writer.write_all(&original_size).await?;

    writer.flush().await?;

    Ok(total_written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key() -> E2eeKeyPair {
        E2eeKeyPair::generate().unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = make_key();
        let data = b"hello, end-to-end encryption!";
        let config = EncryptionConfig::default();

        let encrypted = encrypt_file(&key, data, &config).unwrap();
        let decrypted = decrypt_file(&key, &encrypted).unwrap();
        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_empty_file() {
        let key = make_key();
        let data = b"";
        let config = EncryptionConfig::default();

        let encrypted = encrypt_file(&key, data, &config).unwrap();
        let decrypted = decrypt_file(&key, &encrypted).unwrap();
        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_single_byte() {
        let key = make_key();
        let data = b"X";
        let config = EncryptionConfig::default();

        let encrypted = encrypt_file(&key, data, &config).unwrap();
        let decrypted = decrypt_file(&key, &encrypted).unwrap();
        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_large_file_multi_chunk() {
        let key = make_key();
        let config = EncryptionConfig {
            chunk_size: 64,
            ..Default::default()
        };
        let data = vec![0xABu8; 256];

        let encrypted = encrypt_file(&key, &data, &config).unwrap();
        assert!(encrypted.chunks.len() > 1, "Should have multiple chunks");
        let decrypted = decrypt_file(&key, &encrypted).unwrap();
        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_wrong_key_returns_error() {
        let key = make_key();
        let wrong_key = make_key();
        let data = b"secret data";
        let config = EncryptionConfig::default();

        let encrypted = encrypt_file(&key, data, &config).unwrap();
        let result = decrypt_file(&wrong_key, &encrypted);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_encrypt_basic() {
        let key = make_key();
        let data = b"streaming encryption test data here";
        let config = EncryptionConfig::default();

        let mut reader: &[u8] = data;
        let mut writer = Vec::new();

        let bytes_written = stream_encrypt(&key, &mut reader, &mut writer, &config).await.unwrap();
        assert_eq!(bytes_written, data.len() as u64);
        assert!(!writer.is_empty());
    }
}
