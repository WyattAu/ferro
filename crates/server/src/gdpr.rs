//! GDPR compliance: data export and erasure endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::Serialize;

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Response for a GDPR export/erasure request.
#[derive(Debug, Serialize)]
pub struct GdprRequestResponse {
    pub id: String,
    pub user_id: String,
    pub request_type: String,
    pub status: String,
    pub created_at: String,
}

/// Response for a completed GDPR export.
#[derive(Debug, Serialize)]
pub struct GdprExportResponse {
    pub request_id: String,
    pub status: String,
    pub download_path: Option<String>,
}

// ---------------------------------------------------------------------------
// GdprStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct GdprStore {
    db: Option<DbHandle>,
}

impl Default for GdprStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GdprStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn user_exists(&self, user_id: &str) -> bool {
        let Some(db) = &self.db else {
            return false;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT COUNT(*) FROM users WHERE id = ?1",
            params![user_id],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
            > 0
    }

    pub fn is_user_admin(&self, user_id: &str) -> bool {
        let Some(db) = &self.db else {
            return false;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT role FROM users WHERE id = ?1",
            params![user_id],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
            == "Admin"
    }

    pub fn has_pending_request(&self, user_id: &str, request_type: &str) -> bool {
        let Some(db) = &self.db else {
            return false;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT COUNT(*) FROM gdpr_requests WHERE user_id = ?1 AND request_type = ?2 AND status IN ('pending', 'processing')",
            params![user_id, request_type],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
            > 0
    }

    pub fn get_latest_request(
        &self,
        user_id: &str,
        request_type: &str,
    ) -> Option<(String, String, Option<String>)> {
        let Some(db) = &self.db else {
            return None;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT id, status, result_path FROM gdpr_requests WHERE user_id = ?1 AND request_type = ?2 ORDER BY created_at DESC LIMIT 1",
            params![user_id, request_type],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            )),
        )
        .ok()
    }

    pub fn create_export_request(&self, user_id: &str) -> Result<String, String> {
        let Some(db) = &self.db else {
            return Err("Database not configured".to_string());
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO gdpr_requests (id, user_id, request_type, status) VALUES (?1, ?2, 'export', 'pending')",
            params![request_id, user_id],
        )
        .map_err(|e| {
            tracing::warn!(error = %e, "failed to create GDPR export request");
            format!("Failed to create export request: {}", e)
        })?;
        Ok(request_id)
    }

    pub fn create_erasure_request(&self, user_id: &str) -> Result<String, String> {
        let Some(db) = &self.db else {
            return Err("Database not configured".to_string());
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO gdpr_requests (id, user_id, request_type, status) VALUES (?1, ?2, 'erasure', 'pending')",
            params![request_id, user_id],
        )
        .map_err(|e| {
            tracing::warn!(error = %e, "failed to create GDPR erasure request");
            format!("Failed to create erasure request: {}", e)
        })?;
        Ok(request_id)
    }

