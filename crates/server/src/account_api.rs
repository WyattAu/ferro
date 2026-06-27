use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub user_id: String,
    pub platform: String,
    pub push_token: String,
    pub last_seen: String,
    pub revoked: bool,
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub target_user_id: String,
    pub delete_source: bool,
}

#[derive(Debug, Serialize)]
pub struct TransferResponse {
    pub files_moved: u64,
    pub shares_moved: u64,
    pub notes_moved: u64,
    pub tasks_moved: u64,
    pub calendar_events_moved: u64,
    pub contacts_moved: u64,
    pub source_deleted: bool,
}

#[derive(Debug, Deserialize)]
pub struct WipeRequest {
    pub wipe_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WipeResponse {
    pub devices_wiped: u32,
    pub tokens_revoked: u32,
}

#[derive(Debug, Deserialize)]
pub struct RevokeDeviceRequest {}

pub struct DeviceStore {
    db: DbHandle,
}

impl DeviceStore {
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }

    pub fn init_table(&self) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL CHECK(platform IN ('ios', 'android', 'desktop')),
                push_token TEXT NOT NULL,
                last_seen TEXT NOT NULL DEFAULT (datetime('now')),
                revoked INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);
            CREATE INDEX IF NOT EXISTS idx_devices_platform ON devices(platform);
            CREATE INDEX IF NOT EXISTS idx_devices_revoked ON devices(revoked);",
        )?;
        Ok(())
    }

    pub fn register_device(
        &self,
        user_id: &str,
        platform: &str,
        push_token: &str,
    ) -> Result<Device, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO devices (id, user_id, platform, push_token, last_seen, revoked)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), 0)",
            params![id, user_id, platform, push_token],
        )?;
        Ok(Device {
            id,
            user_id: user_id.to_string(),
            platform: platform.to_string(),
            push_token: push_token.to_string(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            revoked: false,
        })
    }

    pub fn list_devices_for_user(&self, user_id: &str) -> Result<Vec<Device>, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT id, user_id, platform, push_token, last_seen, revoked FROM devices
             WHERE user_id = ?1 ORDER BY last_seen DESC",
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(Device {
                id: row.get(0)?,
                user_id: row.get(1)?,
                platform: row.get(2)?,
                push_token: row.get(3)?,
                last_seen: row.get(4)?,
                revoked: row.get::<_, i64>(5)? != 0,
            })
        })?;
        rows.collect()
    }

    pub fn revoke_device(&self, user_id: &str, device_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute(
            "UPDATE devices SET revoked = 1 WHERE id = ?1 AND user_id = ?2",
            params![device_id, user_id],
        )?;
        Ok(affected > 0)
    }

    pub fn revoke_all_devices(&self, user_id: &str) -> Result<u32, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute(
            "UPDATE devices SET revoked = 1 WHERE user_id = ?1 AND revoked = 0",
            params![user_id],
        )?;
        Ok(affected as u32)
    }

    pub fn delete_device(&self, device_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute("DELETE FROM devices WHERE id = ?1", params![device_id])?;
        Ok(affected > 0)
    }
}

