//! Data Loss Prevention (DLP) API endpoints.
//!
//! Manages DLP policies (file type restrictions, content pattern detection,
//! file size limits, external share restrictions) and provides file scanning
//! against configured policies.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::ComplianceState;

/// A DLP policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpPolicy {
    pub id: String,
    pub name: String,
    pub policy_type: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

/// Request body for creating a DLP policy.
#[derive(Debug, Deserialize)]
pub struct CreateDlpPolicyRequest {
    pub name: String,
    pub policy_type: String,
    pub enabled: Option<bool>,
    pub config: serde_json::Value,
}

/// Request body for updating a DLP policy.
#[derive(Debug, Deserialize)]
pub struct UpdateDlpPolicyRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
}

/// A DLP alert (scan violation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpAlert {
    pub id: String,
    pub policy_id: String,
    pub policy_name: String,
    pub file_path: String,
    pub violation_type: String,
    pub details: String,
    pub severity: String,
    pub created_at: String,
}

/// Scan result for a single policy check.
#[derive(Debug, Serialize)]
pub struct DlpScanResult {
    pub file_path: String,
    pub violations: Vec<DlpViolation>,
    pub safe: bool,
}

/// A single policy violation.
#[derive(Debug, Serialize)]
pub struct DlpViolation {
    pub policy_id: String,
    pub policy_name: String,
    pub policy_type: String,
    pub description: String,
    pub severity: String,
}

/// Known file extensions that should be blocked.
const BLOCKED_EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe",
    "msi",
    "bat",
    "cmd",
    "com",
    "pif",
    "scr",
    "vbs",
    "vbe",
    "js",
    "jse",
    "ws",
    "wsh",
    "ps1",
    "psm1",
    "psd1",
    "reg",
    "dll",
    "sys",
    "cpl",
    "inf",
    "hta",
    "cda",
    "lnk",
    "application",
    "gadget",
];

/// Content patterns for detection (regex-based).
fn content_patterns() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("credit_card", r"\b(?:\d[ -]*?){13,16}\b", "Credit card number detected"),
        ("ssn", r"\b\d{3}-\d{2}-\d{4}\b", "Social Security Number detected"),
        (
            "email",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
            "Email address detected",
        ),
        (
            "phone",
            r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b",
            "Phone number detected",
        ),
        (
            "ip_address",
            r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",
            "IP address detected",
        ),
    ]
}

// ---------------------------------------------------------------------------
// DlpStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct DlpStore {
    db: Option<crate::DbHandle>,
}

impl Default for DlpStore {
    fn default() -> Self {
        Self::new()
    }
}

