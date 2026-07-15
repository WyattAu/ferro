use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// SHA-256 content hash stored as 64 hex characters.
#[repr(align(64))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    /// Create a content hash from a pre-computed 64-char hex string.
    /// Returns None if the input is not exactly 64 hex characters.
    #[must_use]
    pub fn new(hex: String) -> Option<Self> {
        if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            Some(Self(hex))
        } else {
            None
        }
    }

    /// Create a content hash from a pre-computed 64-char hex string without validation.
    /// Only use for internally-generated hashes that are guaranteed valid.
    #[must_use]
    pub fn new_unchecked(hex: String) -> Self {
        debug_assert!(hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()));
        Self(hex)
    }

    /// Compute the SHA-256 hash of the given byte slice.
    #[inline]
    #[must_use]
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
        Ok(Self::new_unchecked(hex::encode(hasher.finalize())))
    }

    /// Return the hash as a hex string slice.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Alias for [`Self::as_str`].
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Parse a content hash from an `ETag` string, stripping surrounding quotes.
    #[must_use]
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
#[repr(align(64))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Virtual filesystem path of the file or collection.
    pub path: String,
    /// SHA-256 content hash.
    pub content_hash: ContentHash,
    /// Size in bytes.
    pub size: u64,
    /// MIME type of the file.
    pub mime_type: String,
    /// Whether this entry is a collection (directory).
    pub is_collection: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub modified_at: DateTime<Utc>,
    /// Owner of the file or collection.
    pub owner: String,
    /// `ETag` string for conditional requests.
    pub etag: String,
}

