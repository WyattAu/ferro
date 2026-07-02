//! Guest account management and data retention policies.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::{ApiError, DbHandle, UserMgmtState};
use ferro_server_sharing::guests::validate_guest_expiry;

// ---------------------------------------------------------------------------
// Guest Accounts (G-10)
// ---------------------------------------------------------------------------

/// Request body for creating a guest account.
#[derive(Debug, Deserialize)]
pub struct CreateGuestRequest {
    /// Display name for the guest.
    pub display_name: String,
    /// Email address (optional).
    pub email: Option<String>,
    /// Hours until guest access expires (default: 72).
    pub expires_in_hours: Option<i64>,
    /// Paths the guest can access (comma-separated or JSON array).
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

/// Response for guest account creation.
#[derive(Debug, Serialize)]
pub struct GuestAccountResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub is_guest: bool,
    pub guest_expires_at: String,
}

// ---------------------------------------------------------------------------
// GuestStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct GuestStore {
    db: Option<DbHandle>,
}

impl Default for GuestStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GuestStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn create_guest(
        &self,
        display_name: &str,
        email: &str,
        password_hash: &str,
        expires_at: &str,
    ) -> Result<(String, String), String> {
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };
        let guest_id = uuid::Uuid::new_v4().to_string();
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let guest_username = format!("guest_{}", &uuid_str[..8]);

        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO users (id, username, display_name, email, role, created_at, status, storage_quota_bytes, storage_used_bytes, is_ldap, password_hash, is_guest, guest_expires_at, totp_secret, totp_enabled) VALUES (?1, ?2, ?3, ?4, 'ReadOnly', datetime('now'), 'active', 0, 0, 0, ?5, 1, ?6, NULL, 0)",
            params![
                guest_id,
                guest_username,
                display_name,
                email,
                password_hash,
                expires_at,
            ],
        )
        .map_err(|e| {
            tracing::warn!(error = %e, "failed to create guest account");
            format!("Failed to create guest account: {}", e)
        })?;

        Ok((guest_id, guest_username))
    }

    pub fn list_guests(&self) -> Result<Vec<serde_json::Value>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, username, display_name, email, guest_expires_at FROM users WHERE is_guest = 1 AND status = 'active'",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "username": row.get::<_, String>(1)?,
                    "display_name": row.get::<_, String>(2)?,
                    "email": row.get::<_, String>(3)?,
                    "expires_at": row.get::<_, String>(4)?,
                }))
            })
            .map_err(|e| format!("Failed to query guests: {}", e))?;
        let mut result = Vec::new();
        for row in rows.flatten() {
            result.push(row);
        }
        Ok(result)
    }

    pub fn revoke_guest(&self, id: &str) -> Result<bool, String> {
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn
            .execute(
                "UPDATE users SET status = 'disabled' WHERE id = ?1 AND is_guest = 1",
                params![id],
            )
            .map_err(|e| {
                tracing::warn!(error = %e, "failed to revoke guest");
                format!("Failed to revoke guest account: {}", e)
            })?;
        Ok(affected > 0)
    }

    pub fn check_guest_expiry(&self) -> Result<u32, String> {
        let Some(db) = &self.db else {
            return Ok(0);
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now().to_rfc3339();
        let count = conn
            .execute(
                "UPDATE users SET status = 'disabled' WHERE is_guest = 1 AND status = 'active' AND guest_expires_at IS NOT NULL AND guest_expires_at < ?1",
                params![now],
            )
            .map_err(|e| {
                tracing::warn!(error = %e, "failed to check guest expiry");
                format!("Failed to check guest expiry: {}", e)
            })?;
        if count > 0 {
            tracing::info!(expired_count = count, "disabled expired guest accounts");
        }
        Ok(count as u32)
    }

    pub fn check_single_guest_expiry(&self, username: &str) -> Result<Option<String>, String> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let result: Result<Option<String>, rusqlite::Error> = conn.query_row(
            "SELECT guest_expires_at FROM users WHERE username = ?1 AND is_guest = 1",
            params![username],
            |row| row.get::<_, Option<String>>(0),
        );
        result.map_err(|e| format!("Failed to check guest expiry: {}", e))
    }

    pub fn create_retention_policy(
        &self,
        req: &CreateRetentionPolicyRequest,
    ) -> Result<RetentionPolicyRecord, String> {
        let policy_id = uuid::Uuid::new_v4().to_string();
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO retention_policies (id, name, path_prefix, max_age_days, max_versions) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                policy_id,
                req.name,
                req.path_prefix,
                req.max_age_days as i64,
                req.max_versions.map(|v| v as i64),
            ],
        )
        .map_err(|e| {
            tracing::warn!(error = %e, "failed to create retention policy");
            format!("Failed to create retention policy: {}", e)
        })?;

        Ok(RetentionPolicyRecord {
            id: policy_id,
            name: req.name.clone(),
            path_prefix: req.path_prefix.clone(),
            max_age_days: req.max_age_days,
            max_versions: req.max_versions,
        })
    }

    pub fn list_retention_policies(&self) -> Result<Vec<serde_json::Value>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, name, path_prefix, max_age_days, max_versions, enabled, last_run_at FROM retention_policies ORDER BY created_at",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "path_prefix": row.get::<_, String>(2)?,
                    "max_age_days": row.get::<_, i64>(3)?,
                    "max_versions": row.get::<_, Option<i64>>(4)?,
                    "enabled": row.get::<_, i32>(5)? != 0,
                    "last_run_at": row.get::<_, Option<String>>(6)?,
                }))
            })
            .map_err(|e| format!("Failed to query retention policies: {}", e))?;
        let mut result = Vec::new();
        for row in rows.flatten() {
            result.push(row);
        }
        Ok(result)
    }

    pub fn delete_retention_policy(&self, id: &str) -> Result<bool, String> {
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn
            .execute("DELETE FROM retention_policies WHERE id = ?1", params![id])
            .map_err(|e| {
                tracing::warn!(error = %e, "failed to delete retention policy");
                format!("Failed to delete retention policy: {}", e)
            })?;
        Ok(affected > 0)
    }

    /// Scan for expired guest accounts, disable them, and return IDs of disabled accounts.
    pub fn disable_expired_guests_for_cleanup(&self) -> Vec<(String, String, String)> {
        let Some(db) = &self.db else {
            return Vec::new();
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now().to_rfc3339();
        let expired_guests: Vec<(String, String, String)> = {
            let mut stmt = match conn.prepare(
                "SELECT id, username, guest_expires_at FROM users WHERE is_guest = 1 AND status = 'active' AND guest_expires_at IS NOT NULL AND guest_expires_at < ?1",
            ) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to query expired guests");
                    return Vec::new();
                }
            };

            stmt.query_map(params![now], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default()
        };

        let mut disabled = Vec::new();
        for (id, username, expires_at) in &expired_guests {
            if let Err(e) = conn.execute(
                "UPDATE users SET status = 'disabled' WHERE id = ?1 AND is_guest = 1",
                params![id],
            ) {
                tracing::warn!(error = %e, guest_id = %id, "failed to disable expired guest");
                continue;
            }

            info!(
                guest_id = %id,
                username = %username,
                expires_at = %expires_at,
                "expired guest account disabled"
            );
            disabled.push((id.clone(), username.clone(), expires_at.clone()));
        }

        disabled
    }
}

