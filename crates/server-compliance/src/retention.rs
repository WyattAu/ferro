use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::AuditEntry;
use crate::ComplianceState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub id: String,
    pub name: String,
    pub path_prefix: String,
    pub max_age_seconds: u64,
    pub max_file_count: Option<u64>,
    pub min_free_bytes: Option<u64>,
    pub dry_run: bool,
    pub enabled: bool,
}

impl RetentionPolicy {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
        let max_age_seconds: i64 = row.get(3)?;
        let max_file_count: Option<i64> = row.get(4)?;
        let min_free_bytes: Option<i64> = row.get(5)?;
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            path_prefix: row.get(2)?,
            max_age_seconds: max_age_seconds.max(0) as u64,
            max_file_count: max_file_count.map(|v| v.max(0) as u64),
            min_free_bytes: min_free_bytes.map(|v| v.max(0) as u64),
            dry_run: row.get::<_, i32>(6)? != 0,
            enabled: row.get::<_, i32>(7)? != 0,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateRetentionPolicyRequest {
    pub name: String,
    pub path_prefix: String,
    #[serde(default = "default_max_age")]
    pub max_age_seconds: u64,
    pub max_file_count: Option<u64>,
    pub min_free_bytes: Option<u64>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_max_age() -> u64 {
    90 * 24 * 3600
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct RetentionExecutionResult {
    pub policy_id: String,
    pub policy_name: String,
    pub scanned_files: u64,
    pub deleted_files: u64,
    pub deleted_bytes: u64,
    pub dry_run: bool,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RetentionStore {
    db: Option<crate::DbHandle>,
}

impl Default for RetentionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RetentionStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: crate::DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn list_policies(&self) -> Result<Vec<RetentionPolicy>, String> {
        let db = self.db.as_ref().ok_or("Database not available")?;
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, name, path_prefix, max_age_seconds, max_file_count, min_free_bytes, dry_run, enabled FROM retention_policies ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt.query_map([], RetentionPolicy::from_row);
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        Ok(result)
    }

    pub fn list_enabled_policies(&self) -> Result<Vec<RetentionPolicy>, String> {
        let db = self.db.as_ref().ok_or("Database not available")?;
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, name, path_prefix, max_age_seconds, max_file_count, min_free_bytes, dry_run, enabled FROM retention_policies WHERE enabled = 1",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(Vec::new()),
        };
        let rows = stmt.query_map([], RetentionPolicy::from_row);
        let mut result = Vec::new();
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        Ok(result)
    }

    pub fn create_policy(&self, req: &CreateRetentionPolicyRequest) -> Result<RetentionPolicy, String> {
        let db = self.db.as_ref().ok_or("Database not available")?;
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());

        let policy_id = uuid::Uuid::new_v4().to_string();

        if let Err(e) = conn.execute(
            "INSERT INTO retention_policies (id, name, path_prefix, max_age_days, max_age_seconds, max_file_count, min_free_bytes, dry_run, enabled) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8)",
            params![
                policy_id,
                req.name,
                req.path_prefix,
                req.max_age_seconds as i64,
                req.max_file_count.map(|v| v as i64),
                req.min_free_bytes.map(|v| v as i64),
                req.dry_run as i32,
                req.enabled as i32,
            ],
        ) {
            warn!(error = %e, "failed to create retention policy");
            return Err("Failed to create retention policy".to_string());
        }

        Ok(RetentionPolicy {
            id: policy_id,
            name: req.name.clone(),
            path_prefix: req.path_prefix.clone(),
            max_age_seconds: req.max_age_seconds,
            max_file_count: req.max_file_count,
            min_free_bytes: req.min_free_bytes,
            dry_run: req.dry_run,
            enabled: req.enabled,
        })
    }

    pub fn delete_policy(&self, id: &str) -> Result<bool, String> {
        let db = self.db.as_ref().ok_or("Database not available")?;
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn
            .execute("DELETE FROM retention_policies WHERE id = ?1", params![id])
            .map_err(|e| {
                warn!(error = %e, "failed to delete retention policy");
                "Failed to delete retention policy".to_string()
            })?;
        Ok(affected > 0)
    }

    pub fn update_last_run(&self) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            let now = chrono::Utc::now().to_rfc3339();
            if let Err(e) = conn.execute(
                "UPDATE retention_policies SET last_run_at = ?1 WHERE enabled = 1",
                params![now],
            ) {
                warn!(error = %e, "failed to update retention policy last_run_at");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_policies<S: ComplianceState>(State(state): State<S>) -> Response {
    let policies = state.retention_store().list_policies().unwrap_or_default();

    (StatusCode::OK, axum::Json(serde_json::json!({ "policies": policies }))).into_response()
}

pub async fn create_policy<S: ComplianceState>(
    State(state): State<S>,
    axum::Json(req): axum::Json<CreateRetentionPolicyRequest>,
) -> Response {
    use crate::ApiError;

    if req.name.trim().is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Policy name must not be empty");
    }
    if req.path_prefix.trim().is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Path prefix must not be empty");
    }
    if req.max_age_seconds == 0 && req.max_file_count.is_none() && req.min_free_bytes.is_none() {
        return ApiError::bad_request(
            ApiError::BAD_REQUEST,
            "At least one of max_age_seconds, max_file_count, or min_free_bytes must be set",
        );
    }

    match state.retention_store().create_policy(&req) {
        Ok(policy) => (
            StatusCode::CREATED,
            axum::Json(serde_json::to_value(policy).unwrap_or_else(|e| {
                tracing::error!(error = %e, "failed to serialize retention policy");
                serde_json::json!({"error": "serialization failed"})
            })),
        )
            .into_response(),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, &e),
    }
}

