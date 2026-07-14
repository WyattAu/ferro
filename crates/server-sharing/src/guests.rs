use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::SharingState;
use crate::api_error::ApiError;
use crate::audit;

#[derive(Debug, Deserialize)]
pub struct CreateGuestRequest {
    pub display_name: String,
    pub email: Option<String>,
    pub expires_in_hours: Option<i64>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct GuestAccountResponse {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub is_guest: bool,
    pub guest_expires_at: String,
}

pub async fn create_guest(
    Extension(state): Extension<SharingState>,
    axum::Json(req): axum::Json<CreateGuestRequest>,
) -> Response {
    let expires_at = Utc::now() + Duration::hours(req.expires_in_hours.unwrap_or(72));

    let uuid_str = uuid::Uuid::new_v4().to_string();
    let guest_username = format!("guest_{}", &uuid_str[..8]);
    let guest_id = uuid::Uuid::new_v4().to_string();

    let password = generate_guest_password();
    let password_hash = match ferro_auth::users::hash_password(&password) {
        Ok(h) => h,
        Err(_) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to hash password");
        }
    };

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

pub async fn list_guests(Extension(state): Extension<SharingState>) -> Response {
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

    (StatusCode::OK, axum::Json(serde_json::json!({ "guests": guests }))).into_response()
}

pub async fn revoke_guest(Extension(state): Extension<SharingState>, Path(id): Path<String>) -> Response {
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
                return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to revoke guest account");
            }
        }
    }
    ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available")
}

pub fn validate_guest_expiry(guest_expires_at: &str) -> bool {
    match chrono::DateTime::parse_from_rfc3339(guest_expires_at) {
        Ok(expires_at) => expires_at < Utc::now(),
        Err(_) => false,
    }
}

pub fn check_guest_expiry(state: &SharingState) -> u32 {
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

async fn cleanup_expired_guests(state: &SharingState) -> u32 {
    let disabled_ids = if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());

        let now = Utc::now().to_rfc3339();
        let expired_guests: Vec<(String, String, String)> = {
            let mut stmt = match conn.prepare(
                "SELECT id, username, guest_expires_at FROM users WHERE is_guest = 1 AND status = 'active' AND guest_expires_at IS NOT NULL AND guest_expires_at < ?1",
            ) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to query expired guests");
                    return 0;
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
            disabled.push(id.clone());
        }

        disabled
    } else {
        Vec::new()
    };

    for id in &disabled_ids {
        state
            .audit_log
            .log_audit(audit::AuditEntry {
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
        info!(expired_count = count, "guest cleanup: disabled expired guest accounts");
    }
    count
}

pub fn spawn_guest_cleanup_daemon(state: Arc<SharingState>, interval_secs: u64, cancel: CancellationToken) {
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

    info!("Guest cleanup daemon started (interval: {}s)", interval_secs);
}

pub async fn guest_expiry_middleware(
    Extension(state): Extension<SharingState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let user_info = req
        .extensions()
        .get::<ferro_auth::users::UserInfo>()
        .map(|u| u.username.clone());

    let expired = if let Some(ref username) = user_info {
        if username.starts_with("guest_") {
            if let Some(ref db) = state.db {
                let conn = db.lock().unwrap_or_else(|e| e.into_inner());
                let result: Result<Option<String>, rusqlite::Error> = conn.query_row(
                    "SELECT guest_expires_at FROM users WHERE username = ?1 AND is_guest = 1",
                    params![username],
                    |row| row.get::<_, Option<String>>(0),
                );
                drop(conn);
                match result {
                    Ok(Some(expires_at)) => validate_guest_expiry(&expires_at),
                    _ => false,
                }
            } else {
                false
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
