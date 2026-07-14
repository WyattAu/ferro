pub use ferro_server_collaboration::comments::Comment;
pub use ferro_server_collaboration::comments::CommentStore;
pub use ferro_server_collaboration::comments::CreateCommentRequest;
pub use ferro_server_collaboration::comments::ListCommentsQuery;
pub use ferro_server_collaboration::comments::UpdateCommentRequest;
pub use ferro_server_collaboration::comments::create_comment_handler;
pub use ferro_server_collaboration::comments::delete_comment_handler;
pub use ferro_server_collaboration::comments::list_comments_handler;
pub use ferro_server_collaboration::comments::resolve_comment_handler;
pub use ferro_server_collaboration::comments::update_comment_handler;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn setup_store() -> (CommentStore, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let conn = db::open_db(dir.path().to_str().unwrap()).unwrap();
        let handle = Arc::new(std::sync::Mutex::new(conn));
        let store = CommentStore::new().with_db(handle);
        (store, dir)
    }

    #[test]
    fn test_add_and_list_comments() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Great doc!", None).unwrap();
        assert_eq!(c.path, "/doc.pdf");
        assert_eq!(c.user_id, "user-1");
        assert_eq!(c.body, "Great doc!");
        assert!(!c.resolved);
        assert!(c.parent_id.is_none());

        let comments = store.list_comments("/doc.pdf").unwrap();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn test_nested_comments() {
        let (store, _dir) = setup_store();
        let parent = store.add_comment("/doc.pdf", "user-1", "Parent comment", None).unwrap();
        let child = store
            .add_comment("/doc.pdf", "user-2", "Reply", Some(&parent.id))
            .unwrap();
        assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));

        let comments = store.list_comments("/doc.pdf").unwrap();
        assert_eq!(comments.len(), 2);
    }

    #[test]
    fn test_update_comment() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Original", None).unwrap();
        let updated = store.update_comment(&c.id, "user-1", "Updated body").unwrap();
        assert_eq!(updated.body, "Updated body");
        assert_ne!(updated.updated_at, c.updated_at);
    }

    #[test]
    fn test_update_comment_permission_denied() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Original", None).unwrap();
        let result = store.update_comment(&c.id, "user-2", "Hacked!");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission denied"));
    }

    #[test]
    fn test_delete_own_comment() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Delete me", None).unwrap();
        assert!(store.delete_comment(&c.id, "user-1", false).is_ok());
        let comments = store.list_comments("/doc.pdf").unwrap();
        assert!(comments.is_empty());
    }

    #[test]
    fn test_delete_comment_by_admin() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Delete me", None).unwrap();
        assert!(store.delete_comment(&c.id, "admin-user", true).is_ok());
        let comments = store.list_comments("/doc.pdf").unwrap();
        assert!(comments.is_empty());
    }

    #[test]
    fn test_delete_comment_permission_denied() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Can't delete", None).unwrap();
        let result = store.delete_comment(&c.id, "user-2", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Permission denied"));
    }

    #[test]
    fn test_resolve_comment() {
        let (store, _dir) = setup_store();
        let c = store.add_comment("/doc.pdf", "user-1", "Resolve me", None).unwrap();
        assert!(!c.resolved);
        let resolved = store.resolve_comment(&c.id, "user-1").unwrap();
        assert!(resolved.resolved);
    }

    #[test]
    fn test_empty_body_rejected() {
        let (store, _dir) = setup_store();
        let result = store.add_comment("/doc.pdf", "user-1", "", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_comment_not_found() {
        let (store, _dir) = setup_store();
        let result = store.update_comment("nonexistent", "user-1", "body");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_comments_isolated_by_path() {
        let (store, _dir) = setup_store();
        store.add_comment("/a.txt", "user-1", "Comment on A", None).unwrap();
        store.add_comment("/b.txt", "user-1", "Comment on B", None).unwrap();
        assert_eq!(store.list_comments("/a.txt").unwrap().len(), 1);
        assert_eq!(store.list_comments("/b.txt").unwrap().len(), 1);
    }
}