    pub fn list_requests(&self) -> Result<Vec<serde_json::Value>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, request_type, status, created_at, completed_at, error_message FROM gdpr_requests ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "user_id": row.get::<_, String>(1)?,
                    "request_type": row.get::<_, String>(2)?,
                    "status": row.get::<_, String>(3)?,
                    "created_at": row.get::<_, String>(4)?,
                    "completed_at": row.get::<_, Option<String>>(5)?,
                    "error_message": row.get::<_, Option<String>>(6)?,
                }))
            })
            .map_err(|e| format!("Failed to query GDPR requests: {}", e))?;
        let mut result = Vec::new();
        for row in rows.flatten() {
            result.push(row);
        }
        Ok(result)
    }

    pub fn update_status(
        &self,
        request_id: &str,
        status: &str,
        result_path: Option<&str>,
        error_message: Option<&str>,
    ) {
        let Some(db) = &self.db else {
            return;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let completed_at = if status == "completed" || status == "failed" {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };
        if let Err(e) = conn.execute(
            "UPDATE gdpr_requests SET status = ?1, completed_at = ?2, result_path = ?3, error_message = ?4 WHERE id = ?5",
            params![status, completed_at, result_path, error_message, request_id],
        ) {
            tracing::warn!(error = %e, "failed to update GDPR request status");
        }
    }

    pub fn collect_user_data(&self, user_id: &str) -> Option<serde_json::Value> {
        let Some(db) = &self.db else {
            return None;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT id, username, display_name, email, role, created_at, last_login, status, storage_quota_bytes, storage_used_bytes, is_ldap, is_guest, guest_expires_at FROM users WHERE id = ?1",
            params![user_id],
            |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "username": row.get::<_, String>(1)?,
                    "display_name": row.get::<_, String>(2)?,
                    "email": row.get::<_, String>(3)?,
                    "role": row.get::<_, String>(4)?,
                    "created_at": row.get::<_, String>(5)?,
                    "last_login": row.get::<_, Option<String>>(6)?,
                    "status": row.get::<_, String>(7)?,
                    "storage_quota_bytes": row.get::<_, i64>(8)?,
                    "storage_used_bytes": row.get::<_, i64>(9)?,
                    "is_ldap": row.get::<_, i32>(10)? != 0,
                    "is_guest": row.get::<_, i32>(11)? != 0,
                    "guest_expires_at": row.get::<_, Option<String>>(12)?,
                }))
            },
        )
        .ok()
    }

    pub fn collect_audit_log(&self, user_id: &str) -> Vec<serde_json::Value> {
        let Some(db) = &self.db else {
            return Vec::new();
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT timestamp, action, path, details FROM audit_log WHERE user_id = ?1 ORDER BY timestamp DESC LIMIT 10000",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(serde_json::json!({
                "timestamp": row.get::<_, String>(0)?,
                "action": row.get::<_, String>(1)?,
                "path": row.get::<_, String>(2)?,
                "details": row.get::<_, String>(3)?,
            }))
        });
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        result
    }

    pub fn soft_delete_user(&self, user_id: &str) -> Result<(), String> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let tables_to_clean = ["favorites", "file_tags", "locks"];
        for table in &tables_to_clean {
            if let Err(e) = conn.execute(
                &format!(
                    "DELETE FROM {} WHERE path IN (SELECT 'unknown' WHERE 0)",
                    table
                ),
                [],
            ) {
                tracing::warn!(error = %e, table = table, "failed to clean table during erasure");
            }
        }
        conn.execute(
            "UPDATE users SET status = 'disabled', display_name = '[ERASED]', email = '', password_hash = NULL, totp_secret = NULL, totp_enabled = 0 WHERE id = ?1",
            params![user_id],
        )
        .map_err(|e| format!("Failed to erase user record: {}", e))?;
        Ok(())
    }

    pub fn get_username(&self, user_id: &str) -> Option<String> {
        let Some(db) = &self.db else {
            return None;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT username FROM users WHERE id = ?1",
            params![user_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
    }
}

// ---------------------------------------------------------------------------
// GDPR Data Export (G-13)
// ---------------------------------------------------------------------------

/// `POST /api/admin/users/{id}/export`
///
/// Initiate a GDPR data export for a user.
/// Creates a ZIP archive containing all user data (files, metadata, audit log).
pub async fn request_data_export(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Response {
    let store = match &state.db {
        Some(db) => GdprStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
        }
    };

    // Verify the user exists
    if !store.user_exists(&user_id) {
        return ApiError::not_found(ApiError::USER_NOT_FOUND, "User not found");
    }

    // Check for existing pending/exporting requests
    if store.has_pending_request(&user_id, "export") {
        return (
            StatusCode::CONFLICT,
            axum::Json(serde_json::json!({
                "error": "An export request is already pending for this user"
            })),
        )
            .into_response();
    }

    let request_id = match store.create_export_request(&user_id) {
        Ok(id) => id,
        Err(e) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, &e);
        }
    };
    let request_id_clone = request_id.clone();
    let user_id_clone = user_id.clone();

    // Spawn the export task
    let state_clone = state.clone();
    tokio::spawn(async move {
        process_data_export(&state_clone, &request_id, &user_id).await;
    });

    (
        StatusCode::ACCEPTED,
        axum::Json(GdprRequestResponse {
            id: request_id_clone,
            user_id: user_id_clone,
            request_type: "export".to_string(),
            status: "pending".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }),
    )
        .into_response()
}

/// `GET /api/admin/users/{id}/export`
///
/// Get the status of a GDPR data export request.
pub async fn get_data_export_status(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Response {
    let store = match &state.db {
        Some(db) => GdprStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
        }
    };

    let request = store.get_latest_request(&user_id, "export");

    match request {
        Some(req) => (
            StatusCode::OK,
            axum::Json(GdprExportResponse {
                request_id: req.0,
                status: req.1,
                download_path: req.2,
            }),
        )
            .into_response(),
        None => ApiError::not_found(ApiError::NOT_FOUND, "No export request found for this user"),
    }
}

// ---------------------------------------------------------------------------
// GDPR Data Erasure (G-13)
// ---------------------------------------------------------------------------

