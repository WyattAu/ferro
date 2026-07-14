use proptest::prelude::*;

use crate::api_keys::{ApiKeyPermission, hash_api_key};

proptest! {
    #[test]
    fn hash_api_key_deterministic(key in "[a-zA-Z0-9_-]{1,256}") {
        let h1 = hash_api_key(&key);
        let h2 = hash_api_key(&key);
        prop_assert_eq!(h1.clone(), h2.clone());
    }

    #[test]
    fn hash_api_key_is_64_hex(key in "[a-zA-Z0-9_-]{1,256}") {
        let hash = hash_api_key(&key);
        prop_assert_eq!(hash.len(), 64);
        prop_assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_api_key_different_inputs_differ(key1 in "[a-zA-Z0-9_-]{1,128}", key2 in "[a-zA-Z0-9_-]{1,128}") {
        prop_assume!(key1 != key2);
        let h1 = hash_api_key(&key1);
        let h2 = hash_api_key(&key2);
        prop_assert_ne!(h1, h2);
    }

    #[test]
    fn permission_allows_read_read(perm in prop_oneof![
        Just(ApiKeyPermission::Read),
        Just(ApiKeyPermission::Write),
        Just(ApiKeyPermission::Admin)
    ]) {
        prop_assert!(perm.allows_action("read"));
    }

    #[test]
    fn permission_allows_list_read(perm in prop_oneof![
        Just(ApiKeyPermission::Read),
        Just(ApiKeyPermission::Write),
        Just(ApiKeyPermission::Admin)
    ]) {
        prop_assert!(perm.allows_action("list"));
    }

    #[test]
    fn read_cannot_write(perm in Just(ApiKeyPermission::Read)) {
        prop_assert!(!perm.allows_action("write"));
    }

    #[test]
    fn read_cannot_delete(perm in Just(ApiKeyPermission::Read)) {
        prop_assert!(!perm.allows_action("delete"));
    }

    #[test]
    fn admin_allows_everything(action in "[a-z]{1,20}") {
        let perm = ApiKeyPermission::Admin;
        prop_assert!(perm.allows_action(&action));
    }

    #[test]
    fn hash_never_empty(key in ".{0,256}") {
        let hash = hash_api_key(&key);
        prop_assert!(!hash.is_empty());
    }
}