pub async fn delete_policy<S: ComplianceState>(State(state): State<S>, Path(id): Path<String>) -> Response {
    use crate::ApiError;

    match state.retention_store().delete_policy(&id) {
        Ok(true) => (StatusCode::NO_CONTENT, "").into_response(),
        Ok(false) => ApiError::not_found(ApiError::NOT_FOUND, "Policy not found"),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, &e),
    }
}

pub async fn execute_policies<S: ComplianceState>(State(state): State<S>) -> Response {
    let policies = state.retention_store().list_enabled_policies().unwrap_or_default();
    if policies.is_empty() {
        return (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "results": [],
                "message": "No enabled retention policies to execute",
            })),
        )
            .into_response();
    }

    let mut results = Vec::new();
    for policy in &policies {
        let result = execute_single_policy(&state, policy).await;
        results.push(result);
    }

    state.retention_store().update_last_run();

    (StatusCode::OK, axum::Json(serde_json::json!({ "results": results }))).into_response()
}

async fn execute_single_policy<S: ComplianceState>(state: &S, policy: &RetentionPolicy) -> RetentionExecutionResult {
    let mut result = RetentionExecutionResult {
        policy_id: policy.id.clone(),
        policy_name: policy.name.clone(),
        scanned_files: 0,
        deleted_files: 0,
        deleted_bytes: 0,
        dry_run: policy.dry_run,
        errors: Vec::new(),
    };

    let entries = match state.storage().list_all(&policy.path_prefix, 10000).await {
        Ok(e) => e,
        Err(e) => {
            let msg = format!("Failed to list files under {}: {}", policy.path_prefix, e);
            warn!("{}", msg);
            result.errors.push(msg);
            return result;
        }
    };

    let now = chrono::Utc::now();
    let mut file_entries: Vec<_> = entries
        .into_iter()
        .filter(|m| !m.is_collection && m.path.starts_with(&policy.path_prefix))
        .collect();

    file_entries.sort_by_key(|b| std::cmp::Reverse(b.modified_at));

    let mut to_delete_paths: Vec<(String, u64)> = Vec::new();
    let mut to_delete_set: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (idx, meta) in file_entries.iter().enumerate() {
        if to_delete_set.contains(&meta.path) {
            continue;
        }

        let age_secs = now.signed_duration_since(meta.modified_at).num_seconds().max(0) as u64;

        if policy.max_age_seconds > 0 && age_secs > policy.max_age_seconds {
            to_delete_paths.push((meta.path.clone(), meta.size));
            to_delete_set.insert(meta.path.clone());
            continue;
        }

        if let Some(max_count) = policy.max_file_count
            && (idx + 1) as u64 > max_count
        {
            to_delete_paths.push((meta.path.clone(), meta.size));
            to_delete_set.insert(meta.path.clone());
            continue;
        }
    }

    if let Some(min_free) = policy.min_free_bytes {
        let total_bytes: u64 = file_entries.iter().map(|m| m.size).sum();
        let used = state.used_bytes().load(std::sync::atomic::Ordering::Relaxed);
        if used + total_bytes > min_free {
            let deficit = (used + total_bytes).saturating_sub(min_free);
            let mut freed: u64 = 0;
            for meta in &file_entries {
                if freed >= deficit {
                    break;
                }
                if !to_delete_set.contains(&meta.path) {
                    to_delete_paths.push((meta.path.clone(), meta.size));
                    to_delete_set.insert(meta.path.clone());
                    freed += meta.size;
                }
            }
        }
    }

    for (path, size) in &to_delete_paths {
        if policy.dry_run {
            info!(
                policy = %policy.name,
                path = %path,
                size = size,
                "retention: would delete (dry-run)"
            );
        } else {
            match state.storage().delete(path).await {
                Ok(()) => {
                    result.deleted_files += 1;
                    result.deleted_bytes += size;
                    state
                        .audit_log()
                        .log(AuditEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            method: "DELETE".to_string(),
                            path: path.clone(),
                            user: "system".to_string(),
                            status: 200,
                            client_ip: None,
                            user_agent: None,
                            content_length: Some(*size),
                        })
                        .await;
                }
                Err(e) => {
                    let msg = format!("Failed to delete {}: {}", path, e);
                    warn!("{}", msg);
                    result.errors.push(msg);
                }
            }
        }
    }

    info!(
        policy = %policy.name,
        scanned = result.scanned_files,
        deleted = result.deleted_files,
        dry_run = policy.dry_run,
        "retention policy execution completed"
    );

    result
}

