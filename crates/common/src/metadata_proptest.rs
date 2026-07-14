use proptest::prelude::*;

use crate::metadata::ContentHash;

proptest! {
    #[test]
    fn content_hash_compute_deterministic(data in ".*") {
        let hash1 = ContentHash::compute(data.as_bytes());
        let hash2 = ContentHash::compute(data.as_bytes());
        prop_assert_eq!(hash1.as_str(), hash2.as_str());
    }

    #[test]
    fn content_hash_is_64_hex_chars(data in ".*") {
        let hash = ContentHash::compute(data.as_bytes());
        prop_assert_eq!(hash.as_str().len(), 64);
        prop_assert!(hash.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn content_hash_new_valid_hex(hex in "[0-9a-fA-F]{64}") {
        let hash = ContentHash::new(hex.clone());
        prop_assert!(hash.is_some());
        let h = hash.unwrap();
        prop_assert_eq!(h.as_str(), hex.as_str());
    }

    #[test]
    fn content_hash_new_rejects_non_hex(hex in ".{64}") {
        // Only reject if it actually contains non-hex chars
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            let hash = ContentHash::new(hex);
            prop_assert!(hash.is_none());
        }
    }

    #[test]
    fn content_hash_new_rejects_wrong_length(len in 0usize..128) {
        if len != 64 {
            let hex = "a".repeat(len);
            let hash = ContentHash::new(hex);
            prop_assert!(hash.is_none());
        }
    }

    #[test]
    fn content_hash_from_etag_never_panics(s in ".*") {
        let _hash = ContentHash::from_etag(&s);
    }

    #[test]
    fn content_hash_as_str_matches_as_hex(data in ".*") {
        let hash = ContentHash::compute(data.as_bytes());
        prop_assert_eq!(hash.as_str(), hash.as_hex());
    }
}
