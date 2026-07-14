use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{AutomationState, DbHandle};

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
        let mut stmt =
            match conn.prepare("SELECT id, path_prefix, enabled, created_at FROM worm_policies ORDER BY created_at") {
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

pub fn load_policies(state: &AutomationState) -> Vec<WormPolicy> {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, path_prefix, enabled, created_at FROM worm_policies WHERE enabled = 1 ORDER BY created_at",
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
    } else {
        Vec::new()
    }
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

fn error_bad_request(msg: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "BAD_REQUEST",
        })),
    )
        .into_response()
}

fn error_not_found(msg: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "NOT_FOUND",
        })),
    )
        .into_response()
}

fn error_internal(msg: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "INTERNAL_ERROR",
        })),
    )
        .into_response()
}

pub async fn list_policies(Extension(state): Extension<std::sync::Arc<AutomationState>>) -> Response {
    let policies = load_policies(&state);
    let json: Vec<serde_json::Value> = policies
        .iter()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .collect();

    (StatusCode::OK, axum::Json(serde_json::json!({ "policies": json }))).into_response()
}

pub async fn create_policy(
    Extension(state): Extension<std::sync::Arc<AutomationState>>,
    axum::Json(req): axum::Json<CreateWormPolicyRequest>,
) -> Response {
    if req.path_prefix.trim().is_empty() {
        return error_bad_request("Path prefix must not be empty");
    }

    let policy_id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let policy = WormPolicy {
        id: policy_id.clone(),
        path_prefix: req.path_prefix,
        enabled: req.enabled,
        created_at,
    };

    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO worm_policies (id, path_prefix, enabled, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![policy.id, policy.path_prefix, policy.enabled as i32, policy.created_at],
        ) {
            warn!(error = %e, "failed to create WORM policy");
            return error_internal("Failed to create WORM policy");
        }
    } else {
        return error_internal("Database not available");
    }

    (
        StatusCode::CREATED,
        axum::Json(serde_json::to_value(policy).unwrap_or_else(|e| {
            tracing::error!(error = %e, "failed to serialize WORM policy");
            serde_json::json!({"error": "serialization failed"})
        })),
    )
        .into_response()
}

pub async fn delete_policy(
    Extension(state): Extension<std::sync::Arc<AutomationState>>,
    Path(id): Path<String>,
) -> Response {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute("DELETE FROM worm_policies WHERE id = ?1", params![id]);
        match affected {
            Ok(0) => return error_not_found("WORM policy not found"),
            Ok(_) => return (StatusCode::NO_CONTENT, "").into_response(),
            Err(e) => {
                warn!(error = %e, "failed to delete WORM policy");
                return error_internal("Failed to delete WORM policy");
            }
        }
    }
    error_internal("Database not available")
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
