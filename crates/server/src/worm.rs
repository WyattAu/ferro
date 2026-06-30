use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

// ---------------------------------------------------------------------------
// WormPolicyStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WormPolicyStore {
    db: Option<DbHandle>,
}

impl Default for WormPolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WormPolicyStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn list_policies(&self) -> Result<Vec<WormPolicy>, String> {
        let Some(ref db) = self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, path_prefix, enabled, created_at FROM worm_policies ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt.query_map([], WormPolicy::from_row);
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        Ok(result)
    }

    pub fn list_enabled_policies(&self) -> Result<Vec<WormPolicy>, String> {
        let Some(ref db) = self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, path_prefix, enabled, created_at FROM worm_policies WHERE enabled = 1 ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt.query_map([], WormPolicy::from_row);
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        Ok(result)
    }

    pub fn create_policy(&self, req: &CreateWormPolicyRequest) -> Result<WormPolicy, String> {
        if req.path_prefix.trim().is_empty() {
            return Err("Path prefix must not be empty".to_string());
        }
        let Some(ref db) = self.db else {
            return Err("Database not available".to_string());
        };
        let policy_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let policy = WormPolicy {
            id: policy_id,
            path_prefix: req.path_prefix.clone(),
            enabled: req.enabled,
            created_at,
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO worm_policies (id, path_prefix, enabled, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![policy.id, policy.path_prefix, policy.enabled as i32, policy.created_at],
        )
        .map_err(|e| {
            warn!(error = %e, "failed to create WORM policy");
            "Failed to create WORM policy".to_string()
        })?;
        Ok(policy)
    }

    pub fn delete_policy(&self, id: &str) -> Result<bool, String> {
        let Some(ref db) = self.db else {
            return Err("Database not available".to_string());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn
            .execute("DELETE FROM worm_policies WHERE id = ?1", params![id])
            .map_err(|e| {
                warn!(error = %e, "failed to delete WORM policy");
                "Failed to delete WORM policy".to_string()
            })?;
        if affected == 0 {
            return Err("WORM policy not found".to_string());
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WormPolicy {
    pub id: String,
    pub path_prefix: String,
    pub enabled: bool,
    pub created_at: String,
}

impl WormPolicy {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            path_prefix: row.get(1)?,
            enabled: row.get::<_, i32>(2)? != 0,
            created_at: row.get(3)?,
        })
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateWormPolicyRequest {
    pub path_prefix: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

pub trait WormStoreTrait: Send + Sync {
    fn list_policies(&self) -> Vec<WormPolicy>;
    fn create_policy(&self, policy: &WormPolicy) -> Result<(), rusqlite::Error>;
    fn delete_policy(&self, id: &str) -> Result<bool, rusqlite::Error>;
}

pub struct SqliteWormStore {
    db: DbHandle,
}

impl SqliteWormStore {
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }
}

impl WormStoreTrait for SqliteWormStore {
    fn list_policies(&self) -> Vec<WormPolicy> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, path_prefix, enabled, created_at FROM worm_policies ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([], WormPolicy::from_row);
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        result
    }

    fn create_policy(&self, policy: &WormPolicy) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO worm_policies (id, path_prefix, enabled, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![policy.id, policy.path_prefix, policy.enabled as i32, policy.created_at],
        )?;
        Ok(())
    }

    fn delete_policy(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute("DELETE FROM worm_policies WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

pub fn load_policies(state: &AppState) -> Vec<WormPolicy> {
    state.worm_store.list_enabled_policies().unwrap_or_default()
}

pub fn is_worm_protected(path: &str, policies: &[WormPolicy]) -> bool {
    for policy in policies {
        if !policy.enabled {
            continue;
        }
        let prefix = policy.path_prefix.trim_end_matches('/');
        let normalized = path.trim_end_matches('/');
        if normalized == prefix || normalized.starts_with(&format!("{}/", prefix)) {
            return true;
        }
    }
    false
}

pub async fn list_policies(State(state): State<AppState>) -> Response {
    let policies = match state.worm_store.list_policies() {
        Ok(p) => p,
        Err(e) => {
            warn!(error = %e, "failed to list WORM policies");
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to list WORM policies");
        }
    };
    let json: Vec<serde_json::Value> = policies
        .iter()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .collect();

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "policies": json })),
    )
        .into_response()
}

pub async fn create_policy(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateWormPolicyRequest>,
) -> Response {
    if req.path_prefix.trim().is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Path prefix must not be empty");
    }

    match state.worm_store.create_policy(&req) {
        Ok(policy) => (
            StatusCode::CREATED,
            axum::Json(serde_json::to_value(policy).unwrap_or_else(|e| {
                tracing::error!(error = %e, "failed to serialize WORM policy");
                serde_json::json!({"error": "serialization failed"})
            })),
        )
            .into_response(),
        Err(e) => {
            warn!(error = %e, "failed to create WORM policy");
            ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create WORM policy")
        }
    }
}

pub async fn delete_policy(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.worm_store.delete_policy(&id) {
        Ok(_) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => {
            if e == "WORM policy not found" {
                ApiError::not_found(ApiError::NOT_FOUND, "WORM policy not found")
            } else {
                warn!(error = %e, "failed to delete WORM policy");
                ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to delete WORM policy")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_worm_protected_exact_match() {
        let policies = vec![WormPolicy {
            id: "1".into(),
            path_prefix: "/archive".into(),
            enabled: true,
            created_at: "2025-01-01T00:00:00+00:00".into(),
        }];
        assert!(is_worm_protected("/archive", &policies));
        assert!(is_worm_protected("/archive/file.txt", &policies));
        assert!(!is_worm_protected("/other/file.txt", &policies));
    }

    #[test]
    fn test_is_worm_protected_disabled_policy() {
        let policies = vec![WormPolicy {
            id: "1".into(),
            path_prefix: "/archive".into(),
            enabled: false,
            created_at: "2025-01-01T00:00:00+00:00".into(),
        }];
        assert!(!is_worm_protected("/archive", &policies));
        assert!(!is_worm_protected("/archive/file.txt", &policies));
    }

    #[test]
    fn test_is_worm_protected_trailing_slash() {
        let policies = vec![WormPolicy {
            id: "1".into(),
            path_prefix: "/archive/".into(),
            enabled: true,
            created_at: "2025-01-01T00:00:00+00:00".into(),
        }];
        assert!(is_worm_protected("/archive", &policies));
        assert!(is_worm_protected("/archive/file.txt", &policies));
    }

    #[test]
    fn test_is_worm_protected_root() {
        let policies = vec![WormPolicy {
            id: "1".into(),
            path_prefix: "/".into(),
            enabled: true,
            created_at: "2025-01-01T00:00:00+00:00".into(),
        }];
        assert!(is_worm_protected("/anything", &policies));
        assert!(is_worm_protected("/", &policies));
    }

    #[test]
    fn test_is_worm_protected_empty_policies() {
        assert!(!is_worm_protected("/anything", &[]));
    }

    #[test]
    fn test_is_worm_protected_multiple_policies() {
        let policies = vec![
            WormPolicy {
                id: "1".into(),
                path_prefix: "/archive".into(),
                enabled: true,
                created_at: "2025-01-01T00:00:00+00:00".into(),
            },
            WormPolicy {
                id: "2".into(),
                path_prefix: "/legal".into(),
                enabled: true,
                created_at: "2025-01-01T00:00:00+00:00".into(),
            },
        ];
        assert!(is_worm_protected("/archive/doc.txt", &policies));
        assert!(is_worm_protected("/legal/contract.pdf", &policies));
        assert!(!is_worm_protected("/tmp/file.txt", &policies));
    }
}
