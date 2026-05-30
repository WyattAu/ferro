//! Smart deduplication using perceptual hashing for near-duplicate detection.

use sha2::{Digest, Sha256};

/// Compute a content-aware hash for deduplication.
/// For images: uses average hash (aHash) for perceptual similarity.
/// For all files: falls back to SHA-256 content hash.
pub fn compute_dedup_hash(content: &[u8], content_type: &str) -> String {
    if content_type.starts_with("image/")
        && content.len() < 10 * 1024 * 1024
        && let Some(hash) = compute_average_hash(content)
    {
        return format!("ahash:{}", hash);
    }
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Compute 8-bit average hash for image deduplication.
fn compute_average_hash(_data: &[u8]) -> Option<u64> {
    // Placeholder: average hash requires image decoding.
    // When image crate is integrated, decode to 8x8 grayscale and compute bit hash.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_hash_fallback_sha256() {
        let content = b"hello world";
        let hash = compute_dedup_hash(content, "application/pdf");
        assert!(!hash.starts_with("ahash:"));
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_dedup_hash_image_fallback_when_large() {
        let content = vec![0u8; 11 * 1024 * 1024];
        let hash = compute_dedup_hash(&content, "image/png");
        assert!(!hash.starts_with("ahash:"));
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_dedup_hash_deterministic() {
        let content = b"test content";
        let h1 = compute_dedup_hash(content, "text/plain");
        let h2 = compute_dedup_hash(content, "text/plain");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_dedup_hash_different_content() {
        let h1 = compute_dedup_hash(b"content a", "application/octet-stream");
        let h2 = compute_dedup_hash(b"content b", "application/octet-stream");
        assert_ne!(h1, h2);
    }
}
