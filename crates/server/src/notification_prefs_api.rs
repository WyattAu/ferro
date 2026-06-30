use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPrefs {
    pub user_id: String,
    pub share_received_email: bool,
    pub share_received_push: bool,
    pub comment_added_email: bool,
    pub comment_added_push: bool,
    pub task_assigned_email: bool,
    pub task_assigned_push: bool,
    pub mention_push: bool,
    pub system_alert_push: bool,
    pub daily_digest_email: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationPrefsRequest {
    pub share_received_email: Option<bool>,
    pub share_received_push: Option<bool>,
    pub comment_added_email: Option<bool>,
    pub comment_added_push: Option<bool>,
    pub task_assigned_email: Option<bool>,
    pub task_assigned_push: Option<bool>,
    pub mention_push: Option<bool>,
    pub system_alert_push: Option<bool>,
    pub daily_digest_email: Option<bool>,
}

#[derive(Clone)]
pub struct NotificationPrefsStore {
    db: Option<DbHandle>,
}

impl Default for NotificationPrefsStore {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationPrefsStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn init_table(&self) -> Result<(), rusqlite::Error> {
        let Some(ref db) = self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notification_prefs (
                user_id TEXT PRIMARY KEY NOT NULL,
                share_received_email INTEGER NOT NULL DEFAULT 1,
                share_received_push INTEGER NOT NULL DEFAULT 1,
                comment_added_email INTEGER NOT NULL DEFAULT 1,
                comment_added_push INTEGER NOT NULL DEFAULT 1,
                task_assigned_email INTEGER NOT NULL DEFAULT 1,
                task_assigned_push INTEGER NOT NULL DEFAULT 1,
                mention_push INTEGER NOT NULL DEFAULT 1,
                system_alert_push INTEGER NOT NULL DEFAULT 1,
                daily_digest_email INTEGER NOT NULL DEFAULT 1
            );",
        )?;
        Ok(())
    }

    pub fn get_prefs(&self, user_id: &str) -> Result<NotificationPrefs, rusqlite::Error> {
        let Some(ref db) = self.db else {
            return Ok(NotificationPrefs {
                user_id: user_id.to_string(),
                share_received_email: true,
                share_received_push: true,
                comment_added_email: true,
                comment_added_push: true,
                task_assigned_email: true,
                task_assigned_push: true,
                mention_push: true,
                system_alert_push: true,
                daily_digest_email: true,
            });
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT user_id, share_received_email, share_received_push,
             comment_added_email, comment_added_push,
             task_assigned_email, task_assigned_push,
             mention_push, system_alert_push, daily_digest_email
             FROM notification_prefs WHERE user_id = ?1",
        )?;
        let mut rows = stmt.query_map(params![user_id], |row| {
            Ok(NotificationPrefs {
                user_id: row.get(0)?,
                share_received_email: row.get::<_, i64>(1)? != 0,
                share_received_push: row.get::<_, i64>(2)? != 0,
                comment_added_email: row.get::<_, i64>(3)? != 0,
                comment_added_push: row.get::<_, i64>(4)? != 0,
                task_assigned_email: row.get::<_, i64>(5)? != 0,
                task_assigned_push: row.get::<_, i64>(6)? != 0,
                mention_push: row.get::<_, i64>(7)? != 0,
                system_alert_push: row.get::<_, i64>(8)? != 0,
                daily_digest_email: row.get::<_, i64>(9)? != 0,
            })
        })?;
        match rows.next() {
            Some(row) => row,
            None => {
                // Return default prefs if none exist
                Ok(NotificationPrefs {
                    user_id: user_id.to_string(),
                    share_received_email: true,
                    share_received_push: true,
                    comment_added_email: true,
                    comment_added_push: true,
                    task_assigned_email: true,
                    task_assigned_push: true,
                    mention_push: true,
                    system_alert_push: true,
                    daily_digest_email: true,
                })
            }
        }
    }

    pub fn update_prefs(
        &self,
        user_id: &str,
        updates: &UpdateNotificationPrefsRequest,
    ) -> Result<NotificationPrefs, rusqlite::Error> {
        // Get current prefs or defaults
        let current = self.get_prefs(user_id)?;

        let share_received_email = updates
            .share_received_email
            .unwrap_or(current.share_received_email);
        let share_received_push = updates
            .share_received_push
            .unwrap_or(current.share_received_push);
        let comment_added_email = updates
            .comment_added_email
            .unwrap_or(current.comment_added_email);
        let comment_added_push = updates
            .comment_added_push
            .unwrap_or(current.comment_added_push);
        let task_assigned_email = updates
            .task_assigned_email
            .unwrap_or(current.task_assigned_email);
        let task_assigned_push = updates
            .task_assigned_push
            .unwrap_or(current.task_assigned_push);
        let mention_push = updates.mention_push.unwrap_or(current.mention_push);
        let system_alert_push = updates
            .system_alert_push
            .unwrap_or(current.system_alert_push);
        let daily_digest_email = updates
            .daily_digest_email
            .unwrap_or(current.daily_digest_email);

        let result = NotificationPrefs {
            user_id: user_id.to_string(),
            share_received_email,
            share_received_push,
            comment_added_email,
            comment_added_push,
            task_assigned_email,
            task_assigned_push,
            mention_push,
            system_alert_push,
            daily_digest_email,
        };

        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "INSERT OR REPLACE INTO notification_prefs
                 (user_id, share_received_email, share_received_push,
                  comment_added_email, comment_added_push,
                  task_assigned_email, task_assigned_push,
                  mention_push, system_alert_push, daily_digest_email)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    user_id,
                    share_received_email as i64,
                    share_received_push as i64,
                    comment_added_email as i64,
                    comment_added_push as i64,
                    task_assigned_email as i64,
                    task_assigned_push as i64,
                    mention_push as i64,
                    system_alert_push as i64,
                    daily_digest_email as i64,
                ],
            )?;
        }

        Ok(result)
    }
}

/// GET /api/notification-prefs
pub async fn get_notification_prefs(State(state): State<AppState>) -> Response {
    // For now, use a default user ID since auth middleware should extract this
    let user_id = "default";

    match state.notification_prefs_store.get_prefs(user_id) {
        Ok(prefs) => (StatusCode::OK, axum::Json(prefs)).into_response(),
        Err(e) => ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to get notification preferences: {}", e),
        ),
    }
}

/// PUT /api/notification-prefs
pub async fn update_notification_prefs(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<UpdateNotificationPrefsRequest>,
) -> Response {
    // For now, use a default user ID since auth middleware should extract this
    let user_id = "default";

    match state.notification_prefs_store.update_prefs(user_id, &body) {
        Ok(prefs) => (StatusCode::OK, axum::Json(prefs)).into_response(),
        Err(e) => ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to update notification preferences: {}", e),
        ),
    }
}