impl DlpStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: crate::DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn init_tables(&self) -> Result<(), String> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dlp_policies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                policy_type TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                config TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS dlp_alerts (
                id TEXT PRIMARY KEY,
                policy_id TEXT NOT NULL,
                policy_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                violation_type TEXT NOT NULL,
                details TEXT NOT NULL,
                severity TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to init DLP tables: {}", e))
    }

    pub fn list_policies(&self) -> Result<Vec<DlpPolicy>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, name, policy_type, enabled, config, created_at, updated_at FROM dlp_policies ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let enabled: i32 = row.get(3)?;
                let config_str: String = row.get(4)?;
                let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null);
                Ok(DlpPolicy {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    policy_type: row.get(2)?,
                    enabled: enabled != 0,
                    config,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to query policies: {}", e))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_policy(&self, id: &str) -> Result<Option<DlpPolicy>, String> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, name, policy_type, enabled, config, created_at, updated_at FROM dlp_policies WHERE id = ?1",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                let enabled: i32 = row.get(3)?;
                let config_str: String = row.get(4)?;
                let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null);
                Ok(DlpPolicy {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    policy_type: row.get(2)?,
                    enabled: enabled != 0,
                    config,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to query policy: {}", e))?;
        rows.next()
            .transpose()
            .map_err(|e| format!("Failed to read policy: {}", e))
    }

    pub fn create_policy(&self, req: &CreateDlpPolicyRequest) -> Result<DlpPolicy, String> {
        let policy_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let enabled = req.enabled.unwrap_or(true);
        let config_str = serde_json::to_string(&req.config).map_err(|e| format!("Invalid config: {}", e))?;

        let valid_types = ["file_type", "content_pattern", "file_size", "external_share"];
        if !valid_types.contains(&req.policy_type.as_str()) {
            return Err(format!("Invalid policy_type. Must be one of: {:?}", valid_types));
        }

        let policy = DlpPolicy {
            id: policy_id,
            name: req.name.clone(),
            policy_type: req.policy_type.clone(),
            enabled,
            config: req.config.clone(),
            created_at: now.clone(),
            updated_at: now,
        };

        if let Some(db) = &self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "INSERT INTO dlp_policies (id, name, policy_type, enabled, config, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    policy.id,
                    policy.name,
                    policy.policy_type,
                    policy.enabled as i32,
                    config_str,
                    policy.created_at,
                    policy.updated_at,
                ],
            )
            .map_err(|e| format!("Failed to create policy: {}", e))?;
        }

        Ok(policy)
    }

    pub fn update_policy(&self, id: &str, req: &UpdateDlpPolicyRequest) -> Result<Option<DlpPolicy>, String> {
        let Some(db) = &self.db else {
            return Err("Database not configured".to_string());
        };

        let conn = db.lock().unwrap_or_else(|e| e.into_inner());

        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM dlp_policies WHERE id = ?1", params![id], |row| {
                row.get::<_, i64>(0)
            })
            .map(|c| c > 0)
            .unwrap_or(false);

        if !exists {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        if let Some(name) = &req.name
            && let Err(e) = conn.execute(
                "UPDATE dlp_policies SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, id],
            )
        {
            tracing::error!(error = %e, "DLP policy name update failed");
        }

        if let Some(enabled) = req.enabled
            && let Err(e) = conn.execute(
                "UPDATE dlp_policies SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![enabled as i32, now, id],
            )
        {
            tracing::error!(error = %e, "DLP policy enabled update failed");
        }

        if let Some(config) = &req.config
            && let Ok(config_str) = serde_json::to_string(config)
            && let Err(e) = conn.execute(
                "UPDATE dlp_policies SET config = ?1, updated_at = ?2 WHERE id = ?3",
                params![config_str, now, id],
            )
        {
            tracing::error!(error = %e, "DLP policy config update failed");
        }

        let mut stmt = conn
            .prepare(
                "SELECT id, name, policy_type, enabled, config, created_at, updated_at FROM dlp_policies WHERE id = ?1",
            )
            .map_err(|e| format!("Failed to reload policy: {}", e))?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                let enabled: i32 = row.get(3)?;
                let config_str: String = row.get(4)?;
                let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null);
                Ok(DlpPolicy {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    policy_type: row.get(2)?,
                    enabled: enabled != 0,
                    config,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to reload policy: {}", e))?;

        match rows.next().transpose() {
            Ok(Some(p)) => Ok(Some(p)),
            _ => Err("Failed to reload policy".to_string()),
        }
    }

    pub fn delete_policy(&self, id: &str) -> Result<bool, String> {
        let Some(db) = &self.db else {
            return Err("Database not configured".to_string());
        };

        let conn = db.lock().unwrap_or_else(|e| e.into_inner());

        let affected = conn
            .execute("DELETE FROM dlp_policies WHERE id = ?1", params![id])
            .unwrap_or(0);

        if affected == 0 { Ok(false) } else { Ok(true) }
    }

    pub fn list_alerts(&self) -> Result<Vec<DlpAlert>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, policy_id, policy_name, file_path, violation_type, details, severity, created_at FROM dlp_alerts ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DlpAlert {
                    id: row.get(0)?,
                    policy_id: row.get(1)?,
                    policy_name: row.get(2)?,
                    file_path: row.get(3)?,
                    violation_type: row.get(4)?,
                    details: row.get(5)?,
                    severity: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("Failed to query alerts: {}", e))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_alert(
        &self,
        policy_id: &str,
        policy_name: &str,
        file_path: &str,
        violation_type: &str,
        details: &str,
        severity: &str,
    ) -> Result<(), String> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let alert_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO dlp_alerts (id, policy_id, policy_name, file_path, violation_type, details, severity, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                alert_id,
                policy_id,
                policy_name,
                file_path,
                violation_type,
                details,
                severity,
                now,
            ],
        )
        .map_err(|e| format!("Failed to record alert: {}", e))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// GET /api/dlp/policies — List DLP policies.
