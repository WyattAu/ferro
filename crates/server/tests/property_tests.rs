//! Property-based tests using proptest for storage, path normalization, and lock state machines.

use common::path::{normalize_path, validate_path};
use common::storage::StorageEngine;
use common::webdav::{LockDepth, LockScope};
use ferro_core::storage::InMemoryStorageEngine;
use ferro_server::lock::LockManager;
use proptest::prelude::*;
use std::collections::HashSet;

// ── AW-003: Storage engine property tests ──────────────────────────────

proptest! {
    /// PUT then GET must return identical bytes (round-trip).
    #[test]
    fn prop_storage_put_get_roundtrip(
        path in "[a-z]{1,8}",
        content in proptest::collection::vec(any::<u8>(), 0..1024),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryStorageEngine::new();
            let full_path = format!("/{}", path);
            let bytes = bytes::Bytes::from(content.clone());

            store.put(&full_path, bytes.clone(), "test").await.unwrap();
            let got = store.get(&full_path).await.unwrap();
            assert_eq!(got, bytes);
        });
    }

    /// PUT then HEAD must return matching size and path.
    #[test]
    fn prop_storage_put_head_consistency(
        path in "[a-z]{1,8}",
        content in proptest::collection::vec(any::<u8>(), 0..1024),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryStorageEngine::new();
            let full_path = format!("/{}", path);
            let bytes = bytes::Bytes::from(content.clone());

            let meta = store.put(&full_path, bytes, "test").await.unwrap();
            assert_eq!(meta.path, full_path);
            assert_eq!(meta.size as usize, content.len());
        });
    }

    /// DELETE after PUT must make the path not exist.
    #[test]
    fn prop_storage_delete_removes_file(
        path in "[a-z]{1,8}",
        content in proptest::collection::vec(any::<u8>(), 1..256),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryStorageEngine::new();
            let full_path = format!("/{}", path);

            store.put(&full_path, bytes::Bytes::from(content), "test").await.unwrap();
            store.delete(&full_path).await.unwrap();
            assert!(!store.exists(&full_path).await.unwrap());
        });
    }

    /// PUT overwrites: second PUT must replace first content.
    #[test]
    fn prop_storage_put_overwrites(
        path in "[a-z]{1,8}",
        content1 in proptest::collection::vec(any::<u8>(), 0..512),
        content2 in proptest::collection::vec(any::<u8>(), 0..512),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryStorageEngine::new();
            let full_path = format!("/{}", path);

            store.put(&full_path, bytes::Bytes::from(content1), "test").await.unwrap();
            store.put(&full_path, bytes::Bytes::from(content2.clone()), "test").await.unwrap();
            let got = store.get(&full_path).await.unwrap();
            assert_eq!(got, bytes::Bytes::from(content2));
        });
    }

    /// Multiple distinct paths must coexist without interference.
    #[test]
    fn prop_storage_multi_path_isolation(
        paths in proptest::collection::vec("[a-z]{1,8}", 1..10),
        content in proptest::collection::vec(any::<u8>(), 1..64),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = InMemoryStorageEngine::new();
            let unique_paths: HashSet<String> = paths.into_iter().collect();

            for p in &unique_paths {
                let full = format!("/{}", p);
                store.put(&full, bytes::Bytes::from(content.clone()), "test").await.unwrap();
            }

            for p in &unique_paths {
                let full = format!("/{}", p);
                assert!(store.exists(&full).await.unwrap());
                let got = store.get(&full).await.unwrap();
                assert_eq!(got, bytes::Bytes::from(content.clone()));
            }
        });
    }
}

// ── AW-004: Path normalization property tests ─────────────────────────

proptest! {
    /// Normalized path must always start with `/`.
    #[test]
    fn prop_path_normalize_starts_with_slash(input in ".*") {
        let result = normalize_path(&input);
        assert!(result.starts_with('/'), "normalize_path({:?}) = {:?} does not start with /", input, result);
    }

    /// Normalized path must never contain `..` as a path component after normalization.
    #[test]
    fn prop_path_normalize_no_dotdot(
        segments in proptest::collection::vec("[a-z]{1,8}", 0..10),
    ) {
        let path = format!("/{}", segments.join("/"));
        let result = normalize_path(&path);
        let components: Vec<&str> = result.split('/').filter(|s| !s.is_empty()).collect();
        assert!(!components.contains(&".."), "normalize_path({:?}) = {:?} contains .. component", path, result);
    }

    /// validate_path must reject empty strings.
    #[test]
    fn prop_path_validate_rejects_empty(s in "") {
        assert!(!validate_path(&s));
    }

    /// validate_path must accept simple absolute paths.
    #[test]
    fn prop_path_validate_accepts_simple(name in "[a-z0-9_]{1,16}") {
        let path = format!("/{}", name);
        assert!(validate_path(&path));
    }

    /// Normalizing the same path twice must be idempotent.
    #[test]
    fn prop_path_normalize_idempotent(input in "[a-z/]{0,20}") {
        let first = normalize_path(&input);
        let second = normalize_path(&first);
        assert_eq!(first, second);
    }

    /// normalize_path must not produce double slashes.
    #[test]
    fn prop_path_normalize_no_double_slash(input in "[a-z/]{0,20}") {
        let result = normalize_path(&input);
        assert!(!result.contains("//"), "normalize_path({:?}) = {:?} contains //", input, result);
    }
}

