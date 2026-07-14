use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AdminState;

/// A single activity feed entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub action: String,
    pub path: String,
    pub size: Option<u64>,
    pub timestamp: String,
    pub user: String,
}

/// Paginated activity feed response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityResponse {
    pub entries: Vec<ActivityEntry>,
    pub total: usize,
}

/// Query parameters for the activity feed.
#[derive(Debug, Deserialize)]
pub struct ActivityParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// GET /api/activity — return recent activity from the audit log.
pub async fn get_activity<S: AdminState>(State(state): State<S>, Query(params): Query<ActivityParams>) -> Response {
    let limit = params.limit.unwrap_or(50) as usize;
    let offset = params.offset.unwrap_or(0) as usize;

    let audit_entries = state.audit_log().entries().await;
    let total = audit_entries.len();

    let start = offset.min(total);
    let end = (offset + limit).min(total);
    let page = &audit_entries[start..end];

    let entries: Vec<ActivityEntry> = page
        .iter()
        .rev()
        .map(|e| {
            let action = classify_action(&e.method, e.status);
            ActivityEntry {
                action,
                path: e.path.clone(),
                size: e.content_length,
                timestamp: e.timestamp.clone(),
                user: e.user.clone(),
            }
        })
        .collect();

    (StatusCode::OK, axum::Json(ActivityResponse { entries, total })).into_response()
}

fn classify_action(method: &str, status: u16) -> String {
    match (method, status) {
        ("PUT", 200..=299) => "upload".to_string(),
        ("DELETE", 200..=299) => "delete".to_string(),
        ("MKCOL", 200..=299) => "create_folder".to_string(),
        ("COPY", 200..=299) => "copy".to_string(),
        ("MOVE", 200..=299) => "move".to_string(),
        ("POST", 200..=299) | ("GET", 200..=299) => "access".to_string(),
        _ => format!("{}_{}", method.to_lowercase(), status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_action_upload() {
        assert_eq!(classify_action("PUT", 201), "upload");
        assert_eq!(classify_action("PUT", 200), "upload");
    }

    #[test]
    fn test_classify_action_delete() {
        assert_eq!(classify_action("DELETE", 204), "delete");
    }

    #[test]
    fn test_classify_action_create_folder() {
        assert_eq!(classify_action("MKCOL", 201), "create_folder");
    }

    #[test]
    fn test_classify_action_copy() {
        assert_eq!(classify_action("COPY", 201), "copy");
    }

    #[test]
    fn test_classify_action_move() {
        assert_eq!(classify_action("MOVE", 201), "move");
    }

    #[test]
    fn test_classify_action_access() {
        assert_eq!(classify_action("GET", 200), "access");
        assert_eq!(classify_action("POST", 200), "access");
    }

    #[test]
    fn test_classify_action_error() {
        assert_eq!(classify_action("PUT", 500), "put_500");
        assert_eq!(classify_action("DELETE", 404), "delete_404");
    }
}