/// Simple retention policy record for GuestStore responses.
#[derive(Debug, Serialize)]
pub struct RetentionPolicyRecord {
    pub id: String,
    pub name: String,
    pub path_prefix: String,
    pub max_age_days: u32,
    pub max_versions: Option<u32>,
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// `POST /api/admin/guests`
///
/// Create a time-limited guest account with automatic expiry.
pub async fn create_guest<S: UserMgmtState>(
    State(state): State<S>,
    axum::Json(req): axum::Json<CreateGuestRequest>,
) -> Response {
    let expires_at = Utc::now() + Duration::hours(req.expires_in_hours.unwrap_or(72));

    let password = generate_guest_password();
    let password_hash = match ferro_auth::users::hash_password(&password) {
        Ok(h) => h,
        Err(_) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to hash password");
        }
    };

    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    let (guest_id, guest_username) = match store.create_guest(
        &req.display_name,
        req.email.as_deref().unwrap_or(""),
        &password_hash,
        &expires_at.to_rfc3339(),
    ) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, &e);
        }
    };

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": guest_id,
            "username": guest_username,
            "password": password,
            "display_name": req.display_name,
            "email": req.email.unwrap_or_default(),
            "is_guest": true,
            "expires_at": expires_at.to_rfc3339(),
        })),
    )
        .into_response()
}