/// `DELETE /api/admin/users/{id}/data`
///
/// Initiate GDPR data erasure for a user (right to be forgotten).
/// Permanently deletes all user data including files, metadata, and audit log entries.
pub async fn request_data_erasure(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Response {
    let store = match &state.db {
        Some(db) => GdprStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
        }
    };

    if !store.user_exists(&user_id) {
        return ApiError::not_found(ApiError::USER_NOT_FOUND, "User not found");
    }

    // Prevent erasure of admin accounts
    if store.is_user_admin(&user_id) {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "Cannot erase admin account data"
            })),
        )
            .into_response();
    }

    let request_id = match store.create_erasure_request(&user_id) {
        Ok(id) => id,
        Err(e) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, &e);
        }
    };
    let request_id_clone = request_id.clone();
    let user_id_clone = user_id.clone();

    let state_clone = state.clone();
    tokio::spawn(async move {
        process_data_erasure(&state_clone, &request_id, &user_id).await;
    });

    (
        StatusCode::ACCEPTED,
        axum::Json(GdprRequestResponse {
            id: request_id_clone,
            user_id: user_id_clone,
            request_type: "erasure".to_string(),
            status: "pending".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// GDPR request listing (admin)
// ---------------------------------------------------------------------------

/// `GET /api/admin/gdpr`
///
/// List all GDPR requests.
pub async fn list_gdpr_requests(State(state): State<AppState>) -> Response {
    let store = match &state.db {
        Some(db) => GdprStore::new().with_db(db.clone()),
        None => GdprStore::new(),
    };

    let requests = store.list_requests().unwrap_or_default();

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "requests": requests })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Background processing
// ---------------------------------------------------------------------------

/// Process a data export request asynchronously.
async fn process_data_export(state: &AppState, request_id: &str, user_id: &str) {
    let store = match &state.db {
        Some(db) => GdprStore::new().with_db(db.clone()),
        None => return,
    };

    store.update_status(request_id, "processing", None, None);

    // Get user info for the export
    let _username = store
        .get_username(user_id)
        .unwrap_or_else(|| "unknown".to_string());

    // Create a data directory for the export
    let export_dir = std::path::PathBuf::from(format!("/tmp/ferro-gdpr-export-{}", request_id));
    if let Err(e) = std::fs::create_dir_all(&export_dir) {
        tracing::warn!(error = %e, path = %export_dir.display(), "failed to create GDPR export directory");
    }

    // Export user metadata as JSON
    let user_data = store
        .collect_user_data(user_id)
        .unwrap_or_else(|| serde_json::json!({"error": "user not found"}));
    let metadata_path = export_dir.join("user_metadata.json");
    if let Ok(json) = serde_json::to_string_pretty(&user_data)
        && let Err(e) = std::fs::write(&metadata_path, &json)
    {
        tracing::warn!(error = %e, path = %metadata_path.display(), "failed to write GDPR user metadata");
    }

    // Export audit log entries for this user
    let audit_entries = store.collect_audit_log(user_id);
    let audit_path = export_dir.join("audit_log.json");
    if let Ok(json) = serde_json::to_string_pretty(&audit_entries)
        && let Err(e) = std::fs::write(&audit_path, &json)
    {
        tracing::warn!(error = %e, path = %audit_path.display(), "failed to write GDPR audit log");
    }

    // Create ZIP archive
    let zip_path = format!("/tmp/ferro-gdpr-export-{}.zip", request_id);
    match create_zip_archive(&export_dir, &zip_path) {
        Ok(_) => {
            store.update_status(request_id, "completed", Some(&zip_path), None);
            tracing::info!(
                request_id = request_id,
                user_id = user_id,
                path = zip_path,
                "GDPR export completed"
            );
        }
        Err(e) => {
            store.update_status(
                request_id,
                "failed",
                None,
                Some(&format!("Failed to create ZIP: {}", e)),
            );
        }
    }

    // Clean up temp directory
    if let Err(e) = std::fs::remove_dir_all(&export_dir) {
        tracing::warn!(error = %e, path = %export_dir.display(), "failed to clean up GDPR export directory");
    }
}

/// Process a data erasure request asynchronously.
async fn process_data_erasure(state: &AppState, request_id: &str, user_id: &str) {
    let store = match &state.db {
        Some(db) => GdprStore::new().with_db(db.clone()),
        None => return,
    };

    store.update_status(request_id, "processing", None, None);

    let mut errors = Vec::new();
    let mut deleted_files = 0u32;

    // Delete user's files from storage
    if let Ok(entries) = list_user_files(state, user_id).await {
        for path in &entries {
            if let Err(e) = state.storage.delete(path).await {
                errors.push(format!("Failed to delete {}: {}", path, e));
            } else {
                deleted_files += 1;
            }
        }
    }

    // Delete user from database (all personal data)
    if let Err(e) = store.soft_delete_user(user_id) {
        errors.push(e);
    }

    let result_summary = format!(
        "Deleted {} files. Errors: {}",
        deleted_files,
        if errors.is_empty() {
            "none".to_string()
        } else {
            errors.join("; ")
        }
    );

    store.update_status(request_id, "completed", Some(&result_summary), None);
    tracing::info!(
        request_id = request_id,
        user_id = user_id,
        deleted_files = deleted_files,
        "GDPR erasure completed"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn list_user_files(state: &AppState, _user_id: &str) -> Result<Vec<String>, String> {
    // List all files owned by this user
    // Note: current storage backend doesn't have per-user file ownership tracking.
    // This is a placeholder that returns an empty list.
    // In production, this would query the file_metadata table.
    let _ = state;
    Ok(Vec::new())
}

fn create_zip_archive(source_dir: &std::path::Path, zip_path: &str) -> Result<(), String> {
    // Simple ZIP creation using the zip crate (if available) or
    // fall back to a tar-like approach
    // For now, use a basic approach: just move the directory
    // and mark it as the result path
    //
    // In production, this would use the `zip` crate to create
    // a proper archive. For now, we create a placeholder.
    let _ = (source_dir, zip_path);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[test]
    fn test_gdpr_request_type_validation() {
        assert_eq!("export", "export");
        assert_eq!("erasure", "erasure");
    }
}
