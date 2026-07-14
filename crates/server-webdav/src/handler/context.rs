use super::*;
use crate::WebdavAppState;
use axum::http::HeaderMap;
use common::error::{FerroError, Result};

pub struct WebdavHandlerContext<'a, S: WebdavAppState> {
    pub(crate) state: &'a S,
    pub path: String,
    pub(crate) headers: &'a HeaderMap,
    pub(crate) owner: String,
}

impl<'a, S: WebdavAppState> WebdavHandlerContext<'a, S> {
    pub(crate) fn new(state: &'a S, path: String, headers: &'a HeaderMap) -> Self {
        let owner = extract_owner(headers, None);
        Self {
            state,
            path,
            headers,
            owner,
        }
    }

    pub(crate) fn validate_path(&self) -> Result<()> {
        if !common::path::validate_path(&self.path) {
            return Err(FerroError::InvalidArgument(format!("Invalid path: {}", self.path)));
        }
        Ok(())
    }

    pub(crate) async fn check_lock(&self) -> Result<()> {
        if let Some(lock) = self.state.lock_manager().check_lock(&self.path).await {
            return Err(FerroError::LockConflict(format!(
                "Resource locked by {}",
                lock.principal
            )));
        }
        Ok(())
    }

    pub(crate) async fn check_lock_for_write(&self, target: &str) -> Result<()> {
        if let Err(e) = self.state.lock_manager().check_lock_for_write(target).await {
            return Err(FerroError::LockConflict(format!("Source locked: {}", e)));
        }
        Ok(())
    }

    pub(crate) fn check_worm(&self) -> Result<()> {
        if self.state.is_worm_protected(&self.path) {
            return Err(FerroError::WormProtected(self.path.clone()));
        }
        Ok(())
    }

    pub(crate) fn check_if_match(&self, etag: &str) -> Result<()> {
        check_conditional_if_match(self.headers, etag)
    }

    pub(crate) fn check_if_none_match(&self, etag: &str) -> bool {
        check_if_none_match(self.headers, etag)
    }

    pub(crate) async fn dispatch_event(&self, event: crate::WebdavFileEvent) {
        self.state.dispatch_post_op(event).await;
    }

    pub(crate) fn record_sync(
        &self,
        op_type: crate::WebdavOpType,
        new_path: Option<&str>,
        size: u64,
        mime_type: Option<&str>,
        content_hash: &str,
    ) {
        self.state.record_sync_op(
            op_type,
            &self.path,
            new_path,
            size,
            mime_type,
            &self.owner,
            content_hash,
        );
    }

    pub(crate) async fn fire_triggers(&self, event_type: crate::WebdavEventType) {
        self.state
            .fire_event_triggers(event_type, &self.path, &self.owner)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_validate_path_valid() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/foo/bar".to_string(), &headers);
        assert!(ctx.validate_path().is_ok());
    }

    #[test]
    fn test_validate_path_root() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/".to_string(), &headers);
        assert!(ctx.validate_path().is_ok());
    }

    #[test]
    fn test_validate_path_traversal() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/foo/../bar".to_string(), &headers);
        assert!(ctx.validate_path().is_err());
    }

    #[test]
    fn test_check_worm_not_protected() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_worm().is_ok());
    }

    #[test]
    fn test_check_worm_protected() {
        let state = crate::test_helpers::MockWebdavState::new();
        state.set_worm_protected("/test.txt", true);
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_worm().is_err());
    }

    #[test]
    fn test_check_if_match_no_header() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_if_match("test-etag").is_ok());
    }

    #[test]
    fn test_check_if_match_match() {
        let state = crate::test_helpers::MockWebdavState::new();
        let mut headers = HeaderMap::new();
        headers.insert("If-Match", "test-etag".parse().unwrap());
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_if_match("test-etag").is_ok());
    }

    #[test]
    fn test_check_if_match_no_match() {
        let state = crate::test_helpers::MockWebdavState::new();
        let mut headers = HeaderMap::new();
        headers.insert("If-Match", "wrong-etag".parse().unwrap());
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_if_match("test-etag").is_err());
    }

    #[test]
    fn test_check_if_none_match_no_header() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(!ctx.check_if_none_match("test-etag"));
    }

    #[test]
    fn test_check_if_none_match_match() {
        let state = crate::test_helpers::MockWebdavState::new();
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "test-etag".parse().unwrap());
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_if_none_match("test-etag"));
    }

    #[test]
    fn test_check_if_none_match_no_match() {
        let state = crate::test_helpers::MockWebdavState::new();
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "wrong-etag".parse().unwrap());
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(!ctx.check_if_none_match("test-etag"));
    }

    #[tokio::test]
    async fn test_check_lock_no_lock() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_lock().await.is_ok());
    }

    #[tokio::test]
    async fn test_check_lock_with_lock() {
        let state = crate::test_helpers::MockWebdavState::new();
        state
            .storage()
            .put("/test.txt", bytes::Bytes::from("test"), "user1")
            .await
            .unwrap();
        state
            .lock_manager()
            .acquire_lock(
                "/test.txt",
                "user1",
                common::webdav::LockScope::Exclusive,
                common::webdav::LockDepth::Zero,
                None,
            )
            .await
            .unwrap();

        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_lock().await.is_err());
    }

    #[tokio::test]
    async fn test_check_lock_for_write_no_lock() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        assert!(ctx.check_lock_for_write("/test.txt").await.is_ok());
    }

    #[test]
    fn test_record_sync() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        // This should not panic
        ctx.record_sync(crate::WebdavOpType::Update, None, 100, Some("text/plain"), "hash");
    }

    #[tokio::test]
    async fn test_dispatch_event() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        let event = crate::WebdavFileEvent {
            op_type: "put",
            path: "/test.txt".to_string(),
            new_path: None,
            size: Some(100),
            mime_type: Some("text/plain".to_string()),
            owner: "user1".to_string(),
            etag: Some("etag".to_string()),
            already_existed: false,
        };
        // This should not panic
        ctx.dispatch_event(event).await;
    }

    #[tokio::test]
    async fn test_fire_triggers() {
        let state = crate::test_helpers::MockWebdavState::new();
        let headers = HeaderMap::new();
        let ctx = WebdavHandlerContext::new(&state, "/test.txt".to_string(), &headers);
        // This should not panic
        ctx.fire_triggers(crate::WebdavEventType::FileUploaded).await;
    }
}
