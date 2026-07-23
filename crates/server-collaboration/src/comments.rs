use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::DbHandle;
use crate::{ApiError, CollaborationState};
use crate::{AuditEntry, build_audit_entry};
use crate::{contains_html, sanitize_control_chars};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub path: String,
    pub user_id: String,
    pub parent_id: Option<String>,
    pub body: String,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CommentStore {
    pub db: Option<DbHandle>,
}

impl CommentStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn has_db(&self) -> bool {
        self.db.is_some()
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.db
            .as_ref()
            .expect("CommentStore requires a database handle")
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn add_comment(
        &self,
        path: &str,
        user_id: &str,
        body: &str,
        parent_id: Option<&str>,
    ) -> Result<Comment, String> {
        if body.trim().is_empty() {
            return Err("Comment body cannot be empty".to_string());
        }
        if body.len() > 10_000 {
            return Err("Comment body exceeds 10000 character limit".to_string());
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let updated_at = created_at.clone();

        self.conn()
            .execute(
                "INSERT INTO comments (id, path, user_id, parent_id, body, resolved, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
                params![id, path, user_id, parent_id, body, created_at, updated_at],
            )
            .map_err(|e| format!("Failed to insert comment: {}", e))?;

        Ok(Comment {
            id,
            path: path.to_string(),
            user_id: user_id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            body: body.to_string(),
            resolved: false,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_comments(&self, path: &str) -> Result<Vec<Comment>, String> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, path, user_id, parent_id, body, resolved, created_at, updated_at FROM comments WHERE path = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map(params![path], |row| {
                let resolved: i64 = row.get(5)?;
                Ok(Comment {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    user_id: row.get(2)?,
                    parent_id: row.get(3)?,
                    body: row.get(4)?,
                    resolved: resolved != 0,
                    created_at: row.get::<_, String>(6)?.parse::<DateTime<Utc>>().unwrap_or_default(),
                    updated_at: row.get::<_, String>(7)?.parse::<DateTime<Utc>>().unwrap_or_default(),
                })
            })
            .map_err(|e| format!("Failed to query comments: {}", e))?;

        let mut comments = Vec::new();
        for row in rows {
            comments.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(comments)
    }

    pub fn update_comment(&self, id: &str, user_id: &str, body: &str) -> Result<Comment, String> {
        if body.trim().is_empty() {
            return Err("Comment body cannot be empty".to_string());
        }
        if body.len() > 10_000 {
            return Err("Comment body exceeds 10000 character limit".to_string());
        }

        let conn = self.conn();
        let existing: Option<Comment> = conn
            .query_row(
                "SELECT id, path, user_id, parent_id, body, resolved, created_at, updated_at FROM comments WHERE id = ?1",
                params![id],
                |row| {
                    let resolved: i64 = row.get(5)?;
                    Ok(Comment {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        user_id: row.get(2)?,
                        parent_id: row.get(3)?,
                        body: row.get(4)?,
                        resolved: resolved != 0,
                        created_at: row.get::<_, String>(6)?.parse::<DateTime<Utc>>().unwrap_or_default(),
                        updated_at: row.get::<_, String>(7)?.parse::<DateTime<Utc>>().unwrap_or_default(),
                    })
                },
            )
            .ok();

        let existing = existing.ok_or("Comment not found")?;

        if existing.user_id != user_id {
            return Err("Permission denied: can only edit own comments".to_string());
        }

        let now = Utc::now();
        let updated_at = now.to_rfc3339();
        conn.execute(
            "UPDATE comments SET body = ?1, updated_at = ?2 WHERE id = ?3",
            params![body, updated_at, id],
        )
        .map_err(|e| format!("Failed to update comment: {}", e))?;

        Ok(Comment {
            body: body.to_string(),
            updated_at: now,
            ..existing
        })
    }

    pub fn delete_comment(&self, id: &str, user_id: &str, is_admin: bool) -> Result<(), String> {
        let conn = self.conn();
        let owner: Option<String> = conn
            .query_row("SELECT user_id FROM comments WHERE id = ?1", params![id], |row| {
                row.get::<_, String>(0)
            })
            .ok();

        let owner = owner.ok_or("Comment not found")?;

        if owner != user_id && !is_admin {
            return Err("Permission denied: can only delete own comments".to_string());
        }

        conn.execute("DELETE FROM comments WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete comment: {}", e))?;

        conn.execute("DELETE FROM comments WHERE parent_id = ?1", params![id])
            .map_err(|e| format!("Failed to delete child comments: {}", e))?;

        Ok(())
    }

    pub fn resolve_comment(&self, id: &str, user_id: &str) -> Result<Comment, String> {
        let conn = self.conn();
        let existing: Option<Comment> = conn
            .query_row(
                "SELECT id, path, user_id, parent_id, body, resolved, created_at, updated_at FROM comments WHERE id = ?1",
                params![id],
                |row| {
                    let resolved: i64 = row.get(5)?;
                    Ok(Comment {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        user_id: row.get(2)?,
                        parent_id: row.get(3)?,
                        body: row.get(4)?,
                        resolved: resolved != 0,
                        created_at: row.get::<_, String>(6)?.parse::<DateTime<Utc>>().unwrap_or_default(),
                        updated_at: row.get::<_, String>(7)?.parse::<DateTime<Utc>>().unwrap_or_default(),
                    })
                },
            )
            .ok();

        let existing = existing.ok_or("Comment not found")?;

        if existing.user_id != user_id {
            return Err("Permission denied: can only resolve own comments".to_string());
        }

        let now = Utc::now();
        let updated_at = now.to_rfc3339();
        conn.execute(
            "UPDATE comments SET resolved = 1, updated_at = ?1 WHERE id = ?2",
            params![updated_at, id],
        )
        .map_err(|e| format!("Failed to resolve comment: {}", e))?;

        Ok(Comment {
            resolved: true,
            updated_at: now,
            ..existing
        })
    }

    pub fn load_all_from_db(&self, _conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        Ok(())
    }
}

impl Default for CommentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
pub struct ListCommentsQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub path: String,
    pub body: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub body: String,
}

fn get_user_id(req: &axum::http::Request<axum::body::Body>) -> String {
    req.headers()
        .get("X-Ferro-User")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string()
}

fn is_admin_user<S: CollaborationState>(state: &S, user_id: &str) -> bool {
    state.admin_user() == Some(user_id)
}

async fn log_comment_audit<S: CollaborationState>(state: &S, entry: AuditEntry) {
    state.audit_log().log(entry).await;
}

pub async fn list_comments_handler<S: CollaborationState>(
    State(state): State<S>,
    Query(params): Query<ListCommentsQuery>,
) -> Response {
    let user_id = "anonymous".to_string();

    if state.comments().db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    match state.comments().list_comments(&params.path) {
        Ok(comments) => {
            log_comment_audit(
                &state,
                build_audit_entry(
                    "GET",
                    &format!("/api/comments?path={}", params.path),
                    &user_id,
                    200,
                    None,
                    None,
                ),
            )
            .await;
            (StatusCode::OK, axum::Json(serde_json::json!({ "comments": comments }))).into_response()
        }
        Err(e) => {
            warn!(error = %e, "Failed to list comments");
            ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to list comments")
        }
    }
}

pub async fn create_comment_handler<S: CollaborationState>(
    State(state): State<S>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);

    if state.comments().db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    let (_parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_BODY, "Failed to read request body");
        }
    };

    if body_bytes.len() > 1024 * 1024 {
        return ApiError::bad_request(ApiError::INVALID_BODY, "Request body too large");
    }

    let request: CreateCommentRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::bad_request(ApiError::INVALID_JSON, format!("Invalid JSON: {}", e));
        }
    };

    // Validate the raw path — reject traversal BEFORE normalizing.
    if !common::path::validate_path(&request.path) {
        return ApiError::bad_request(ApiError::PATH_INVALID, "Invalid path: path traversal is not allowed");
    }

    // Now normalize for storage.
    let normalized_path = common::path::normalize_path(&request.path);

    // Sanitize comment body: strip control characters.
    let sanitized_body = sanitize_control_chars(&request.body);
    if contains_html(&sanitized_body) {
        return ApiError::bad_request(
            ApiError::BAD_REQUEST,
            "Comment body contains HTML content, which is not permitted",
        );
    }

    match state.comments().add_comment(
        &normalized_path,
        &user_id,
        &sanitized_body,
        request.parent_id.as_deref(),
    ) {
        Ok(comment) => {
            log_comment_audit(
                &state,
                build_audit_entry(
                    "POST",
                    &format!("/api/comments (path={})", normalized_path),
                    &user_id,
                    200,
                    None,
                    None,
                ),
            )
            .await;
            (StatusCode::CREATED, axum::Json(comment)).into_response()
        }
        Err(e) => ApiError::bad_request(ApiError::BAD_REQUEST, e),
    }
}

