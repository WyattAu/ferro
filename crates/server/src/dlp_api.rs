//! Data Loss Prevention (DLP) API endpoints.
//!
//! Manages DLP policies (file type restrictions, content pattern detection,
//! file size limits, external share restrictions) and provides file scanning
//! against configured policies.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::db::DbHandle;

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
    "exe", "msi", "bat", "cmd", "com", "pif", "scr", "vbs", "vbe", "js", "jse",
    "ws", "wsh", "ps1", "psm1", "psd1", "reg", "dll", "sys", "cpl", "inf",
    "hta", "cda", "lnk", "application", "gadget",
];

/// Content patterns for detection (regex-based).
fn content_patterns() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("credit_card", r"\b(?:\d[ -]*?){13,16}\b", "Credit card number detected"),
        ("ssn", r"\b\d{3}-\d{2}-\d{4}\b", "Social Security Number detected"),
        ("email", r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "Email address detected"),
        ("phone", r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b", "Phone number detected"),
        ("ip_address", r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b", "IP address detected"),
    ]
}

/// Initialize the DLP policies table in SQLite.
pub fn init_dlp_table(db: &DbHandle) -> Result<(), rusqlite::Error> {
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
    )?;
    Ok(())
}

/// Load all policies from the database.
fn load_policies(db: &DbHandle) -> Vec<DlpPolicy> {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            let c = e.into_inner();
            c
        }
    };
    let mut stmt = match conn.prepare(
        "SELECT id, name, policy_type, enabled, config, created_at, updated_at FROM dlp_policies ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| {
        let enabled: i32 = row.get(3)?;
        let config_str: String = row.get(4)?;
        let config: serde_json::Value =
            serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null);
        Ok(DlpPolicy {
            id: row.get(0)?,
            name: row.get(1)?,
            policy_type: row.get(2)?,
            enabled: enabled != 0,
            config,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    });
    match rows {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// GET /api/dlp/policies — List DLP policies.
pub async fn list_policies(State(state): State<AppState>) -> Response {
    let policies = match &state.db {
        Some(db) => load_policies(db),
        None => Vec::new(),
    };

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
pub async fn create_policy(
    State(state): State<AppState>,
    Json(req): Json<CreateDlpPolicyRequest>,
) -> Response {
    let policy_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let enabled = req.enabled.unwrap_or(true);
    let config_str = match serde_json::to_string(&req.config) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid config: {}", e) })),
            )
                .into_response();
        }
    };

    let valid_types = ["file_type", "content_pattern", "file_size", "external_share"];
    if !valid_types.contains(&req.policy_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Invalid policy_type. Must be one of: {:?}", valid_types),
            })),
        )
            .into_response();
    }

    let policy = DlpPolicy {
        id: policy_id.clone(),
        name: req.name,
        policy_type: req.policy_type,
        enabled,
        config: req.config,
        created_at: now.clone(),
        updated_at: now,
    };

    if let Some(db) = &state.db {
        let conn = match db.lock() {
            Ok(c) => c,
            Err(e) => {
                let c = e.into_inner();
                c
            }
        };
        if let Err(e) = conn.execute(
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
        ) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to create policy: {}", e) })),
            )
                .into_response();
        }
    }

    (
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
        .into_response()
}