// ── AW-005: Lock state machine property tests ─────────────────────────

proptest! {
    /// Acquiring a lock must increase lock count by 1.
    #[test]
    fn prop_lock_acquire_increments_count(
        path in "[a-z]{1,8}",
        principal in "[a-z]{1,8}",
    ) {
        let mgr = LockManager::new();
        let full_path = format!("/{}", path);
        let initial = mgr.lock_count();
        let _ = mgr.acquire_lock_sync(&full_path, &principal, LockScope::Exclusive, LockDepth::Zero, None);
        assert_eq!(mgr.lock_count(), initial + 1);
    }

    /// Releasing a lock must decrease lock count by 1.
    #[test]
    fn prop_lock_release_decrements_count(
        path in "[a-z]{1,8}",
        principal in "[a-z]{1,8}",
    ) {
        let mgr = LockManager::new();
        let full_path = format!("/{}", path);
        let lock = mgr.acquire_lock_sync(&full_path, &principal, LockScope::Exclusive, LockDepth::Zero, None).unwrap();
        mgr.release_lock_sync(&lock.token.as_str()).unwrap();
        assert_eq!(mgr.lock_count(), 0);
    }

    /// Exclusive lock on a path must block second exclusive lock on same path.
    #[test]
    fn prop_lock_exclusive_blocks_exclusive(
        path in "[a-z]{1,8}",
        user1 in "[a-z]{1,4}",
        user2 in "[a-z]{1,4}",
    ) {
        prop_assume!(user1 != user2);
        let mgr = LockManager::new();
        let full_path = format!("/{}", path);
        mgr.acquire_lock_sync(&full_path, &user1, LockScope::Exclusive, LockDepth::Zero, None).unwrap();
        let result = mgr.acquire_lock_sync(&full_path, &user2, LockScope::Exclusive, LockDepth::Zero, None);
        assert!(result.is_err());
    }

    /// Two shared locks on the same path must both succeed.
    #[test]
    fn prop_lock_shared_allows_shared(
        path in "[a-z]{1,8}",
        user1 in "[a-z]{1,4}",
        user2 in "[a-z]{1,4}",
    ) {
        let mgr = LockManager::new();
        let full_path = format!("/{}", path);
        mgr.acquire_lock_sync(&full_path, &user1, LockScope::Shared, LockDepth::Zero, None).unwrap();
        let result = mgr.acquire_lock_sync(&full_path, &user2, LockScope::Shared, LockDepth::Zero, None);
        assert!(result.is_ok());
    }

    /// Releasing a lock twice must fail on the second attempt.
    #[test]
    fn prop_lock_double_release_fails(
        path in "[a-z]{1,8}",
        principal in "[a-z]{1,8}",
    ) {
        let mgr = LockManager::new();
        let full_path = format!("/{}", path);
        let lock = mgr.acquire_lock_sync(&full_path, &principal, LockScope::Exclusive, LockDepth::Zero, None).unwrap();
        let token = lock.token;
        mgr.release_lock_sync(&token.as_str()).unwrap();
        let result = mgr.release_lock_sync(&token.as_str());
        assert!(result.is_err());
    }

    /// Lock count must equal the number of distinct paths locked.
    #[test]
    fn prop_lock_count_equals_distinct_paths(
        paths in proptest::collection::vec("[a-z]{1,4}", 1..8),
    ) {
        let mgr = LockManager::new();
        let unique: HashSet<String> = paths.into_iter().collect();
        for p in &unique {
            let full = format!("/{}", p);
            let _ = mgr.acquire_lock_sync(&full, "user", LockScope::Exclusive, LockDepth::Zero, None);
        }
        assert_eq!(mgr.lock_count(), unique.len());
    }

    /// Refreshing a lock must not change the lock count.
    #[test]
    fn prop_lock_refresh_preserves_count(
        path in "[a-z]{1,8}",
        principal in "[a-z]{1,8}",
    ) {
        let mgr = LockManager::new();
        let full_path = format!("/{}", path);
        let lock = mgr.acquire_lock_sync(&full_path, &principal, LockScope::Exclusive, LockDepth::Zero, None).unwrap();
        let _ = mgr.refresh_lock_sync(&lock.token.as_str(), Some(120));
        assert_eq!(mgr.lock_count(), 1);
    }

    /// Infinity lock on parent must block child writes.
    #[test]
    fn prop_lock_infinity_blocks_child(
        parent in "[a-z]{1,4}",
        child_name in "[a-z]{1,4}",
    ) {
        let mgr = LockManager::new();
        let parent_path = format!("/{}", parent);
        mgr.acquire_lock_sync(&parent_path, "user", LockScope::Exclusive, LockDepth::Infinity, None).unwrap();
        let child_path = format!("{}/{}", parent_path, child_name);
        let result = mgr.check_lock_for_write_sync(&child_path);
        assert!(result.is_err());
    }
}
