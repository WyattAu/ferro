//! Guest account management and data retention policies.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;

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

/// `POST /api/admin/guests`
///
/// Create a time-limited guest account with automatic expiry.
pub async fn create_guest(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateGuestRequest>,
) -> Response {
    let expires_at = Utc::now() + Duration::hours(req.expires_in_hours.unwrap_or(72));

    // Check that the username is not already taken
    let guest_username = format!(
        "guest_{}",
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    );
    let guest_id = uuid::Uuid::new_v4().to_string();

    // Generate a random password for the guest
    let password = generate_guest_password();
    let password_hash = match ferro_auth::users::hash_password(&password) {
        Ok(h) => h,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                &format!("Failed to hash password"),
            );
        }
    };

    // Store in database directly
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO users (id, username, display_name, email, role, created_at, status, storage_quota_bytes, storage_used_bytes, is_ldap, password_hash, is_guest, guest_expires_at, totp_secret, totp_enabled) VALUES (?1, ?2, ?3, ?4, 'ReadOnly', datetime('now'), 'active', 0, 0, 0, ?5, 1, ?6, NULL, 0)",
            params![
                guest_id,
                guest_username,
                req.display_name,
                req.email.as_deref().unwrap_or(""),
                password_hash,
                expires_at.to_rfc3339(),
            ],
        ) {
            tracing::warn!(error = %e, "failed to create guest account");
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create guest account");
        }
    } else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
    }

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
pub async fn list_guests(State(state): State<AppState>) -> Response {
    let guests = if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, username, display_name, email, guest_expires_at FROM users WHERE is_guest = 1 AND status = 'active'",
        ) {
            Ok(s) => s,
            Err(_) => return (StatusCode::OK, axum::Json(serde_json::json!({ "guests": [] }))).into_response(),
        };
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "username": row.get::<_, String>(1)?,
                "display_name": row.get::<_, String>(2)?,
                "email": row.get::<_, String>(3)?,
                "expires_at": row.get::<_, String>(4)?,
            }))
        });
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        result
    } else {
        Vec::new()
    };

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "guests": guests })),
    )
        .into_response()
}

/// `DELETE /api/admin/guests/{id}`
///
/// Revoke a guest account immediately.
pub async fn revoke_guest(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute(
            "UPDATE users SET status = 'disabled' WHERE id = ?1 AND is_guest = 1",
            params![id],
        );
        match affected {
            Ok(0) => return ApiError::not_found(ApiError::USER_NOT_FOUND, "Guest not found"),
            Ok(_) => return (StatusCode::NO_CONTENT, "").into_response(),
            Err(e) => {
                tracing::warn!(error = %e, "failed to revoke guest");
                return ApiError::internal(
                    ApiError::INTERNAL_ERROR,
                    "Failed to revoke guest account",
                );
            }
        }
    }
    ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available")
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
pub async fn create_retention_policy(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateRetentionPolicyRequest>,
) -> Response {
    let policy_id = uuid::Uuid::new_v4().to_string();

    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO retention_policies (id, name, path_prefix, max_age_days, max_versions) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                policy_id,
                req.name,
                req.path_prefix,
                req.max_age_days as i64,
                req.max_versions.map(|v| v as i64),
            ],
        ) {
            tracing::warn!(error = %e, "failed to create retention policy");
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create retention policy");
        }
    }

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": policy_id,
            "name": req.name,
            "path_prefix": req.path_prefix,
            "max_age_days": req.max_age_days,
            "max_versions": req.max_versions,
        })),
    )
        .into_response()
}

/// `GET /api/admin/retention`
///
/// List all retention policies.
pub async fn list_retention_policies(State(state): State<AppState>) -> Response {
    let policies = if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, name, path_prefix, max_age_days, max_versions, enabled, last_run_at FROM retention_policies ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::OK,
                    axum::Json(serde_json::json!({ "policies": [] })),
                )
                    .into_response();
            }
        };
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "path_prefix": row.get::<_, String>(2)?,
                "max_age_days": row.get::<_, i64>(3)?,
                "max_versions": row.get::<_, Option<i64>>(4)?,
                "enabled": row.get::<_, i32>(5)? != 0,
                "last_run_at": row.get::<_, Option<String>>(6)?,
            }))
        });
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        result
    } else {
        Vec::new()
    };

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "policies": policies })),
    )
        .into_response()
}

/// `DELETE /api/admin/retention/{id}`
///
/// Delete a retention policy.
pub async fn delete_retention_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute("DELETE FROM retention_policies WHERE id = ?1", params![id]);
        match affected {
            Ok(0) => return ApiError::not_found(ApiError::NOT_FOUND, "Policy not found"),
            Ok(_) => return (StatusCode::NO_CONTENT, "").into_response(),
            Err(e) => {
                tracing::warn!(error = %e, "failed to delete retention policy");
                return ApiError::internal(
                    ApiError::INTERNAL_ERROR,
                    "Failed to delete retention policy",
                );
            }
        }
    }
    ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available")
}

// ---------------------------------------------------------------------------
// Guest expiry check (called periodically)
// ---------------------------------------------------------------------------

/// Check and disable expired guest accounts.
/// Returns the number of expired guests that were disabled.
pub fn check_guest_expiry(state: &AppState) -> u32 {
    let now = Utc::now().to_rfc3339();
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        match conn.execute(
            "UPDATE users SET status = 'disabled' WHERE is_guest = 1 AND status = 'active' AND guest_expires_at IS NOT NULL AND guest_expires_at < ?1",
            params![now],
        ) {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(expired_count = count, "disabled expired guest accounts");
                }
                count as u32
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to check guest expiry");
                0
            }
        }
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a random guest password (16 alphanumeric characters).
fn generate_guest_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
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
}