/// `GET /api/admin/guests`
///
/// List all active guest accounts.
pub async fn list_guests<S: UserMgmtState>(State(state): State<S>) -> Response {
    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => GuestStore::new(),
    };

    let guests = store.list_guests().unwrap_or_default();

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "guests": guests })),
    )
        .into_response()
}

/// `DELETE /api/admin/guests/{id}`
///
/// Revoke a guest account immediately.
pub async fn revoke_guest<S: UserMgmtState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Response {
    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    match store.revoke_guest(&id) {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => ApiError::not_found(ApiError::USER_NOT_FOUND, "Guest not found"),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, &e),
    }
}

// ---------------------------------------------------------------------------
// Data Retention Policies (G-23)
// ---------------------------------------------------------------------------

/// Request body for creating a retention policy.
#[derive(Debug, Deserialize)]
pub struct CreateRetentionPolicyRequest {
    /// Human-readable name for the policy.
    pub name: String,
    /// Path prefix this policy applies to (e.g., "/documents/").
    pub path_prefix: String,
    /// Maximum age of files in days before automatic deletion.
    pub max_age_days: u32,
    /// Maximum number of versions to keep (None = unlimited).
    pub max_versions: Option<u32>,
}

/// `POST /api/admin/retention`
///
/// Create a data retention policy.
pub async fn create_retention_policy<S: UserMgmtState>(
    State(state): State<S>,
    axum::Json(req): axum::Json<CreateRetentionPolicyRequest>,
) -> Response {
    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    match store.create_retention_policy(&req) {
        Ok(record) => (
            StatusCode::CREATED,
            axum::Json(serde_json::json!({
                "id": record.id,
                "name": record.name,
                "path_prefix": record.path_prefix,
                "max_age_days": record.max_age_days,
                "max_versions": record.max_versions,
            })),
        )
            .into_response(),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, &e),
    }
}

/// `GET /api/admin/retention`
///
/// List all retention policies.
pub async fn list_retention_policies<S: UserMgmtState>(State(state): State<S>) -> Response {
    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => GuestStore::new(),
    };

    let policies = store.list_retention_policies().unwrap_or_default();

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "policies": policies })),
    )
        .into_response()
}

/// `DELETE /api/admin/retention/{id}`
///
/// Delete a retention policy.
pub async fn delete_retention_policy<S: UserMgmtState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Response {
    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    match store.delete_retention_policy(&id) {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => ApiError::not_found(ApiError::NOT_FOUND, "Policy not found"),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, &e),
    }
}

// ---------------------------------------------------------------------------
// Guest expiry check (called periodically and during authentication)
// ---------------------------------------------------------------------------

/// Check and disable expired guest accounts.
/// Returns the number of expired guests that were disabled.
pub fn check_guest_expiry<S: UserMgmtState>(state: &S) -> u32 {
    let store = match state.db() {
        Some(db) => GuestStore::new().with_db(db.clone()),
        None => GuestStore::new(),
    };
    store.check_guest_expiry().unwrap_or(0)
}

/// Scan for expired guest accounts, disable them, log audit entries, and
/// return the count of accounts disabled.
async fn cleanup_expired_guests<S: UserMgmtState>(state: &S) -> u32 {
    let disabled_ids = {
        let store = match state.db() {
            Some(db) => GuestStore::new().with_db(db.clone()),
            None => GuestStore::new(),
        };
        store.disable_expired_guests_for_cleanup()
    };

    for (id, _username, _expires_at) in &disabled_ids {
        state
            .audit_log()
            .log(crate::AuditEntry {
                timestamp: Utc::now().to_rfc3339(),
                method: "SYSTEM".to_string(),
                path: format!("/api/admin/guests/{}", id),
                user: "system".to_string(),
                status: 200,
                client_ip: None,
                user_agent: None,
                content_length: None,
            })
            .await;
    }

    let count = disabled_ids.len() as u32;
    if count > 0 {
        info!(
            expired_count = count,
            "guest cleanup: disabled expired guest accounts"
        );
    }
    count
}

