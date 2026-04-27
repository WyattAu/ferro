use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// SHA-256 content hash stored as 64 hex characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Create a content hash from a pre-computed 64-char hex string. Panics if length is not 64.
    pub fn new(hex: String) -> Self {
        assert!(hex.len() == 64, "ContentHash must be 64 hex characters");
        Self(hex)
    }

    /// Compute the SHA-256 hash of the given byte slice.
    pub fn compute(data: &[u8]) -> Self {
        let hash = Sha256::digest(data);
        Self(hex::encode(hash))
    }

    /// Compute the SHA-256 hash by streaming from a reader.
    pub fn compute_reader<R: std::io::Read>(mut reader: R) -> std::io::Result<Self> {
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        Ok(Self(hex::encode(hasher.finalize())))
    }

    /// Return the hash as a hex string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Alias for [`Self::as_str`].
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Parse a content hash from an ETag string, stripping surrounding quotes.
    pub fn from_etag(etag: &str) -> Self {
        let clean = etag.trim_matches('"');
        if clean.len() == 64 {
            Self(clean.to_string())
        } else {
            let hash = Sha256::digest(clean.as_bytes());
            Self(hex::encode(hash))
        }
    }
}

/// Metadata for a file or collection (directory) in the virtual filesystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub content_hash: ContentHash,
    pub size: u64,
    pub mime_type: String,
    pub is_collection: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub owner: String,
    pub etag: String,
}

impl FileMetadata {
    /// Create metadata for a regular file with sensible defaults.
    pub fn new(path: String, content_hash: ContentHash, size: u64, owner: String) -> Self {
        let now = Utc::now();
        Self {
            path,
            etag: format!("\"{}\"", content_hash.as_str()),
            content_hash,
            size,
            mime_type: "application/octet-stream".to_string(),
            is_collection: false,
            created_at: now,
            modified_at: now,
            owner,
        }
    }

    /// Create metadata for a collection (directory).
    pub fn new_collection(path: String, owner: String) -> Self {
        let now = Utc::now();
        Self {
            path,
            etag: format!("\"col-{}\"", now.timestamp_millis()),
            content_hash: ContentHash::new("0".repeat(64)),
            size: 0,
            mime_type: "httpd/unix-directory".to_string(),
            is_collection: true,
            created_at: now,
            modified_at: now,
            owner,
        }
    }
}