pub async fn list_policies<S: ComplianceState>(State(state): State<S>) -> Response {
    let policies = state.dlp_store().list_policies().unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "policies": policies,
            "total": policies.len(),
        })),
    )
        .into_response()
}

/// POST /api/dlp/policies — Create a DLP policy.
pub async fn create_policy<S: ComplianceState>(
    State(state): State<S>,
    Json(req): Json<CreateDlpPolicyRequest>,
) -> Response {
    let store = state.dlp_store();

    match store.create_policy(&req) {
        Ok(policy) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "message": "Policy created",
                "policy": {
                    "id": policy.id,
                    "name": policy.name,
                    "policy_type": policy.policy_type,
                    "enabled": policy.enabled,
                    "config": policy.config,
                    "created_at": policy.created_at,
                    "updated_at": policy.updated_at,
                },
            })),
        )
            .into_response(),
        Err(e) => {
            if e.starts_with("Invalid") {
                (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e })),
                )
                    .into_response()
            }
        }
    }
}

/// PUT /api/dlp/policies/{id} — Update a DLP policy.
pub async fn update_policy<S: ComplianceState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDlpPolicyRequest>,
) -> Response {
    let store = state.dlp_store();

    match store.update_policy(&id, &req) {
        Ok(Some(policy)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Policy updated",
                "policy": policy,
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Policy not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// DELETE /api/dlp/policies/{id} — Delete a DLP policy.
pub async fn delete_policy<S: ComplianceState>(State(state): State<S>, Path(id): Path<String>) -> Response {
    let store = state.dlp_store();

    match store.delete_policy(&id) {
        Ok(true) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Policy deleted",
                "id": id,
            })),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Policy not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// POST /api/dlp/scan/{path} — Scan file against DLP policies.
pub async fn scan_file_dlp<S: ComplianceState>(State(state): State<S>, Path(file_path): Path<String>) -> Response {
    let content = match state.storage().get(&file_path).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("File not found: {}", e),
                })),
            )
                .into_response();
        }
    };

    let store = state.dlp_store();
    let policies = store.list_policies().unwrap_or_default();

    let mut violations = Vec::new();

    for policy in &policies {
        if !policy.enabled {
            continue;
        }

        match policy.policy_type.as_str() {
            "file_type" => {
                if let Some(blocked) = policy.config.get("blocked_extensions")
                    && let Some(exts) = blocked.as_array()
                    && let Some(dot_pos) = file_path.rfind('.')
                {
                    let ext = &file_path[dot_pos + 1..].to_lowercase();
                    for blocked_ext in exts {
                        if let Some(bext) = blocked_ext.as_str()
                            && *ext == bext.to_lowercase()
                        {
                            violations.push(DlpViolation {
                                policy_id: policy.id.clone(),
                                policy_name: policy.name.clone(),
                                policy_type: policy.policy_type.clone(),
                                description: format!("Blocked file extension: .{}", ext),
                                severity: policy
                                    .config
                                    .get("severity")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("high")
                                    .to_string(),
                            });
                        }
                    }
                }
                // Default: block executables if no specific config
                if policy.config.get("blocked_extensions").is_none()
                    && let Some(dot_pos) = file_path.rfind('.')
                {
                    let ext = &file_path[dot_pos + 1..].to_lowercase();
                    if BLOCKED_EXECUTABLE_EXTENSIONS.contains(&ext.as_str()) {
                        violations.push(DlpViolation {
                            policy_id: policy.id.clone(),
                            policy_name: policy.name.clone(),
                            policy_type: policy.policy_type.clone(),
                            description: format!("Executable file blocked: .{}", ext),
                            severity: "high".to_string(),
                        });
                    }
                }
            }
            "content_pattern" => {
                let content_str = String::from_utf8_lossy(&content);
                let patterns = content_patterns();
                let patterns_to_check =
                    if let Some(check_patterns) = policy.config.get("patterns").and_then(|p| p.as_array()) {
                        check_patterns.iter().filter_map(|p| p.as_str()).collect::<Vec<_>>()
                    } else {
                        patterns.iter().map(|(name, _, _)| *name).collect()
                    };

                for (name, regex, description) in &patterns {
                    if !patterns_to_check.contains(name) {
                        continue;
                    }
                    if let Ok(re) = regex::Regex::new(regex)
                        && re.is_match(&content_str)
                    {
                        violations.push(DlpViolation {
                            policy_id: policy.id.clone(),
                            policy_name: policy.name.clone(),
                            policy_type: policy.policy_type.clone(),
                            description: description.to_string(),
                            severity: policy
                                .config
                                .get("severity")
                                .and_then(|s| s.as_str())
                                .unwrap_or("medium")
                                .to_string(),
                        });
                    }
                }
            }
            "file_size" => {
                if let Some(max_size) = policy.config.get("max_size_bytes").and_then(|s| s.as_u64())
                    && content.len() as u64 > max_size
                {
                    violations.push(DlpViolation {
                        policy_id: policy.id.clone(),
                        policy_name: policy.name.clone(),
                        policy_type: policy.policy_type.clone(),
                        description: format!("File size {} exceeds limit of {} bytes", content.len(), max_size),
                        severity: policy
                            .config
                            .get("severity")
                            .and_then(|s| s.as_str())
                            .unwrap_or("medium")
                            .to_string(),
                    });
                }
            }
            "external_share" => {
                // Check if file is shared externally (placeholder logic)
                if let Some(deny_external) = policy.config.get("deny_external").and_then(|s| s.as_bool())
                    && deny_external
                {
                    // In a real implementation, check if file is in a shared context
                    // For now, flag files that look like they might be shared
                }
            }
            _ => {}
        }
    }

    // Record violations as alerts
    if !violations.is_empty() {
        for violation in &violations {
            if let Err(e) = store.record_alert(
                &violation.policy_id,
                &violation.policy_name,
                &file_path,
                &violation.policy_type,
                &violation.description,
                &violation.severity,
            ) {
                tracing::error!(error = %e, "DLP alert record failed");
            }
        }
    }

    let safe = violations.is_empty();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "file_path": file_path,
            "safe": safe,
            "violations": violations,
            "policies_checked": policies.iter().filter(|p| p.enabled).count(),
        })),
    )
        .into_response()
}

