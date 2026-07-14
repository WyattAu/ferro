pub use ferro_core::storage::InMemoryStorageEngine;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use common::storage::StorageEngine;

    #[tokio::test]
    async fn test_put_get_delete() {
        let engine = InMemoryStorageEngine::new();

        engine.put("/hello.txt", Bytes::from("hello"), "user1").await.unwrap();

        let meta = engine.head("/hello.txt").await.unwrap();
        assert_eq!(meta.size, 5);

        let content = engine.get("/hello.txt").await.unwrap();
        assert_eq!(&content[..], b"hello");

        engine.delete("/hello.txt").await.unwrap();
        assert!(!engine.exists("/hello.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_list() {
        let engine = InMemoryStorageEngine::new();

        engine.create_collection("/docs", "user1").await.unwrap();
        engine.put("/docs/a.txt", Bytes::from("a"), "user1").await.unwrap();
        engine.put("/docs/b.txt", Bytes::from("b"), "user1").await.unwrap();

        let items = engine.list("/docs").await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_copy() {
        let engine = InMemoryStorageEngine::new();

        engine.put("/original.txt", Bytes::from("data"), "user1").await.unwrap();
        engine.copy("/original.txt", "/copy.txt").await.unwrap();

        let content = engine.get("/copy.txt").await.unwrap();
        assert_eq!(&content[..], b"data");

        assert!(engine.exists("/original.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_move_path() {
        let engine = InMemoryStorageEngine::new();

        engine.put("/source.txt", Bytes::from("data"), "user1").await.unwrap();
        engine.move_path("/source.txt", "/dest.txt").await.unwrap();

        assert!(!engine.exists("/source.txt").await.unwrap());
        let content = engine.get("/dest.txt").await.unwrap();
        assert_eq!(&content[..], b"data");
    }

    #[tokio::test]
    async fn test_not_found() {
        let engine = InMemoryStorageEngine::new();
        assert!(engine.head("/missing").await.is_err());
        assert!(engine.get("/missing").await.is_err());
        assert!(engine.delete("/missing").await.is_err());
    }

    #[tokio::test]
    async fn test_list_all_with_depth_limit() {
        let engine = InMemoryStorageEngine::new();
        engine.create_collection("/root", "user1").await.unwrap();
        engine.create_collection("/root/sub", "user1").await.unwrap();
        engine.create_collection("/root/sub/deep", "user1").await.unwrap();
        engine.put("/root/f1.txt", Bytes::from("a"), "user1").await.unwrap();
        engine.put("/root/sub/f2.txt", Bytes::from("b"), "user1").await.unwrap();
        engine
            .put("/root/sub/deep/f3.txt", Bytes::from("c"), "user1")
            .await
            .unwrap();

        // depth=1 should get root/* (sub, f1.txt) — 2 items
        let items = engine.list_all("/root", 1).await.unwrap();
        assert_eq!(items.len(), 2);

        // depth=2 should get root/* and root/*/* (sub, f1.txt, deep, f2.txt) — 4 items
        let items = engine.list_all("/root", 2).await.unwrap();
        assert_eq!(items.len(), 4);

        // depth=100 should get everything except /root itself (5 items)
        let items = engine.list_all("/root", 100).await.unwrap();
        assert_eq!(items.len(), 5);
    }
}