/// PUT /api/dlp/policies/{id} — Update a DLP policy.
pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDlpPolicyRequest>,
) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "Database not configured" })),
            )
                .into_response();
        }
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            let c = e.into_inner();
            c
        }
    };

    // Check policy exists
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM dlp_policies WHERE id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Policy not found" })),
        )
            .into_response();
    }

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(name) = &req.name {
        let _ = conn.execute(
            "UPDATE dlp_policies SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, now, id],
        );
    }

    if let Some(enabled) = req.enabled {
        let _ = conn.execute(
            "UPDATE dlp_policies SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled as i32, now, id],
        );
    }

    if let Some(config) = &req.config {
        if let Ok(config_str) = serde_json::to_string(config) {
            let _ = conn.execute(
                "UPDATE dlp_policies SET config = ?1, updated_at = ?2 WHERE id = ?3",
                params![config_str, now, id],
            );
        }
    }

    // Reload and return
    let policy = {
        let mut stmt = match conn.prepare(
            "SELECT id, name, policy_type, enabled, config, created_at, updated_at FROM dlp_policies WHERE id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to reload policy" })),
                )
                    .into_response();
            }
        };
        let rows = stmt.query_map(params![id], |row| {
            let enabled: i32 = row.get(3)?;
            let config_str: String = row.get(4)?;
            let config: serde_json::Value =
                serde_json::from_str(&config_str).unwrap_or(serde_json::Value::Null);
            Ok(DlpPolicy {
                id: row.get(0)?,
                name: row.get(1)?,
                policy_type: row.get(2)?,
                enabled: enabled != 0,
                config,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        });
        match rows.and_then(|mut r| r.next().transpose()) {
            Ok(Some(p)) => p,
            _ => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to reload policy" })),
                )
                    .into_response();
            }
        }
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Policy updated",
            "policy": policy,
        })),
    )
        .into_response()
}

/// DELETE /api/dlp/policies/{id} — Delete a DLP policy.
pub async fn delete_policy(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let db = match &state.db {
        Some(db) => db,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "Database not configured" })),
            )
                .into_response();
        }
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            let c = e.into_inner();
            c
        }
    };

    let affected = conn
        .execute("DELETE FROM dlp_policies WHERE id = ?1", params![id])
        .unwrap_or(0);

    if affected == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Policy not found" })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Policy deleted",
            "id": id,
        })),
    )
        .into_response()
}

