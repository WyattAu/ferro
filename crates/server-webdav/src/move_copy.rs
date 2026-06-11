use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::path::normalize_path;
use serde::Deserialize;

use crate::WebdavAppState;
use crate::WebdavFileEvent;

#[derive(Debug, Deserialize)]
pub struct MoveCopyRequest {
    pub source: String,
    pub destination: String,
}

fn bad_request(code: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        })),
    )
        .into_response()
}

fn not_found(code: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        })),
    )
        .into_response()
}

fn conflict(code: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::CONFLICT,
        axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        })),
    )
        .into_response()
}

fn forbidden(code: &str, message: impl Into<String>) -> Response {
    (
        StatusCode::FORBIDDEN,
        axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        })),
    )
        .into_response()
}

fn internal(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": "internal_error",
        })),
    )
        .into_response()
}

pub async fn move_file<S: WebdavAppState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<MoveCopyRequest>,
) -> Response {
    let source = normalize_path(&body.source);
    let destination = normalize_path(&body.destination);

    if source.is_empty() || destination.is_empty() {
        return bad_request("path_invalid", "Source and destination must be non-empty");
    }

    if source == destination {
        return bad_request("bad_request", "Source and destination are the same");
    }

    if let Err(e) = state.lock_manager().check_lock_for_write(&source).await {
        return (
            StatusCode::LOCKED,
            axum::Json(serde_json::json!({
                "error": "Locked",
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }
    if let Err(e) = state
        .lock_manager()
        .check_lock_for_write(&destination)
        .await
    {
        return (
            StatusCode::LOCKED,
            axum::Json(serde_json::json!({
                "error": "Locked",
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }

    let _lock = state
        .lock_manager()
        .acquire_lock(
            &source,
            "system",
            common::webdav::LockScope::Exclusive,
            common::webdav::LockDepth::Zero,
            Some(10),
        )
        .await;
    let _lock2 = state
        .lock_manager()
        .acquire_lock(
            &destination,
            "system",
            common::webdav::LockScope::Exclusive,
            common::webdav::LockDepth::Zero,
            Some(10),
        )
        .await;

    match state.storage().head(&source).await {
        Ok(meta) => {
            if meta.is_collection {
                if let Err(e) = move_collection_recursive(&state, &source, &destination).await {
                    return internal(format!("Move failed: {}", e));
                }
            } else {
                if state.is_worm_protected(&source) {
                    return forbidden("worm_protected", format!("WORM-protected: {}", source));
                }
                match state.storage().move_path(&source, &destination).await {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("not found") || msg.contains("NotFound") {
                            return not_found(
                                "file_not_found",
                                format!("Source not found: {}", source),
                            );
                        }
                        return internal(format!("Move failed: {}", e));
                    }
                }
            }
            state
                .dispatch_post_op(WebdavFileEvent {
                    op_type: "move",
                    path: format!("/api/files/move {} -> {}", source, destination),
                    new_path: None,
                    size: None,
                    mime_type: None,
                    owner: "admin".to_string(),
                    etag: None,
                    already_existed: false,
                })
                .await;
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({"status": "ok"})),
            )
                .into_response()
        }
        Err(_) => not_found("file_not_found", format!("Source not found: {}", source)),
    }
}

pub async fn copy_file<S: WebdavAppState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<MoveCopyRequest>,
) -> Response {
    let source = normalize_path(&body.source);
    let destination = normalize_path(&body.destination);

    if source.is_empty() || destination.is_empty() {
        return bad_request("path_invalid", "Source and destination must be non-empty");
    }

    if source == destination {
        return bad_request("bad_request", "Source and destination are the same");
    }

    if let Err(e) = state.lock_manager().check_lock_for_write(&source).await {
        return (
            StatusCode::LOCKED,
            axum::Json(serde_json::json!({
                "error": "Locked",
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }
    if let Err(e) = state
        .lock_manager()
        .check_lock_for_write(&destination)
        .await
    {
        return (
            StatusCode::LOCKED,
            axum::Json(serde_json::json!({
                "error": "Locked",
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }

    if state.storage().exists(&destination).await.unwrap_or(false) {
        return conflict(
            "file_exists",
            format!("Destination already exists: {}", destination),
        );
    }

    let _lock = state
        .lock_manager()
        .acquire_lock(
            &source,
            "system",
            common::webdav::LockScope::Exclusive,
            common::webdav::LockDepth::Zero,
            Some(10),
        )
        .await;
    let _lock2 = state
        .lock_manager()
        .acquire_lock(
            &destination,
            "system",
            common::webdav::LockScope::Exclusive,
            common::webdav::LockDepth::Zero,
            Some(10),
        )
        .await;

    match state.storage().head(&source).await {
        Ok(meta) => {
            if meta.is_collection {
                if let Err(e) = copy_collection_recursive(&state, &source, &destination).await {
                    return internal(format!("Copy failed: {}", e));
                }
            } else {
                if state.is_worm_protected(&source) {
                    return forbidden("worm_protected", format!("WORM-protected: {}", source));
                }
                match state.storage().copy(&source, &destination).await {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("not found") || msg.contains("NotFound") {
                            return not_found(
                                "file_not_found",
                                format!("Source not found: {}", source),
                            );
                        }
                        return internal(format!("Copy failed: {}", e));
                    }
                }
            }
            state
                .dispatch_post_op(WebdavFileEvent {
                    op_type: "copy",
                    path: format!("/api/files/copy {} -> {}", source, destination),
                    new_path: None,
                    size: None,
                    mime_type: None,
                    owner: "admin".to_string(),
                    etag: None,
                    already_existed: false,
                })
                .await;
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({"status": "ok"})),
            )
                .into_response()
        }
        Err(_) => not_found("file_not_found", format!("Source not found: {}", source)),
    }
}

async fn move_collection_recursive<S: WebdavAppState>(
    state: &S,
    source: &str,
    destination: &str,
) -> anyhow::Result<()> {
    let children = state.storage().list(source).await?;

    if !state.storage().exists(destination).await? {
        state
            .storage()
            .create_collection(destination, "admin")
            .await?;
    }

    for child in &children {
        let child_name = child.path.rsplit('/').next().unwrap_or("");
        let new_path = if destination == "/" {
            format!("/{}", child_name)
        } else {
            format!("{}/{}", destination, child_name)
        };

        state.storage().move_path(&child.path, &new_path).await?;
    }

    state.storage().delete(source).await?;
    Ok(())
}

async fn copy_collection_recursive<S: WebdavAppState>(
    state: &S,
    source: &str,
    destination: &str,
) -> anyhow::Result<()> {
    let children = state.storage().list(source).await?;

    if !state.storage().exists(destination).await? {
        state
            .storage()
            .create_collection(destination, "admin")
            .await?;
    }

    for child in &children {
        let child_name = child.path.rsplit('/').next().unwrap_or("");
        let new_path = if destination == "/" {
            format!("/{}", child_name)
        } else {
            format!("{}/{}", destination, child_name)
        };

        if child.is_collection {
            Box::pin(copy_collection_recursive(state, &child.path, &new_path)).await?;
        } else {
            state.storage().copy(&child.path, &new_path).await?;
        }
    }

    Ok(())
}