impl FileMetadata {
    /// Create metadata for a regular file with sensible defaults.
    #[inline]
    #[must_use]
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
    #[inline]
    #[must_use]
    pub fn new_collection(path: String, owner: String) -> Self {
        let now = Utc::now();
        Self {
            path,
            etag: format!("\"col-{}\"", now.timestamp_millis()),
            content_hash: ContentHash::new_unchecked("0".repeat(64)),
            size: 0,
            mime_type: "httpd/unix-directory".to_string(),
            is_collection: true,
            created_at: now,
            modified_at: now,
            owner,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_new_valid() {
        let hash = ContentHash::new("a".repeat(64));
        assert!(hash.is_some());
        assert_eq!(hash.unwrap().as_str(), "a".repeat(64));
    }

    #[test]
    fn test_content_hash_new_invalid_length() {
        assert!(ContentHash::new("abc".into()).is_none());
        assert!(ContentHash::new("a".repeat(63)).is_none());
        assert!(ContentHash::new("a".repeat(65)).is_none());
    }

    #[test]
    fn test_content_hash_new_invalid_chars() {
        assert!(ContentHash::new("g".repeat(64)).is_none());
        assert!(ContentHash::new("Z".repeat(64)).is_none());
        assert!(ContentHash::new(" ".repeat(64)).is_none());
    }

    #[test]
    fn test_content_hash_new_empty() {
        assert!(ContentHash::new(String::new()).is_none());
    }

    #[test]
    fn test_content_hash_new_unchecked() {
        let hash = ContentHash::new_unchecked("a".repeat(64));
        assert_eq!(hash.as_str(), "a".repeat(64));
        assert_eq!(hash.as_hex(), "a".repeat(64));
    }

    #[test]
    fn test_content_hash_compute() {
        let hash = ContentHash::compute(b"hello");
        assert_eq!(hash.as_str().len(), 64);
        assert!(hash.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_content_hash_compute_empty() {
        let hash = ContentHash::compute(b"");
        assert_eq!(hash.as_str().len(), 64);
        assert_eq!(
            hash.as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_content_hash_compute_deterministic() {
        let a = ContentHash::compute(b"test");
        let b = ContentHash::compute(b"test");
        assert_eq!(a, b);
    }

    #[test]
    fn test_content_hash_compute_reader() {
        let data = b"hello world";
        let hash = ContentHash::compute_reader(&data[..]).unwrap();
        assert_eq!(hash.as_str().len(), 64);
        assert_eq!(hash, ContentHash::compute(data));
    }

    #[test]
    fn test_content_hash_from_etag_with_quotes() {
        let hash = ContentHash::from_etag("\"abc\"");
        assert_eq!(hash.as_str().len(), 64);
    }

    #[test]
    fn test_content_hash_from_etag_without_quotes() {
        let hash = ContentHash::from_etag("a".repeat(64).as_str());
        assert_eq!(hash.as_str(), "a".repeat(64));
    }

    #[test]
    fn test_file_metadata_new() {
        let hash = ContentHash::compute(b"data");
        let meta = FileMetadata::new("/test.txt".into(), hash, 42, "alice".into());
        assert_eq!(meta.path, "/test.txt");
        assert_eq!(meta.size, 42);
        assert!(!meta.is_collection);
        assert_eq!(meta.owner, "alice");
        assert!(meta.etag.starts_with('"'));
        assert!(meta.etag.ends_with('"'));
    }

    #[test]
    fn test_file_metadata_new_collection() {
        let meta = FileMetadata::new_collection("/docs".into(), "alice".into());
        assert_eq!(meta.path, "/docs");
        assert!(meta.is_collection);
        assert_eq!(meta.size, 0);
        assert_eq!(meta.mime_type, "httpd/unix-directory");
    }

    #[test]
    fn test_content_hash_compute_different_inputs() {
        let hash1 = ContentHash::compute(b"data1");
        let hash2 = ContentHash::compute(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_compute_large_data() {
        let data = vec![0u8; 1024 * 1024]; // 1MB
        let hash = ContentHash::compute(&data);
        assert_eq!(hash.as_str().len(), 64);
    }

    #[test]
    fn test_content_hash_compute_reader_empty() {
        let data = b"";
        let hash = ContentHash::compute_reader(&data[..]).unwrap();
        assert_eq!(hash.as_str().len(), 64);
    }

    #[test]
    fn test_content_hash_eq() {
        let hash1 = ContentHash::compute(b"test");
        let hash2 = ContentHash::compute(b"test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_ne() {
        let hash1 = ContentHash::compute(b"test1");
        let hash2 = ContentHash::compute(b"test2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_clone() {
        let hash1 = ContentHash::compute(b"test");
        let hash2 = hash1.clone();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_content_hash_debug() {
        let hash = ContentHash::compute(b"test");
        let debug = format!("{:?}", hash);
        assert!(debug.contains("ContentHash"));
    }

    #[test]
    fn test_content_hash_serialize_deserialize() {
        let hash = ContentHash::compute(b"test");
        let json = serde_json::to_string(&hash).unwrap();
        let deserialized: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, deserialized);
    }

    #[test]
    fn test_file_metadata_debug() {
        let hash = ContentHash::compute(b"data");
        let meta = FileMetadata::new("/test.txt".into(), hash, 42, "alice".into());
        let debug = format!("{:?}", meta);
        assert!(debug.contains("FileMetadata"));
    }

    #[test]
    fn test_file_metadata_clone() {
        let hash = ContentHash::compute(b"data");
        let meta1 = FileMetadata::new("/test.txt".into(), hash, 42, "alice".into());
        let meta2 = meta1.clone();
        assert_eq!(meta1.path, meta2.path);
        assert_eq!(meta1.size, meta2.size);
    }

    #[test]
    fn test_file_metadata_serialize_deserialize() {
        let hash = ContentHash::compute(b"data");
        let meta = FileMetadata::new("/test.txt".into(), hash, 42, "alice".into());
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: FileMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.path, deserialized.path);
        assert_eq!(meta.size, deserialized.size);
    }

    #[test]
    fn test_file_metadata_collection_etag() {
        let meta = FileMetadata::new_collection("/docs".into(), "alice".into());
        assert!(meta.etag.starts_with("\"col-"));
    }

    #[test]
    fn test_file_metadata_mime_type() {
        let hash = ContentHash::compute(b"data");
        let meta = FileMetadata::new("/test.txt".into(), hash, 42, "alice".into());
        assert_eq!(meta.mime_type, "application/octet-stream");
    }
}