/// POST /api/dlp/scan/{path} — Scan file against DLP policies.
pub async fn scan_file_dlp(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
) -> Response {
    let content = match state.storage.get(&file_path).await {
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

    let policies = match &state.db {
        Some(db) => load_policies(db),
        None => Vec::new(),
    };

    let mut violations = Vec::new();

    for policy in &policies {
        if !policy.enabled {
            continue;
        }

        match policy.policy_type.as_str() {
            "file_type" => {
                if let Some(blocked) = policy.config.get("blocked_extensions") {
                    if let Some(exts) = blocked.as_array() {
                    if let Some(dot_pos) = file_path.rfind('.') {
                        let ext = &file_path[dot_pos + 1..].to_lowercase();
                        for blocked_ext in exts {
                            if let Some(bext) = blocked_ext.as_str() {
                                if *ext == bext.to_lowercase() {
                                        violations.push(DlpViolation {
                                            policy_id: policy.id.clone(),
                                            policy_name: policy.name.clone(),
                                            policy_type: policy.policy_type.clone(),
                                            description: format!(
                                                "Blocked file extension: .{}",
                                                ext
                                            ),
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
                        }
                    }
                }
                // Default: block executables if no specific config
                if policy.config.get("blocked_extensions").is_none() {
                    if let Some(dot_pos) = file_path.rfind('.') {
                        let ext = &file_path[dot_pos + 1..].to_lowercase();
                        if BLOCKED_EXECUTABLE_EXTENSIONS.contains(&ext.as_str()) {
                            violations.push(DlpViolation {
                                policy_id: policy.id.clone(),
                                policy_name: policy.name.clone(),
                                policy_type: policy.policy_type.clone(),
                                description: format!(
                                    "Executable file blocked: .{}",
                                    ext
                                ),
                                severity: "high".to_string(),
                            });
                        }
                    }
                }
            }
            "content_pattern" => {
                let content_str = String::from_utf8_lossy(&content);
                let patterns = content_patterns();
                let patterns_to_check = if let Some(check_patterns) =
                    policy.config.get("patterns").and_then(|p| p.as_array())
                {
                    check_patterns
                        .iter()
                        .filter_map(|p| p.as_str())
                        .collect::<Vec<_>>()
                } else {
                    patterns.iter().map(|(name, _, _)| *name).collect()
                };

                for (name, regex, description) in &patterns {
                    if !patterns_to_check.contains(name) {
                        continue;
                    }
                    if let Ok(re) = regex::Regex::new(regex) {
                        if re.is_match(&content_str) {
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
            }
            "file_size" => {
                if let Some(max_size) = policy
                    .config
                    .get("max_size_bytes")
                    .and_then(|s| s.as_u64())
                {
                    if content.len() as u64 > max_size {
                        violations.push(DlpViolation {
                            policy_id: policy.id.clone(),
                            policy_name: policy.name.clone(),
                            policy_type: policy.policy_type.clone(),
                            description: format!(
                                "File size {} exceeds limit of {} bytes",
                                content.len(),
                                max_size
                            ),
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
            "external_share" => {
                // Check if file is shared externally (placeholder logic)
                if let Some(deny_external) =
                    policy.config.get("deny_external").and_then(|s| s.as_bool())
                {
                    if deny_external {
                        // In a real implementation, check if file is in a shared context
                        // For now, flag files that look like they might be shared
                    }
                }
            }
            _ => {}
        }
    }

    // Record violations as alerts
    if !violations.is_empty() {
        if let Some(db) = &state.db {
            let conn = match db.lock() {
                Ok(c) => c,
                Err(e) => {
                    let c = e.into_inner();
                    c
                }
            };
            let now = chrono::Utc::now().to_rfc3339();
            for violation in &violations {
                let alert_id = uuid::Uuid::new_v4().to_string();
                let _ = conn.execute(
                    "INSERT INTO dlp_alerts (id, policy_id, policy_name, file_path, violation_type, details, severity, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        alert_id,
                        violation.policy_id,
                        violation.policy_name,
                        file_path,
                        violation.policy_type,
                        violation.description,
                        violation.severity,
                        now,
                    ],
                );
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
pub async fn list_alerts(State(state): State<AppState>) -> Response {
    let alerts = match &state.db {
        Some(db) => {
            let conn = match db.lock() {
                Ok(c) => c,
                Err(e) => {
                    let c = e.into_inner();
                    c
                }
            };
            let mut stmt = match conn.prepare(
                "SELECT id, policy_id, policy_name, file_path, violation_type, details, severity, created_at FROM dlp_alerts ORDER BY created_at DESC LIMIT 100",
            ) {
                Ok(s) => s,
                Err(_) => {
                    return (
                        StatusCode::OK,
                        Json(serde_json::json!({ "alerts": [], "total": 0 })),
                    )
                        .into_response();
                }
            };
            let rows = stmt.query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "policy_id": row.get::<_, String>(1)?,
                    "policy_name": row.get::<_, String>(2)?,
                    "file_path": row.get::<_, String>(3)?,
                    "violation_type": row.get::<_, String>(4)?,
                    "details": row.get::<_, String>(5)?,
                    "severity": row.get::<_, String>(6)?,
                    "created_at": row.get::<_, String>(7)?,
                }))
            });
            match rows {
                Ok(mapped) => mapped.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                Err(_) => Vec::new(),
            }
        }
        None => Vec::new(),
    };

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
    use crate::AppState;

    #[tokio::test]
    async fn test_list_policies_no_db() {
        let state = AppState::in_memory();
        let response = list_policies(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_policy_no_db() {
        let state = AppState::in_memory();
        let response = create_policy(
            State(state),
            Json(CreateDlpPolicyRequest {
                name: "Test Policy".to_string(),
                policy_type: "file_type".to_string(),
                enabled: None,
                config: serde_json::json!({}),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_scan_file_not_found() {
        let state = AppState::in_memory();
        let response = scan_file_dlp(State(state), Path("/missing.txt".to_string())).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_alerts_no_db() {
        let state = AppState::in_memory();
        let response = list_alerts(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
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
}
