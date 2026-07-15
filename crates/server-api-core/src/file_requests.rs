use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A file request is an upload-only share link with additional metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    pub id: String,
    pub path: String,
    pub message: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_uploads: Option<u32>,
    pub upload_count: u32,
    pub created_by: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFileRequest {
    pub path: String,
    pub message: Option<String>,
    pub expires_in_hours: Option<i64>,
    pub max_uploads: Option<u32>,
}

#[async_trait]
pub trait FileRequestStoreTrait: Send + Sync {
    async fn create(&self, req: CreateFileRequest, created_by: String) -> FileRequest;
    async fn get(&self, id: &str) -> Option<FileRequest>;
    async fn get_by_token(&self, token: &str) -> Option<FileRequest>;
    async fn delete(&self, id: &str) -> bool;
    async fn list(&self) -> Vec<FileRequest>;
    async fn increment_upload(&self, id: &str) -> bool;
}

pub struct FileRequestStore {
    requests: Arc<RwLock<Vec<FileRequest>>>,
}

impl FileRequestStore {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn load_request(&self, request: FileRequest) {
        self.requests.write().await.push(request);
    }

    pub fn load_requests_blocking(&self, requests: Vec<FileRequest>) {
        tokio::task::block_in_place(|| {
            let mut guard = self.requests.blocking_write();
            for req in requests {
                guard.push(req);
            }
        });
    }
}

impl Default for FileRequestStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileRequestStoreTrait for FileRequestStore {
    async fn create(&self, req: CreateFileRequest, created_by: String) -> FileRequest {
        let id = uuid::Uuid::new_v4().to_string();
        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = req.expires_in_hours.map(|h| Utc::now() + chrono::Duration::hours(h));

        let file_request = FileRequest {
            id: id.clone(),
            path: req.path,
            message: req.message,
            expires_at,
            max_uploads: req.max_uploads,
            upload_count: 0,
            created_by,
            token: token.clone(),
        };

        self.requests.write().await.push(file_request.clone());
        file_request
    }

    async fn get(&self, id: &str) -> Option<FileRequest> {
        let requests = self.requests.read().await;
        requests.iter().find(|r| r.id == id).cloned()
    }

    async fn get_by_token(&self, token: &str) -> Option<FileRequest> {
        let requests = self.requests.read().await;
        requests.iter().find(|r| r.token == token).cloned()
    }

    async fn delete(&self, id: &str) -> bool {
        let mut requests = self.requests.write().await;
        if let Some(pos) = requests.iter().position(|r| r.id == id) {
            requests.remove(pos);
            true
        } else {
            false
        }
    }

    async fn list(&self) -> Vec<FileRequest> {
        let requests = self.requests.read().await;
        requests
            .iter()
            .filter(|r| r.expires_at.map(|e| e > Utc::now()).unwrap_or(true))
            .cloned()
            .collect()
    }

    async fn increment_upload(&self, id: &str) -> bool {
        let mut requests = self.requests.write().await;
        if let Some(req) = requests.iter_mut().find(|r| r.id == id) {
            req.upload_count += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_file_request() {
        let store = FileRequestStore::new();
        let req = CreateFileRequest {
            path: "/uploads".to_string(),
            message: Some("Please upload your resume".to_string()),
            expires_in_hours: Some(48),
            max_uploads: Some(10),
        };
        let fr = store.create(req, "admin".to_string()).await;
        assert_eq!(fr.path, "/uploads");
        assert_eq!(fr.message, Some("Please upload your resume".to_string()));
        assert_eq!(fr.max_uploads, Some(10));
        assert_eq!(fr.upload_count, 0);
        assert_eq!(fr.created_by, "admin");
        assert!(!fr.id.is_empty());
        assert!(!fr.token.is_empty());
    }

    #[tokio::test]
    async fn test_get_file_request() {
        let store = FileRequestStore::new();
        let req = CreateFileRequest {
            path: "/test".to_string(),
            message: None,
            expires_in_hours: None,
            max_uploads: None,
        };
        let fr = store.create(req, "user".to_string()).await;
        let found = store.get(&fr.id).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, fr.id);
    }

    #[tokio::test]
    async fn test_get_by_token() {
        let store = FileRequestStore::new();
        let req = CreateFileRequest {
            path: "/test".to_string(),
            message: None,
            expires_in_hours: None,
            max_uploads: None,
        };
        let fr = store.create(req, "user".to_string()).await;
        let found = store.get_by_token(&fr.token).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().token, fr.token);
    }

    #[tokio::test]
    async fn test_delete_file_request() {
        let store = FileRequestStore::new();
        let req = CreateFileRequest {
            path: "/test".to_string(),
            message: None,
            expires_in_hours: None,
            max_uploads: None,
        };
        let fr = store.create(req, "user".to_string()).await;
        assert!(store.delete(&fr.id).await);
        assert!(store.get(&fr.id).await.is_none());
    }

    #[tokio::test]
    async fn test_list_filters_expired() {
        let store = FileRequestStore::new();
        store
            .create(
                CreateFileRequest {
                    path: "/active".to_string(),
                    message: None,
                    expires_in_hours: Some(24),
                    max_uploads: None,
                },
                "user".to_string(),
            )
            .await;
        store
            .create(
                CreateFileRequest {
                    path: "/expired".to_string(),
                    message: None,
                    expires_in_hours: Some(-1),
                    max_uploads: None,
                },
                "user".to_string(),
            )
            .await;
        let list = store.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/active");
    }

    #[tokio::test]
    async fn test_increment_upload() {
        let store = FileRequestStore::new();
        let req = CreateFileRequest {
            path: "/test".to_string(),
            message: None,
            expires_in_hours: None,
            max_uploads: Some(5),
        };
        let fr = store.create(req, "user".to_string()).await;
        assert!(store.increment_upload(&fr.id).await);
        assert!(store.increment_upload(&fr.id).await);
        let found = store.get(&fr.id).await.unwrap();
        assert_eq!(found.upload_count, 2);
    }
}
