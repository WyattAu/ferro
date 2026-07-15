use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::{ApiError, UserMgmtState};

/// Response for wipe status check.
#[derive(Debug, Serialize)]
pub struct WipeStatusResponse {
    pub wipe_pending: bool,
    pub wipe_message: Option<String>,
}

/// Response for wipe confirmation.
#[derive(Debug, Serialize)]
pub struct WipeConfirmResponse {
    pub success: bool,
    pub message: String,
}

/// Request body for wipe confirmation.
#[derive(Debug, Deserialize)]
pub struct WipeConfirmRequest {
    pub device_id: Option<String>,
}

/// GET /api/wipe-status
///
/// Client checks if a remote wipe is pending for the authenticated user.
/// The client should poll this endpoint periodically or check on startup.
pub async fn get_wipe_status<S: UserMgmtState>(State(state): State<S>) -> Response {
    let username = state.admin_user().as_deref().unwrap_or("anonymous");

    let user = match state.user_store().get_user_by_username(username).await {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::OK,
                axum::Json(WipeStatusResponse {
                    wipe_pending: false,
                    wipe_message: None,
                }),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        axum::Json(WipeStatusResponse {
            wipe_pending: user.wipe_pending,
            wipe_message: None,
        }),
    )
        .into_response()
}

/// POST /api/wipe-confirm
///
/// Client confirms that it has completed the wipe (deleted local data).
/// After confirmation, the server clears the wipe_pending flag.
pub async fn confirm_wipe<S: UserMgmtState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<WipeConfirmRequest>,
) -> Response {
    let username = state.admin_user().as_deref().unwrap_or("anonymous");

    let user = match state.user_store().get_user_by_username(username).await {
        Ok(u) => u,
        Err(_) => {
            return ApiError::not_found("USER_NOT_FOUND", "User not found");
        }
    };

    // Clear the wipe pending flag
    if let Err(e) = state.user_store().set_wipe_pending(&user.id, false).await {
        return ApiError::internal("WIPE_CONFIRM_ERROR", format!("Failed to clear wipe status: {:?}", e));
    }

    tracing::info!(
        user_id = %user.id,
        device_id = ?body.device_id,
        "Wipe confirmed by client"
    );

    (
        StatusCode::OK,
        axum::Json(WipeConfirmResponse {
            success: true,
            message: "Wipe confirmed successfully".to_string(),
        }),
    )
        .into_response()
}
