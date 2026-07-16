use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AppState;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceSummary {
    pub total_policies: usize,
    pub active_retention_policies: usize,
    pub active_worm_policies: usize,
    pub active_dlp_policies: usize,
    pub total_dlp_alerts: usize,
    pub overall_status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataRetentionStatus {
    pub total_policies: usize,
    pub enabled_policies: usize,
    pub last_run: Option<String>,
    pub policies: Vec<RetentionPolicyInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RetentionPolicyInfo {
    pub id: String,
    pub name: String,
    pub path_prefix: String,
    pub max_age_seconds: u64,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WormStatus {
    pub total_policies: usize,
    pub enabled_policies: usize,
    pub policies: Vec<WormPolicyInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WormPolicyInfo {
    pub id: String,
    pub path_prefix: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DlpAlertSummary {
    pub total_alerts: usize,
    pub high_severity: usize,
    pub medium_severity: usize,
    pub low_severity: usize,
    pub recent_alerts: Vec<DlpAlertInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DlpAlertInfo {
    pub id: String,
    pub policy_name: String,
    pub file_path: String,
    pub violation_type: String,
    pub severity: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_events: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub unique_users: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: String,
    pub include_retention: Option<bool>,
    pub include_worm: Option<bool>,
    pub include_dlp: Option<bool>,
    pub include_audit: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub data: serde_json::Value,
    pub format: String,
    pub generated_at: String,
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/admin/compliance/summary — Overall compliance status.
pub async fn compliance_summary(State(_state): State<AppState>) -> Response {
    let retention_policies = get_retention_policies().await;
    let worm_policies = get_worm_policies().await;
    let dlp_policies = get_dlp_policies().await;
    let dlp_alerts = get_dlp_alerts().await;

    let total = retention_policies.len() + worm_policies.len() + dlp_policies.len();
    let active = retention_policies.iter().filter(|p| p.enabled).count()
        + worm_policies.iter().filter(|p| p.enabled).count()
        + dlp_policies.len(); // DLP policies are always considered active in stubs

    let status = if active > 0 {
        "compliant"
    } else if total > 0 {
        "partial"
    } else {
        "no_policies"
    };

    let summary = ComplianceSummary {
        total_policies: total,
        active_retention_policies: retention_policies.iter().filter(|p| p.enabled).count(),
        active_worm_policies: worm_policies.iter().filter(|p| p.enabled).count(),
        active_dlp_policies: dlp_policies.len(), // DLP policies are always active in stubs
        total_dlp_alerts: dlp_alerts.len(),
        overall_status: status.to_string(),
    };

    (StatusCode::OK, Json(summary)).into_response()
}

/// GET /api/v1/admin/compliance/data-retention — Retention policy status.
pub async fn data_retention_status(State(_state): State<AppState>) -> Response {
    let policies = get_retention_policies().await;
    let enabled = policies.iter().filter(|p| p.enabled).count();

    let status = DataRetentionStatus {
        total_policies: policies.len(),
        enabled_policies: enabled,
        last_run: None,
        policies: policies
            .into_iter()
            .map(|p| RetentionPolicyInfo {
                id: p.id,
                name: p.name,
                path_prefix: p.path_prefix,
                max_age_seconds: p.max_age_seconds,
                enabled: p.enabled,
            })
            .collect(),
    };

    (StatusCode::OK, Json(status)).into_response()
}

/// GET /api/v1/admin/compliance/worm — WORM policy status.
pub async fn worm_status(State(_state): State<AppState>) -> Response {
    let policies = get_worm_policies().await;
    let enabled = policies.iter().filter(|p| p.enabled).count();

    let status = WormStatus {
        total_policies: policies.len(),
        enabled_policies: enabled,
        policies: policies
            .into_iter()
            .map(|p| WormPolicyInfo {
                id: p.id,
                path_prefix: p.path_prefix,
                enabled: p.enabled,
            })
            .collect(),
    };

    (StatusCode::OK, Json(status)).into_response()
}

/// GET /api/v1/admin/compliance/dlp — DLP alert summary.
pub async fn dlp_summary(State(_state): State<AppState>) -> Response {
    let alerts = get_dlp_alerts().await;
    let high = alerts.iter().filter(|a| a.severity == "high").count();
    let medium = alerts.iter().filter(|a| a.severity == "medium").count();
    let low = alerts.iter().filter(|a| a.severity == "low").count();

    let summary = DlpAlertSummary {
        total_alerts: alerts.len(),
        high_severity: high,
        medium_severity: medium,
        low_severity: low,
        recent_alerts: alerts
            .into_iter()
            .take(10)
            .map(|a| DlpAlertInfo {
                id: a.id,
                policy_name: a.policy_name,
                file_path: a.file_path,
                violation_type: a.violation_type,
                severity: a.severity,
                created_at: a.created_at,
            })
            .collect(),
    };

    (StatusCode::OK, Json(summary)).into_response()
}

/// GET /api/v1/admin/compliance/audit-summary — Audit log statistics.
pub async fn audit_summary(State(_state): State<AppState>) -> Response {
    let summary = AuditSummary {
        total_events: 0,
        successful_requests: 0,
        failed_requests: 0,
        unique_users: 0,
    };

    (StatusCode::OK, Json(summary)).into_response()
}

/// POST /api/v1/admin/compliance/export — Export compliance report.
pub async fn export_compliance_report(
    State(_state): State<AppState>,
    Json(req): Json<ExportRequest>,
) -> Response {
    let format = req.format.clone();
    let mut data = serde_json::json!({});

    if req.include_retention.unwrap_or(true) {
        let policies = get_retention_policies().await;
        data["retention"] = serde_json::json!({ "policies": policies });
    }

    if req.include_worm.unwrap_or(true) {
        let policies = get_worm_policies().await;
        data["worm"] = serde_json::json!({ "policies": policies });
    }

    if req.include_dlp.unwrap_or(true) {
        let alerts = get_dlp_alerts().await;
        data["dlp"] = serde_json::json!({ "alerts": alerts });
    }

    if req.include_audit.unwrap_or(true) {
        data["audit"] = serde_json::json!({ "summary": "Audit data not available" });
    }

    let response = ExportResponse {
        data,
        format: format.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

// ---------------------------------------------------------------------------
// Helper functions (stubs - in production these would query the actual stores)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct RetentionPolicyStub {
    id: String,
    name: String,
    path_prefix: String,
    max_age_seconds: u64,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct WormPolicyStub {
    id: String,
    path_prefix: String,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DlpAlertStub {
    id: String,
    policy_name: String,
    file_path: String,
    violation_type: String,
    severity: String,
    created_at: String,
}

async fn get_retention_policies() -> Vec<RetentionPolicyStub> {
    vec![]
}

async fn get_worm_policies() -> Vec<WormPolicyStub> {
    vec![]
}

async fn get_dlp_policies() -> Vec<serde_json::Value> {
    vec![]
}

async fn get_dlp_alerts() -> Vec<DlpAlertStub> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_summary_serde() {
        let summary = ComplianceSummary {
            total_policies: 5,
            active_retention_policies: 2,
            active_worm_policies: 1,
            active_dlp_policies: 2,
            total_dlp_alerts: 10,
            overall_status: "compliant".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let parsed: ComplianceSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_policies, 5);
        assert_eq!(parsed.overall_status, "compliant");
    }

    #[test]
    fn test_data_retention_status_serde() {
        let status = DataRetentionStatus {
            total_policies: 3,
            enabled_policies: 2,
            last_run: Some("2025-01-01T00:00:00Z".to_string()),
            policies: vec![],
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: DataRetentionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_policies, 3);
    }

    #[test]
    fn test_worm_status_serde() {
        let status = WormStatus {
            total_policies: 2,
            enabled_policies: 1,
            policies: vec![],
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: WormStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_policies, 2);
    }

    #[test]
    fn test_dlp_alert_summary_serde() {
        let summary = DlpAlertSummary {
            total_alerts: 15,
            high_severity: 3,
            medium_severity: 7,
            low_severity: 5,
            recent_alerts: vec![],
        };
        let json = serde_json::to_string(&summary).unwrap();
        let parsed: DlpAlertSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_alerts, 15);
        assert_eq!(parsed.high_severity, 3);
    }

    #[test]
    fn test_audit_summary_serde() {
        let summary = AuditSummary {
            total_events: 1000,
            successful_requests: 950,
            failed_requests: 50,
            unique_users: 25,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let parsed: AuditSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_events, 1000);
    }

    #[test]
    fn test_export_request_serde() {
        let req = ExportRequest {
            format: "json".to_string(),
            include_retention: Some(true),
            include_worm: Some(true),
            include_dlp: Some(false),
            include_audit: Some(true),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: ExportRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.format, "json");
        assert!(parsed.include_dlp == Some(false));
    }
}