/// Spawn a background tokio task that periodically scans for and disables
/// expired guest accounts.
///
/// Follows the same pattern as `retention::spawn_retention_daemon`.
pub fn spawn_guest_cleanup_daemon<S: UserMgmtState + 'static>(
    state: Arc<S>,
    interval_secs: u64,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        if !cancel.is_cancelled() {
            cleanup_expired_guests(&state).await;
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !cancel.is_cancelled() {
                        cleanup_expired_guests(&state).await;
                    }
                }
                _ = cancel.cancelled() => {
                    info!("Guest cleanup daemon shutting down");
                    break;
                }
            }
        }
    });

    info!(
        "Guest cleanup daemon started (interval: {}s)",
        interval_secs
    );
}

// ---------------------------------------------------------------------------
// Guest expiry middleware (axum layer)
// ---------------------------------------------------------------------------

/// Axum middleware that enforces guest account expiry on every request.
///
/// Runs after authentication middleware, so the `UserInfo` extension is
/// available. If the authenticated user's username starts with `guest_`,
/// we look up `guest_expires_at` from the database and reject expired
/// guests with a `GUEST_EXPIRED` error.
pub async fn guest_expiry_middleware<S: UserMgmtState>(
    State(state): State<S>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let user_info = req
        .extensions()
        .get::<crate::UserInfo>()
        .map(|u| u.username.clone());

    let expired = if let Some(ref username) = user_info {
        if username.starts_with("guest_") {
            let store = match state.db() {
                Some(db) => GuestStore::new().with_db(db.clone()),
                None => GuestStore::new(),
            };
            match store.check_single_guest_expiry(username) {
                Ok(Some(expires_at)) => validate_guest_expiry(&expires_at),
                _ => false,
            }
        } else {
            false
        }
    } else {
        false
    };

    if expired {
        return ApiError::unauthorized(ApiError::GUEST_EXPIRED, "Guest account has expired");
    }

    next.run(req).await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a random guest password (16 alphanumeric characters).
fn generate_guest_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_guest_password() {
        let pw1 = generate_guest_password();
        let pw2 = generate_guest_password();
        assert_eq!(pw1.len(), 16);
        assert_eq!(pw2.len(), 16);
        assert_ne!(pw1, pw2, "passwords should be unique");
        assert!(
            pw1.chars().all(|c| c.is_alphanumeric()),
            "password should be alphanumeric"
        );
    }

    #[test]
    fn test_default_guest_expiry() {
        let req = CreateGuestRequest {
            display_name: "Test Guest".to_string(),
            email: None,
            expires_in_hours: None,
            allowed_paths: vec![],
        };
        let expires_at = Utc::now() + Duration::hours(req.expires_in_hours.unwrap_or(72));
        // Should be ~72 hours from now
        let diff = expires_at - Utc::now();
        assert!(diff.num_hours() >= 71 && diff.num_hours() <= 72);
    }

    #[test]
    fn test_validate_guest_expiry_not_expired() {
        let future = (Utc::now() + Duration::hours(24)).to_rfc3339();
        assert!(!validate_guest_expiry(&future));
    }

    #[test]
    fn test_validate_guest_expiry_expired() {
        let past = (Utc::now() - Duration::hours(1)).to_rfc3339();
        assert!(validate_guest_expiry(&past));
    }

    #[test]
    fn test_validate_guest_expiry_invalid_format() {
        assert!(!validate_guest_expiry("not-a-date"));
    }

    #[test]
    fn test_custom_guest_expiry_hours() {
        let req = CreateGuestRequest {
            display_name: "Short Guest".to_string(),
            email: None,
            expires_in_hours: Some(2),
            allowed_paths: vec![],
        };
        let expires_at = Utc::now() + Duration::hours(req.expires_in_hours.unwrap_or(72));
        let diff = expires_at - Utc::now();
        assert!(diff.num_hours() >= 1 && diff.num_hours() <= 2);
    }

    #[test]
    fn test_zero_hours_guest_expiry() {
        let req = CreateGuestRequest {
            display_name: "Zero Guest".to_string(),
            email: None,
            expires_in_hours: Some(0),
            allowed_paths: vec![],
        };
        let expires_at = Utc::now() + Duration::hours(req.expires_in_hours.unwrap_or(72));
        let diff = expires_at - Utc::now();
        assert!(diff.num_seconds().abs() <= 2);
    }
}