pub fn spawn_retention_daemon<S: ComplianceState>(state: Arc<S>, interval_secs: u64, cancel: CancellationToken) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        if !cancel.is_cancelled() {
            run_retention_check(state.as_ref()).await;
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !cancel.is_cancelled() {
                        run_retention_check(state.as_ref()).await;
                    }
                }
                _ = cancel.cancelled() => {
                    info!("Retention daemon shutting down");
                    break;
                }
            }
        }
    });

    info!("Retention daemon started (interval: {}s)", interval_secs);
}

async fn run_retention_check<S: ComplianceState>(state: &S) {
    let policies = match state.retention_store().list_enabled_policies() {
        Ok(p) => p,
        Err(_) => return,
    };
    if policies.is_empty() {
        return;
    }

    info!("Running {} retention policies", policies.len());
    for policy in &policies {
        execute_single_policy(state, policy).await;
    }
    state.retention_store().update_last_run();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(name: &str, path_prefix: &str, max_age_seconds: u64) -> CreateRetentionPolicyRequest {
        CreateRetentionPolicyRequest {
            name: name.to_string(),
            path_prefix: path_prefix.to_string(),
            max_age_seconds,
            max_file_count: None,
            min_free_bytes: None,
            dry_run: false,
            enabled: true,
        }
    }

    fn make_request_with_counts(
        name: &str,
        path_prefix: &str,
        max_file_count: Option<u64>,
        min_free_bytes: Option<u64>,
    ) -> CreateRetentionPolicyRequest {
        CreateRetentionPolicyRequest {
            name: name.to_string(),
            path_prefix: path_prefix.to_string(),
            max_age_seconds: 0,
            max_file_count,
            min_free_bytes,
            dry_run: false,
            enabled: true,
        }
    }

    fn init_db() -> crate::DbHandle {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS retention_policies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path_prefix TEXT NOT NULL,
                max_age_days INTEGER NOT NULL DEFAULT 0,
                max_age_seconds INTEGER NOT NULL DEFAULT 0,
                max_file_count INTEGER,
                min_free_bytes INTEGER,
                dry_run INTEGER NOT NULL DEFAULT 0,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_run_at TEXT
            );",
        )
        .unwrap();
        std::sync::Arc::new(std::sync::Mutex::new(conn))
    }

    #[test]
    fn test_create_request_validation() {
        let req = CreateRetentionPolicyRequest {
            name: "test".to_string(),
            path_prefix: "/logs".to_string(),
            max_age_seconds: 3600,
            max_file_count: None,
            min_free_bytes: None,
            dry_run: true,
            enabled: true,
        };
        assert_eq!(req.name, "test");
        assert_eq!(req.path_prefix, "/logs");
        assert_eq!(req.max_age_seconds, 3600);
        assert!(req.dry_run);
        assert!(req.enabled);
    }

    #[test]
    fn test_default_max_age() {
        assert_eq!(default_max_age(), 90 * 24 * 3600);
    }

    #[test]
    fn test_default_enabled() {
        assert!(default_true());
    }

    #[test]
    fn test_retention_store_new_no_db() {
        let store = RetentionStore::new();
        assert!(store.db.is_none());
    }

    #[test]
    fn test_retention_store_list_policies_no_db() {
        let store = RetentionStore::new();
        let result = store.list_policies();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Database not available");
    }

    #[test]
    fn test_retention_store_list_policies_empty_db() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        let policies = store.list_policies().unwrap();
        assert!(policies.is_empty());
    }

    #[test]
    fn test_retention_store_list_enabled_policies_no_db() {
        let store = RetentionStore::new();
        let result = store.list_enabled_policies();
        assert!(result.is_err());
    }

    #[test]
    fn test_retention_store_list_enabled_policies_empty_db() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        let policies = store.list_enabled_policies().unwrap();
        assert!(policies.is_empty());
    }

    #[test]
    fn test_retention_store_create_policy_no_db() {
        let store = RetentionStore::new();
        let req = make_request("test", "/logs", 3600);
        let result = store.create_policy(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_retention_store_create_and_list_policy() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        let req = make_request("daily-cleanup", "/logs/app", 86400);
        let policy = store.create_policy(&req).unwrap();

        assert_eq!(policy.name, "daily-cleanup");
        assert_eq!(policy.path_prefix, "/logs/app");
        assert_eq!(policy.max_age_seconds, 86400);
        assert!(policy.enabled);
        assert!(!policy.dry_run);
        assert!(!policy.id.is_empty());

        let policies = store.list_policies().unwrap();
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].name, "daily-cleanup");
    }

    #[test]
    fn test_retention_store_create_multiple_policies() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);

        let req1 = make_request("daily", "/logs", 86400);
        let req2 = make_request("weekly", "/logs", 604800);
        store.create_policy(&req1).unwrap();
        store.create_policy(&req2).unwrap();

        let policies = store.list_policies().unwrap();
        assert_eq!(policies.len(), 2);
    }

    #[test]
    fn test_retention_store_create_policy_with_counts() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        let req = make_request_with_counts("count-limit", "/data", Some(1000), Some(1_000_000_000));
        let policy = store.create_policy(&req).unwrap();

        assert_eq!(policy.max_file_count, Some(1000));
        assert_eq!(policy.min_free_bytes, Some(1_000_000_000));
        assert_eq!(policy.max_age_seconds, 0);
    }

    #[test]
    fn test_retention_store_delete_policy_not_found() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        let result = store.delete_policy("nonexistent-id").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_retention_store_delete_policy_success() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        let req = make_request("to-delete", "/tmp", 3600);
        let policy = store.create_policy(&req).unwrap();

        let deleted = store.delete_policy(&policy.id).unwrap();
        assert!(deleted);

        let policies = store.list_policies().unwrap();
        assert!(policies.is_empty());
    }

    #[test]
    fn test_retention_store_list_enabled_filters_disabled() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);

        let req_enabled = make_request("enabled", "/logs", 3600);
        store.create_policy(&req_enabled).unwrap();

        let mut req_disabled = make_request("disabled", "/data", 3600);
        req_disabled.enabled = false;
        store.create_policy(&req_disabled).unwrap();

        let all = store.list_policies().unwrap();
        assert_eq!(all.len(), 2);

        let enabled = store.list_enabled_policies().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "enabled");
    }

    #[test]
    fn test_retention_store_update_last_run_no_db() {
        let store = RetentionStore::new();
        store.update_last_run();
    }

    #[test]
    fn test_retention_store_update_last_run_with_db() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);
        store.update_last_run();
    }

    #[test]
    fn test_retention_policy_from_row_negative_age_clamped() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE retention_policies (
                id TEXT, name TEXT, path_prefix TEXT, max_age_days INTEGER,
                max_age_seconds INTEGER, max_file_count INTEGER, min_free_bytes INTEGER,
                dry_run INTEGER, enabled INTEGER, created_at TEXT
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO retention_policies (id, name, path_prefix, max_age_seconds, max_file_count, min_free_bytes, dry_run, enabled) VALUES ('1', 'test', '/logs', -100, NULL, NULL, 0, 1)",
            [],
        )
        .unwrap();

        let policy: RetentionPolicy = conn
            .query_row(
                "SELECT id, name, path_prefix, max_age_seconds, max_file_count, min_free_bytes, dry_run, enabled FROM retention_policies WHERE id = '1'",
                [],
                RetentionPolicy::from_row,
            )
            .unwrap();

        assert_eq!(policy.max_age_seconds, 0);
        assert_eq!(policy.max_file_count, None);
        assert_eq!(policy.min_free_bytes, None);
    }

    #[test]
    fn test_retention_store_default() {
        let store = RetentionStore::default();
        assert!(store.db.is_none());
    }

    #[test]
    fn test_retention_store_create_and_delete_cycle() {
        let db = init_db();
        let store = RetentionStore::new().with_db(db);

        let req = make_request("cycle-test", "/archive", 2592000);
        let policy = store.create_policy(&req).unwrap();
        assert_eq!(store.list_policies().unwrap().len(), 1);

        store.delete_policy(&policy.id).unwrap();
        assert_eq!(store.list_policies().unwrap().len(), 0);
    }

    #[test]
    fn test_retention_policy_serialization() {
        let policy = RetentionPolicy {
            id: "test-id".into(),
            name: "test-policy".into(),
            path_prefix: "/data".into(),
            max_age_seconds: 86400,
            max_file_count: Some(500),
            min_free_bytes: Some(500_000_000),
            dry_run: true,
            enabled: false,
        };
        let json = serde_json::to_value(&policy).unwrap();
        assert_eq!(json["id"], "test-id");
        assert_eq!(json["name"], "test-policy");
        assert_eq!(json["path_prefix"], "/data");
        assert_eq!(json["max_age_seconds"], 86400);
        assert_eq!(json["max_file_count"], 500);
        assert_eq!(json["min_free_bytes"], 500_000_000);
        assert_eq!(json["dry_run"], true);
        assert_eq!(json["enabled"], false);
    }

    #[test]
    fn test_retention_execution_result_serialization() {
        let result = RetentionExecutionResult {
            policy_id: "p1".into(),
            policy_name: "test".into(),
            scanned_files: 100,
            deleted_files: 5,
            deleted_bytes: 1024,
            dry_run: false,
            errors: vec!["some error".into()],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["scanned_files"], 100);
        assert_eq!(json["deleted_files"], 5);
        assert_eq!(json["deleted_bytes"], 1024);
        assert_eq!(json["dry_run"], false);
        assert_eq!(json["errors"][0], "some error");
    }
}
