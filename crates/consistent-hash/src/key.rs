use sha2::{Digest, Sha256};

/// Compute the SHA-256 hash of a key and return it as a hex-encoded string.
pub fn hash_key(key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute a salted SHA-256 hash for consistent placement.
///
/// The salt is prepended to the key before hashing, ensuring that
/// keys with different salts map to different positions on the ring.
pub fn hash_key_with_salt(key: &[u8], salt: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(key);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Convert a hex-encoded hash string to a u64 position on the ring.
///
/// Uses the first 8 bytes of the hash to produce the ring position.
pub fn hash_to_position(hash: &str) -> u64 {
    let mut buf = [0u8; 8];
    for (i, byte) in buf.iter_mut().enumerate() {
        let start = i * 2;
        let end = start + 2;
        if end <= hash.len() {
            *byte = u8::from_str_radix(&hash[start..end], 16).unwrap_or(0);
        }
    }
    u64::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_key_deterministic() {
        let h1 = hash_key(b"test-key");
        let h2 = hash_key(b"test-key");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_key_different_keys() {
        let h1 = hash_key(b"key-a");
        let h2 = hash_key(b"key-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_key_returns_64_hex_chars() {
        let h = hash_key(b"hello");
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn test_hash_key_with_salt() {
        let h1 = hash_key_with_salt(b"key", b"salt1");
        let h2 = hash_key_with_salt(b"key", b"salt2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_key_with_same_salt() {
        let h1 = hash_key_with_salt(b"key", b"salt");
        let h2 = hash_key_with_salt(b"key", b"salt");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_to_position() {
        let h = hash_to_position("0000000000000000");
        assert_eq!(h, 0);

        let h = hash_to_position("0000000000000001");
        assert_eq!(h, 1);
    }
}
