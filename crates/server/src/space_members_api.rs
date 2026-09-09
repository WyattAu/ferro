//! Admin API for the Cedar space-members file.
//!
//! GET /api/admin/space-members — current memberships
//! PUT /api/admin/space-members — replace memberships and hot-reload the
//! generated Cedar policy set (no restart needed).

use crate::state::AppState;
use ferro_auth::cedar::{generate_auto_policies, SpaceMembership};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

fn members_path(state: &AppState) -> Option<std::path::PathBuf> {
    state
        .space_members_file
        .as_ref()
        .filter(|p| !p.as_os_str().is_empty())
        .cloned()
}

pub async fn get_space_members(State(state): State<AppState>) -> Response {
    let Some(path) = members_path(&state) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": "NOT_CONFIGURED",
                "message": "FERRO_SPACE_MEMBERS_FILE is not set",
            })),
        )
            .into_response();
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (StatusCode::OK, axum::Json(Vec::<SpaceMembership>::new())).into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": format!("read failed: {e}") })),
            )
                .into_response();
        }
    };

    match serde_json::from_str::<Vec<SpaceMembership>>(&text) {
        Ok(members) => (StatusCode::OK, axum::Json(members)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": format!("members file corrupt: {e}") })),
        )
            .into_response(),
    }
}

pub async fn put_space_members(
    State(state): State<AppState>,
    axum::Json(members): axum::Json<Vec<SpaceMembership>>,
) -> Response {
    let Some(path) = members_path(&state) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": "NOT_CONFIGURED",
                "message": "FERRO_SPACE_MEMBERS_FILE is not set",
            })),
        )
            .into_response();
    };
    let Some(admin_sub) = state.admin_sub.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": "NOT_CONFIGURED",
                "message": "FERRO_ADMIN_SUB is not set (policies cannot be regenerated)",
            })),
        )
            .into_response();
    };
    let Some(authorizer) = state.cedar.clone() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({ "error": "Cedar authorizer not configured" })),
        )
            .into_response();
    };

    // Reject duplicate spaces up front — they'd silently shadow each other.
    let mut seen = std::collections::HashSet::new();
    for m in &members {
        if m.space.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": "space name required" })),
            )
                .into_response();
        }
        if !seen.insert(m.space.clone()) {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": format!("duplicate space: {}", m.space) })),
            )
                .into_response();
        }
    }

    // In-place write: the members file is typically a SINGLE-FILE bind
    // mount, where tmp+rename fails (rename over the mountpoint inode is
    // EBUSY, and :ro mounts reject writes entirely). Compose mounts it :rw.
    let serialized = match serde_json::to_string_pretty(&members) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": format!("serialize failed: {e}") })),
            )
                .into_response();
        }
    };
    if let Err(e) = std::fs::write(&path, serialized) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": format!("write failed (is the mount :ro?): {e}"),
            })),
        )
            .into_response();
    }

    // Hot-reload the generated policy set.
    let policies = generate_auto_policies(&admin_sub, &members);
    match authorizer.load_policies(&[policies.join("\n\n")]).await {
        Ok(()) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "ok": true,
                "spaces": members.len(),
                "policies": policies.len(),
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": format!("members saved but policy reload FAILED (restart required): {e}"),
                "saved": true,
            })),
        )
            .into_response(),
    }
}