pub async fn update_comment_handler<S: CollaborationState>(
    State(state): State<S>,
    Path(id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);

    if state.comments().db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    let (_parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_BODY, "Failed to read request body");
        }
    };

    if body_bytes.len() > 1024 * 1024 {
        return ApiError::bad_request(ApiError::INVALID_BODY, "Request body too large");
    }

    let request: UpdateCommentRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::bad_request(ApiError::INVALID_JSON, format!("Invalid JSON: {}", e));
        }
    };

    match state.comments().update_comment(&id, &user_id, &request.body) {
        Ok(comment) => {
            log_comment_audit(
                &state,
                build_audit_entry("PUT", &format!("/api/comments/{}", id), &user_id, 200, None, None),
            )
            .await;
            (StatusCode::OK, axum::Json(comment)).into_response()
        }
        Err(e) => {
            if e.contains("not found") {
                ApiError::not_found(ApiError::NOT_FOUND, e)
            } else if e.contains("Permission denied") {
                ApiError::forbidden(ApiError::POLICY_DENIED, e)
            } else {
                ApiError::bad_request(ApiError::BAD_REQUEST, e)
            }
        }
    }
}

pub async fn delete_comment_handler<S: CollaborationState>(
    State(state): State<S>,
    Path(id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);
    let admin = is_admin_user(&state, &user_id);

    if state.comments().db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    match state.comments().delete_comment(&id, &user_id, admin) {
        Ok(()) => {
            log_comment_audit(
                &state,
                build_audit_entry("DELETE", &format!("/api/comments/{}", id), &user_id, 200, None, None),
            )
            .await;
            (StatusCode::NO_CONTENT, "").into_response()
        }
        Err(e) => {
            if e.contains("not found") {
                ApiError::not_found(ApiError::NOT_FOUND, e)
            } else if e.contains("Permission denied") {
                ApiError::forbidden(ApiError::POLICY_DENIED, e)
            } else {
                ApiError::internal(ApiError::INTERNAL_ERROR, e)
            }
        }
    }
}

pub async fn resolve_comment_handler<S: CollaborationState>(
    State(state): State<S>,
    Path(id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);

    if state.comments().db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    match state.comments().resolve_comment(&id, &user_id) {
        Ok(comment) => {
            log_comment_audit(
                &state,
                build_audit_entry(
                    "POST",
                    &format!("/api/comments/{}/resolve", id),
                    &user_id,
                    200,
                    None,
                    None,
                ),
            )
            .await;
            (StatusCode::OK, axum::Json(comment)).into_response()
        }
        Err(e) => {
            if e.contains("not found") {
                ApiError::not_found(ApiError::NOT_FOUND, e)
            } else if e.contains("Permission denied") {
                ApiError::forbidden(ApiError::POLICY_DENIED, e)
            } else {
                ApiError::internal(ApiError::INTERNAL_ERROR, e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_store() -> (CommentStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::tests::open_test_db(dir.path().to_str().unwrap());
        let handle = std::sync::Arc::new(std::sync::Mutex::new(conn));
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