/// GET /api/dlp/alerts — List DLP alerts.
pub async fn list_alerts<S: ComplianceState>(State(state): State<S>) -> Response {
    let alerts = state.dlp_store().list_alerts().unwrap_or_default();

    let total = alerts.len();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "alerts": alerts,
            "total": total,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_db() -> crate::DbHandle {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let store = DlpStore::new().with_db(std::sync::Arc::new(std::sync::Mutex::new(conn)));
        store.init_tables().unwrap();
        store.db.unwrap()
    }

    fn make_request(name: &str, policy_type: &str, config: serde_json::Value) -> CreateDlpPolicyRequest {
        CreateDlpPolicyRequest {
            name: name.to_string(),
            policy_type: policy_type.to_string(),
            enabled: None,
            config,
        }
    }

    #[test]
    fn test_blocked_executable_extensions() {
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"exe"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"bat"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"ps1"));
        assert!(!BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"txt"));
    }

    #[test]
    fn test_content_patterns_exist() {
        let patterns = content_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|(name, _, _)| *name == "credit_card"));
        assert!(patterns.iter().any(|(name, _, _)| *name == "ssn"));
    }

    #[test]
    fn test_blocked_extensions_completeness() {
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"msi"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"vbs"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"cmd"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"com"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"dll"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"sys"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"reg"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"lnk"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"hta"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"ws"));
        assert!(BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"wsh"));
        assert!(!BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"pdf"));
        assert!(!BLOCKED_EXECUTABLE_EXTENSIONS.contains(&"docx"));
    }

    #[test]
    fn test_content_patterns_all_present() {
        let patterns = content_patterns();
        let names: Vec<&str> = patterns.iter().map(|(name, _, _)| *name).collect();
        assert!(names.contains(&"credit_card"));
        assert!(names.contains(&"ssn"));
        assert!(names.contains(&"email"));
        assert!(names.contains(&"phone"));
        assert!(names.contains(&"ip_address"));
    }

    // --- DlpStore tests ---

    #[test]
    fn test_dlp_store_new_no_db() {
        let store = DlpStore::new();
        assert!(store.db.is_none());
    }

    #[test]
    fn test_dlp_store_default() {
        let store = DlpStore::default();
        assert!(store.db.is_none());
    }

    #[test]
    fn test_dlp_store_init_tables_no_db() {
        let store = DlpStore::new();
        let result = store.init_tables();
        assert!(result.is_ok());
    }

    #[test]
    fn test_dlp_store_init_tables_with_db() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let db = std::sync::Arc::new(std::sync::Mutex::new(conn));
        let store = DlpStore::new().with_db(db);
        let result = store.init_tables();
        assert!(result.is_ok());
    }

    #[test]
    fn test_dlp_store_list_policies_no_db() {
        let store = DlpStore::new();
        let policies = store.list_policies().unwrap();
        assert!(policies.is_empty());
    }

    #[test]
    fn test_dlp_store_list_policies_empty_db() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let policies = store.list_policies().unwrap();
        assert!(policies.is_empty());
    }

    #[test]
    fn test_dlp_store_get_policy_no_db() {
        let store = DlpStore::new();
        let result = store.get_policy("some-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_dlp_store_get_policy_not_found() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let result = store.get_policy("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_dlp_store_create_policy_file_type() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({
            "blocked_extensions": ["exe", "bat", "ps1"]
        });
        let req = make_request("Block Executables", "file_type", config);
        let policy = store.create_policy(&req).unwrap();

        assert_eq!(policy.name, "Block Executables");
        assert_eq!(policy.policy_type, "file_type");
        assert!(policy.enabled);
        assert!(!policy.id.is_empty());
    }

    #[test]
    fn test_dlp_store_create_policy_content_pattern() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({
            "patterns": ["credit_card", "ssn"],
            "severity": "high"
        });
        let req = make_request("PII Detection", "content_pattern", config);
        let policy = store.create_policy(&req).unwrap();

        assert_eq!(policy.policy_type, "content_pattern");
        assert_eq!(policy.config["patterns"][0], "credit_card");
    }

    #[test]
    fn test_dlp_store_create_policy_file_size() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({
            "max_size_bytes": 10485760
        });
        let req = make_request("Size Limit", "file_size", config);
        let policy = store.create_policy(&req).unwrap();

        assert_eq!(policy.policy_type, "file_size");
        assert_eq!(policy.config["max_size_bytes"], 10485760);
    }

    #[test]
    fn test_dlp_store_create_policy_external_share() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({
            "deny_external": true
        });
        let req = make_request("External Share Block", "external_share", config);
        let policy = store.create_policy(&req).unwrap();

        assert_eq!(policy.policy_type, "external_share");
    }

    #[test]
    fn test_dlp_store_create_policy_invalid_type() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Bad Policy", "invalid_type", config);
        let result = store.create_policy(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid policy_type"));
    }

    #[test]
    fn test_dlp_store_create_policy_all_valid_types() {
        let valid_types = ["file_type", "content_pattern", "file_size", "external_share"];
        for ptype in valid_types {
            let db = init_db();
            let store = DlpStore::new().with_db(db);
            let config = serde_json::json!({});
            let req = make_request(&format!("Policy {}", ptype), ptype, config);
            let result = store.create_policy(&req);
            assert!(result.is_ok(), "Failed for type: {}", ptype);
        }
    }

    #[test]
    fn test_dlp_store_create_policy_disabled() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let mut req = make_request("Disabled Policy", "file_type", config);
        req.enabled = Some(false);
        let policy = store.create_policy(&req).unwrap();
        assert!(!policy.enabled);
    }

    #[test]
    fn test_dlp_store_create_policy_enabled_by_default() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Default Policy", "file_type", config);
        let policy = store.create_policy(&req).unwrap();
        assert!(policy.enabled);
    }

    #[test]
    fn test_dlp_store_get_policy_after_create() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Find Me", "file_type", config);
        let created = store.create_policy(&req).unwrap();

        let found = store.get_policy(&created.id).unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "Find Me");
        assert_eq!(found.id, created.id);
    }

    #[test]
    fn test_dlp_store_update_policy_not_found() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let req = UpdateDlpPolicyRequest {
            name: Some("new-name".into()),
            enabled: None,
            config: None,
        };
        let result = store.update_policy("nonexistent", &req).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_dlp_store_update_policy_no_db() {
        let store = DlpStore::new();
        let req = UpdateDlpPolicyRequest {
            name: Some("new-name".into()),
            enabled: None,
            config: None,
        };
        let result = store.update_policy("some-id", &req);
        assert!(result.is_err());
    }

    #[test]
    fn test_dlp_store_update_policy_name() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Original Name", "file_type", config);
        let created = store.create_policy(&req).unwrap();

        let update = UpdateDlpPolicyRequest {
            name: Some("Updated Name".into()),
            enabled: None,
            config: None,
        };
        let updated = store.update_policy(&created.id, &update).unwrap();
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().name, "Updated Name");
    }

    #[test]
    fn test_dlp_store_update_policy_enabled() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Toggle Me", "file_type", config);
        let created = store.create_policy(&req).unwrap();

        let update = UpdateDlpPolicyRequest {
            name: None,
            enabled: Some(false),
            config: None,
        };
        let updated = store.update_policy(&created.id, &update).unwrap();
        assert!(updated.is_some());
        assert!(!updated.unwrap().enabled);
    }

    #[test]
    fn test_dlp_store_update_policy_config() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({"max_size_bytes": 100});
        let req = make_request("Size Policy", "file_size", config);
        let created = store.create_policy(&req).unwrap();

        let new_config = serde_json::json!({"max_size_bytes": 999});
        let update = UpdateDlpPolicyRequest {
            name: None,
            enabled: None,
            config: Some(new_config),
        };
        let updated = store.update_policy(&created.id, &update).unwrap();
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().config["max_size_bytes"], 999);
    }

    #[test]
    fn test_dlp_store_update_policy_multiple_fields() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Multi Update", "file_type", config);
        let created = store.create_policy(&req).unwrap();

        let update = UpdateDlpPolicyRequest {
            name: Some("New Name".into()),
            enabled: Some(false),
            config: Some(serde_json::json!({"blocked_extensions": ["exe"]})),
        };
        let updated = store.update_policy(&created.id, &update).unwrap();
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name, "New Name");
        assert!(!updated.enabled);
        assert_eq!(updated.config["blocked_extensions"][0], "exe");
    }

    #[test]
    fn test_dlp_store_delete_policy_no_db() {
        let store = DlpStore::new();
        let result = store.delete_policy("some-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_dlp_store_delete_policy_not_found() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let deleted = store.delete_policy("nonexistent").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_dlp_store_delete_policy_success() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Delete Me", "file_type", config);
        let created = store.create_policy(&req).unwrap();

        let deleted = store.delete_policy(&created.id).unwrap();
        assert!(deleted);

        let found = store.get_policy(&created.id).unwrap();
        assert!(found.is_none());
    }

    // --- Alert tests ---

    #[test]
    fn test_dlp_store_list_alerts_no_db() {
        let store = DlpStore::new();
        let alerts = store.list_alerts().unwrap();
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_dlp_store_list_alerts_empty_db() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let alerts = store.list_alerts().unwrap();
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_dlp_store_record_alert_no_db() {
        let store = DlpStore::new();
        let result = store.record_alert("pid", "pname", "/file", "type", "details", "high");
        assert!(result.is_ok());
    }

    #[test]
    fn test_dlp_store_record_and_list_alerts() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);

        store
            .record_alert(
                "policy-1",
                "PII Policy",
                "/docs/secret.txt",
                "content_pattern",
                "Credit card detected",
                "high",
            )
            .unwrap();

        let alerts = store.list_alerts().unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].policy_id, "policy-1");
        assert_eq!(alerts[0].policy_name, "PII Policy");
        assert_eq!(alerts[0].file_path, "/docs/secret.txt");
        assert_eq!(alerts[0].violation_type, "content_pattern");
        assert_eq!(alerts[0].details, "Credit card detected");
        assert_eq!(alerts[0].severity, "high");
        assert!(!alerts[0].id.is_empty());
    }

    #[test]
    fn test_dlp_store_multiple_alerts() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);

        store
            .record_alert("p1", "Policy 1", "/a.txt", "file_type", "Blocked ext", "medium")
            .unwrap();
        store
            .record_alert("p2", "Policy 2", "/b.txt", "content_pattern", "SSN found", "high")
            .unwrap();
        store
            .record_alert("p1", "Policy 1", "/c.txt", "file_size", "Too large", "low")
            .unwrap();

        let alerts = store.list_alerts().unwrap();
        assert_eq!(alerts.len(), 3);
    }

    // --- Serialization tests ---

    #[test]
    fn test_dlp_policy_serialization() {
        let policy = DlpPolicy {
            id: "p1".into(),
            name: "Test Policy".into(),
            policy_type: "file_type".into(),
            enabled: true,
            config: serde_json::json!({"blocked_extensions": ["exe"]}),
            created_at: "2025-01-01T00:00:00+00:00".into(),
            updated_at: "2025-01-01T00:00:00+00:00".into(),
        };
        let json = serde_json::to_value(&policy).unwrap();
        assert_eq!(json["id"], "p1");
        assert_eq!(json["name"], "Test Policy");
        assert_eq!(json["policy_type"], "file_type");
        assert_eq!(json["enabled"], true);
        assert_eq!(json["config"]["blocked_extensions"][0], "exe");
    }

    #[test]
    fn test_dlp_alert_serialization() {
        let alert = DlpAlert {
            id: "a1".into(),
            policy_id: "p1".into(),
            policy_name: "PII Policy".into(),
            file_path: "/secret.txt".into(),
            violation_type: "content_pattern".into(),
            details: "Credit card detected".into(),
            severity: "high".into(),
            created_at: "2025-01-01T00:00:00+00:00".into(),
        };
        let json = serde_json::to_value(&alert).unwrap();
        assert_eq!(json["id"], "a1");
        assert_eq!(json["severity"], "high");
    }

    #[test]
    fn test_dlp_scan_result_serialization() {
        let result = DlpScanResult {
            file_path: "/test.txt".into(),
            violations: vec![DlpViolation {
                policy_id: "p1".into(),
                policy_name: "PII Policy".into(),
                policy_type: "content_pattern".into(),
                description: "Credit card detected".into(),
                severity: "high".into(),
            }],
            safe: false,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["safe"], false);
        assert_eq!(json["violations"][0]["severity"], "high");
    }

    #[test]
    fn test_dlp_scan_result_safe() {
        let result = DlpScanResult {
            file_path: "/clean.txt".into(),
            violations: vec![],
            safe: true,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["safe"], true);
        assert!(json["violations"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_create_dlp_policy_request_deserialization() {
        let json = r#"{
            "name": "Block Exe",
            "policy_type": "file_type",
            "enabled": true,
            "config": {"blocked_extensions": ["exe"]}
        }"#;
        let req: CreateDlpPolicyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Block Exe");
        assert_eq!(req.policy_type, "file_type");
        assert_eq!(req.enabled, Some(true));
    }

    #[test]
    fn test_update_dlp_policy_request_deserialization() {
        let json = r#"{"name": "New Name", "enabled": false}"#;
        let req: UpdateDlpPolicyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("New Name".into()));
        assert_eq!(req.enabled, Some(false));
        assert!(req.config.is_none());
    }

    // --- Policy type validation edge cases ---

    #[test]
    fn test_create_policy_empty_name() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("", "file_type", config);
        let policy = store.create_policy(&req).unwrap();
        assert_eq!(policy.name, "");
    }

    #[test]
    fn test_create_policy_special_chars() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Policy with <special> & chars!", "file_type", config);
        let policy = store.create_policy(&req).unwrap();
        assert_eq!(policy.name, "Policy with <special> & chars!");
    }

    #[test]
    fn test_policy_created_at_and_updated_at_same() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Time Check", "file_type", config);
        let policy = store.create_policy(&req).unwrap();
        assert_eq!(policy.created_at, policy.updated_at);
    }

    #[test]
    fn test_policy_updated_at_changes_on_update() {
        let db = init_db();
        let store = DlpStore::new().with_db(db);
        let config = serde_json::json!({});
        let req = make_request("Time Change", "file_type", config);
        let created = store.create_policy(&req).unwrap();

        let update = UpdateDlpPolicyRequest {
            name: Some("Updated".into()),
            enabled: None,
            config: None,
        };
        let updated = store.update_policy(&created.id, &update).unwrap().unwrap();
        assert_ne!(created.created_at, updated.updated_at);
    }
}