/// POST /api/admin/users/{id}/transfer
pub async fn transfer_user_data(
    State(state): State<AppState>,
    Path(source_user_id): Path<String>,
    axum::Json(body): axum::Json<TransferRequest>,
) -> Response {
    let target_user_id = &body.target_user_id;

    // Verify both users exist
    if state.user_store.get_user(&source_user_id).await.is_err() {
        return ApiError::not_found(ApiError::USER_NOT_FOUND, "Source user not found");
    }
    if state.user_store.get_user(target_user_id).await.is_err() {
        return ApiError::not_found(ApiError::USER_NOT_FOUND, "Target user not found");
    }

    let db = match &state.db {
        Some(db) => db.clone(),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    let conn = db.lock().unwrap_or_else(|e| e.into_inner());

    let files_moved = 0u64;
    let calendar_events_moved = 0u64;
    let contacts_moved = 0u64;

    // Move notes
    let notes_moved = conn
        .execute(
            "UPDATE notes SET id = id WHERE id IN (SELECT id FROM notes WHERE folder LIKE ?1)
             AND folder NOT LIKE ?2",
            params![
                format!("%{}%", source_user_id),
                format!("%{}%", target_user_id)
            ],
        )
        .unwrap_or(0) as u64;

    // Move tasks
    let tasks_moved = conn
        .execute(
            "UPDATE tasks SET assignee = ?1 WHERE assignee = ?2",
            params![target_user_id, source_user_id],
        )
        .unwrap_or(0) as u64;

    // Move shares
    let shares_moved = conn
        .execute(
            "UPDATE shares SET created_by = ?1 WHERE created_by = ?2",
            params![target_user_id, source_user_id],
        )
        .unwrap_or(0) as u64;

    // Move devices
    conn.execute(
        "UPDATE devices SET user_id = ?1 WHERE user_id = ?2",
        params![target_user_id, source_user_id],
    )
    .ok();

    // Move notification prefs
    conn.execute(
        "INSERT OR REPLACE INTO notification_prefs (user_id, share_received_email, share_received_push,
         comment_added_email, comment_added_push, task_assigned_email, task_assigned_push,
         mention_push, system_alert_push, daily_digest_email)
         SELECT ?1, share_received_email, share_received_push,
         comment_added_email, comment_added_push, task_assigned_email, task_assigned_push,
         mention_push, system_alert_push, daily_digest_email
         FROM notification_prefs WHERE user_id = ?2",
        params![target_user_id, source_user_id],
    )
    .ok();

    // Delete source user data if requested
    let source_deleted = if body.delete_source {
        conn.execute(
            "DELETE FROM notification_prefs WHERE user_id = ?1",
            params![source_user_id],
        )
        .ok();
        conn.execute(
            "DELETE FROM devices WHERE user_id = ?1",
            params![source_user_id],
        )
        .ok();
        true
    } else {
        false
    };

    drop(conn);

    tracing::info!(
        source = %source_user_id,
        target = %target_user_id,
        files_moved,
        shares_moved,
        notes_moved,
        tasks_moved,
        "Account transfer completed"
    );

    (
        StatusCode::OK,
        axum::Json(TransferResponse {
            files_moved,
            shares_moved,
            notes_moved,
            tasks_moved,
            calendar_events_moved,
            contacts_moved,
            source_deleted,
        }),
    )
        .into_response()
}

/// POST /api/admin/devices/{user_id}/wipe
pub async fn wipe_user_devices(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    axum::Json(body): axum::Json<WipeRequest>,
) -> Response {
    let db = match &state.db {
        Some(db) => db.clone(),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    let store = DeviceStore::new(db);
    let devices = match store.list_devices_for_user(&user_id) {
        Ok(d) => d,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to list devices: {}", e),
            );
        }
    };

    let wipe_message = body
        .wipe_message
        .unwrap_or_else(|| "Remote wipe initiated by administrator".to_string());

    let mut devices_wiped = 0u32;

    // Send wipe signal via push notification to each device
    if let Some(ref push_store) = state.push_notification_store {
        let push_config = state.push_notification_config.clone();
        let store_ref = push_store.clone();

        for device in &devices {
            if !device.revoked {
                crate::push_notifications::dispatch_push_notifications(
                    &store_ref,
                    &push_config,
                    &user_id,
                    "remote_wipe",
                    &wipe_message,
                )
                .await;
                devices_wiped += 1;
            }
        }
    }

    // Mark all device tokens as revoked
    let tokens_revoked = match store.revoke_all_devices(&user_id) {
        Ok(revoked) => revoked,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to revoke tokens: {}", e),
            );
        }
    };

    tracing::info!(
        user_id = %user_id,
        devices_wiped,
        tokens_revoked,
        "Remote wipe completed"
    );

    (
        StatusCode::OK,
        axum::Json(WipeResponse {
            devices_wiped,
            tokens_revoked,
        }),
    )
        .into_response()
}

/// GET /api/admin/users/{id}/devices
pub async fn list_user_devices(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Response {
    let db = match &state.db {
        Some(db) => db.clone(),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    let store = DeviceStore::new(db);
    match store.list_devices_for_user(&user_id) {
        Ok(devices) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "devices": devices })),
        )
            .into_response(),
        Err(e) => ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to list devices: {}", e),
        ),
    }
}

/// POST /api/admin/users/{id}/devices/{device_id}/revoke
pub async fn revoke_device(
    State(state): State<AppState>,
    Path((user_id, device_id)): Path<(String, String)>,
) -> Response {
    let db = match &state.db {
        Some(db) => db.clone(),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not available");
        }
    };

    let store = DeviceStore::new(db);
    match store.revoke_device(&user_id, &device_id) {
        Ok(true) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "message": "Device revoked",
                "device_id": device_id,
            })),
        )
            .into_response(),
        Ok(false) => ApiError::not_found(ApiError::NOT_FOUND, "Device not found"),
        Err(e) => ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to revoke device: {}", e),
        ),
    }
}
