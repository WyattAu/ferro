use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use common::auth::Claims;
use tracing::instrument;

use crate::AppState;
use crate::api_error::ApiError;
use ferro_server_state::ServerState as _;

pub use ferro_server_api::{
    AuthInfoResponse, CallbackParams, CopyMoveResponse, FileEntryJson, ListFilesParams, ListFilesResponse,
    LoginParams, MkdirResponse, PutFileResponse, RefreshTokenRequest, auth_callback_impl, auth_change_password_impl,
    auth_info_impl, auth_login_impl, auth_refresh_token_impl, copy_file_impl, get_file_impl, list_files_impl,
    mkdir_impl, move_file_rest_impl, normalize_api_path,
};

/// GET /api/auth/info — return current user info from OIDC claims.
#[utoipa::path(
    get,
    path = "/api/auth/info",
    responses(
        (status = 200, description = "Auth info", body = AuthInfoResponse),
    ),
    tags = ["auth"],
)]
#[instrument(name = "auth_info", skip(state, claims))]
pub async fn auth_info(claims: Option<axum::Extension<Claims>>, State(state): State<AppState>) -> Response {
    auth_info_impl(&state, claims).await
}

/// GET /api/auth/login — redirect to OIDC provider with PKCE.
#[utoipa::path(
    get,
    path = "/api/auth/login",
    params(LoginParams),
    responses(
        (status = 200, description = "Authorization URL for OIDC redirect"),
        (status = 503, description = "OIDC not configured", body = ApiError),
    ),
    tags = ["auth"],
)]
pub async fn auth_login(State(state): State<AppState>, Query(params): Query<LoginParams>) -> Response {
    auth_login_impl(&state, params).await
}

/// POST /api/auth/change-password — change admin password.
pub async fn auth_change_password(State(state): State<AppState>, req: axum::extract::Request) -> Response {
    auth_change_password_impl(&state, req).await
}

/// GET /api/auth/callback — handle OIDC callback.
#[utoipa::path(
    get,
    path = "/api/auth/callback",
    params(CallbackParams),
    responses(
        (status = 200, description = "Token exchange result"),
        (status = 503, description = "OIDC not configured", body = ApiError),
    ),
    tags = ["auth"],
)]
pub async fn auth_callback(State(state): State<AppState>, Query(params): Query<CallbackParams>) -> Response {
    auth_callback_impl(&state, params).await
}

/// POST /api/auth/refresh — exchange a refresh token for a new access token.
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "New access token"),
        (status = 401, description = "Invalid or expired refresh token"),
        (status = 503, description = "OIDC not configured"),
    ),
    tags = ["auth"],
)]
pub async fn auth_refresh_token(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<RefreshTokenRequest>,
) -> Response {
    auth_refresh_token_impl(&state, body).await
}

/// GET /api/v1/files — JSON file listing (alternative to WebDAV PROPFIND).
#[utoipa::path(
    get,
    path = "/api/v1/files",
    params(ListFilesParams),
    responses(
        (status = 200, description = "File listing", body = ListFilesResponse),
        (status = 409, description = "Not a collection", body = ApiError),
        (status = 404, description = "Path not found", body = ApiError),
        (status = 500, description = "List failed", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "list_files", skip(state, params))]
pub async fn list_files(State(state): State<AppState>, Query(params): Query<ListFilesParams>) -> Response {
    list_files_impl(&state, &params).await
}

/// GET /api/v1/files/{path} — download file content or get collection metadata.
#[utoipa::path(
    get,
    path = "/api/v1/files/{path}",
    responses(
        (status = 200, description = "File content or collection metadata", body = FileEntryJson),
        (status = 304, description = "Not modified"),
        (status = 404, description = "Not found", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "get_file", skip(state, headers), fields(path = %path))]
pub async fn get_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    get_file_impl(&state, path, headers).await
}

/// PUT /api/v1/files/{path} — upload/replace file content.
#[utoipa::path(
    put,
    path = "/api/v1/files/{path}",
    request_body(content = [u8], description = "Raw file content (binary)"),
    responses(
        (status = 201, description = "File created/updated", body = PutFileResponse),
        (status = 409, description = "Precondition failed", body = ApiError),
        (status = 500, description = "Upload failed", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "put_file", skip(state, headers, body), fields(path = %path))]
pub async fn put_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let path = match normalize_api_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    if let Err(reason) = crate::security::validate_path(&path) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "invalid_path",
                "message": reason,
            })),
        )
            .into_response();
    }

    if let Some(declared) = headers.get("content-type").and_then(|v| v.to_str().ok())
        && let Some(detected) = crate::security::verify_content_type(declared, &body)
    {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "content_type_mismatch",
                "message": format!(
                    "Declared Content-Type '{}' does not match detected type '{}'",
                    declared, detected
                ),
            })),
        )
            .into_response();
    }

    #[allow(clippy::collapsible_if)]
    if let Some(if_match) = headers.get("if-match").and_then(|v| v.to_str().ok()) {
        if let Ok(existing) = state.storage().head(&path).await {
            if if_match != existing.etag && if_match != "*" {
                return (
                    StatusCode::PRECONDITION_FAILED,
                    axum::Json(serde_json::json!({
                        "error": "precondition_failed",
                        "message": "ETag does not match",
                        "current_etag": existing.etag,
                    })),
                )
                    .into_response();
            }
        }
    }

    let owner = "anonymous".to_string();
    match state.storage().put(&path, body.clone(), &owner).await {
        Ok(meta) => {
            let etag = meta.etag.clone();
            let size = meta.size;
            let mime_type = meta.mime_type.clone();
            crate::events::dispatch_post_op(
                &state,
                crate::events::FileEvent {
                    op_type: "put",
                    path: path.clone(),
                    new_path: None,
                    size: Some(size),
                    mime_type: Some(mime_type),
                    owner: owner.clone(),
                    etag: Some(etag.clone()),
                    already_existed: false,
                },
            )
            .await;
            (
                StatusCode::CREATED,
                [
                    (axum::http::header::ETAG, etag.clone()),
                    (axum::http::header::LOCATION, path.clone()),
                ],
                axum::Json(PutFileResponse {
                    path: meta.path,
                    size,
                    etag,
                    content_hash: meta.content_hash.as_str().to_string(),
                    created_at: meta.created_at.to_rfc3339(),
                    modified_at: meta.modified_at.to_rfc3339(),
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "upload_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/files/{path} — delete a file or collection.
#[utoipa::path(
    delete,
    path = "/api/v1/files/{path}",
    responses(
        (status = 204, description = "File deleted"),
        (status = 404, description = "Not found", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "delete_file", skip(state), fields(path = %path))]
pub async fn delete_file(State(state): State<AppState>, AxumPath(path): AxumPath<String>) -> Response {
    let path = match normalize_api_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };
    match state.storage().delete(&path).await {
        Ok(()) => {
            crate::events::dispatch_post_op(
                &state,
                crate::events::FileEvent {
                    op_type: "delete",
                    path: path.clone(),
                    new_path: None,
                    size: None,
                    mime_type: None,
                    owner: "anonymous".to_string(),
                    etag: None,
                    already_existed: true,
                },
            )
            .await;
            (StatusCode::NO_CONTENT, "").into_response()
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "delete_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /api/v1/files/mkdir — create a directory/collection.
#[utoipa::path(
    post,
    path = "/api/v1/files/mkdir",
    request_body(content = serde_json::Value, description = "JSON with 'path' field"),
    responses(
        (status = 201, description = "Directory created", body = MkdirResponse),
        (status = 409, description = "Already exists", body = ApiError),
        (status = 500, description = "Mkdir failed", body = ApiError),
    ),
    tags = ["files"],
)]
pub async fn mkdir(State(state): State<AppState>, body: axum::Json<serde_json::Value>) -> Response {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    mkdir_impl(&state, path).await
}

/// Handler for `/api/v1/files/{*path}` — dispatches GET/PUT/DELETE.
pub async fn files_content_handler(
    method: axum::http::Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Option<AxumPath<String>>,
    body: axum::body::Bytes,
) -> Response {
    let file_path = match path {
        Some(AxumPath(p)) => p,
        None => {
            let path_str = uri.path();
            match path_str
                .strip_prefix("/api/v1/files/")
                .or_else(|| path_str.strip_prefix("/api/files/"))
            {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => {
                    return (
                        StatusCode::NOT_FOUND,
                        axum::Json(serde_json::json!({
                            "error": "not_found",
                            "message": "Unknown API endpoint",
                        })),
                    )
                        .into_response();
                }
            }
        }
    };

    match method {
        axum::http::Method::GET => get_file(State(state), AxumPath(file_path), headers).await,
        axum::http::Method::PUT => put_file(State(state), AxumPath(file_path), headers, body).await,
        axum::http::Method::DELETE => delete_file(State(state), AxumPath(file_path)).await,
        _ => (
            StatusCode::METHOD_NOT_ALLOWED,
            axum::Json(serde_json::json!({
                "error": "method_not_allowed",
                "message": "Only GET, PUT, and DELETE are supported for file operations",
            })),
        )
            .into_response(),
    }
}

/// POST /api/v1/files/copy — copy a file or directory.
#[utoipa::path(
    post,
    path = "/api/v1/files/copy",
    request_body(content = serde_json::Value, description = "JSON with 'from' and 'to' fields"),
    responses(
        (status = 201, description = "File copied", body = CopyMoveResponse),
        (status = 400, description = "Missing parameters", body = ApiError),
        (status = 404, description = "Copy failed", body = ApiError),
    ),
    tags = ["files"],
)]
pub async fn copy_file(State(state): State<AppState>, body: axum::Json<serde_json::Value>) -> Response {
    let from = body.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = body.get("to").and_then(|v| v.as_str()).unwrap_or("");

    if from.is_empty() || to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "missing_params",
                "message": "Both 'from' and 'to' are required",
            })),
        )
            .into_response();
    }

    let from = match normalize_api_path(from) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };
    let to = match normalize_api_path(to) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    copy_file_impl(&state, &from, &to).await
}

/// POST /api/v1/files/move — move/rename a file or directory.
#[utoipa::path(
    post,
    path = "/api/v1/files/move",
    request_body(content = serde_json::Value, description = "JSON with 'from' and 'to' fields"),
    responses(
        (status = 201, description = "File moved", body = CopyMoveResponse),
        (status = 400, description = "Missing parameters", body = ApiError),
        (status = 404, description = "Move failed", body = ApiError),
    ),
    tags = ["files"],
)]
pub async fn move_file_rest(State(state): State<AppState>, body: axum::Json<serde_json::Value>) -> Response {
    let from = body.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = body.get("to").and_then(|v| v.as_str()).unwrap_or("");

    if from.is_empty() || to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "missing_params",
                "message": "Both 'from' and 'to' are required",
            })),
        )
            .into_response();
    }

    let from = match normalize_api_path(from) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };
    let to = match normalize_api_path(to) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    move_file_rest_impl(&state, &from, &to).await
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use crate::AppState;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app_no_oidc() -> axum::Router {
        crate::build_router(AppState::in_memory())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_auth_login_without_oidc_returns_503() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/auth/login")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = body_json(response).await;
        assert_eq!(json["error"], "OIDC not configured");
    }

    #[tokio::test]
    async fn test_auth_callback_without_oidc_returns_503() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/auth/callback?code=test&state=invalid")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = body_json(response).await;
        assert_eq!(json["error"], "OIDC not configured");
    }

    #[tokio::test]
    async fn test_auth_info_returns_anonymous() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/auth/info")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["sub"], "anonymous");
        assert_eq!(json["iss"], "ferro");
        assert_eq!(json["aud"], "ferro");
    }

    #[tokio::test]
    async fn test_api_config_all_fields_present() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let expected_fields = [
            "version",
            "auth_enabled",
            "search_enabled",
            "wasm_enabled",
            "wasm_workers_enabled",
            "cedar_enabled",
            "metadata_persistent",
            "cas_enabled",
            "storage",
            "external_url",
            "wopi_configured",
        ];
        for field in &expected_fields {
            assert!(json.get(*field).is_some(), "Missing field: {}", field);
        }
    }

    #[tokio::test]
    async fn test_health_check_format() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/.well-known/ferro")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert!(json.get("version").is_some());
        assert!(json.get("uptime_seconds").is_some());
        assert!(json.get("subsystems").is_some());
        assert!(json["subsystems"].is_object());
        assert!(json["subsystems"].get("storage").is_some());
        assert!(json["subsystems"].get("auth").is_some());
        assert!(json["subsystems"].get("search").is_some());
        assert!(json["subsystems"].get("wasm").is_some());
        assert!(json["subsystems"].get("metadata").is_some());
        assert!(json["subsystems"].get("cas").is_some());
    }

    #[tokio::test]
    async fn test_metrics_endpoint_format() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.get("uptime_seconds").is_some());
        assert!(json.get("storage").is_some());
        assert!(json["storage"].is_object());
        assert!(json["storage"].get("files").is_some());
        assert!(json["storage"].get("total_bytes").is_some());
        assert!(json.get("requests").is_some());
        assert!(json["requests"].is_object());
    }

    #[tokio::test]
    async fn test_security_headers_present() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let headers = resp.headers();
        assert!(
            headers.get("X-Content-Type-Options").is_some(),
            "Missing X-Content-Type-Options header"
        );
        assert!(
            headers.get("X-Frame-Options").is_some(),
            "Missing X-Frame-Options header"
        );
        assert!(
            headers.get("Referrer-Policy").is_some(),
            "Missing Referrer-Policy header"
        );
    }

    #[tokio::test]
    async fn test_rest_put_returns_201_not_204() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/api/v1/files/test-dir/test-file.txt")
                    .header("content-type", "text/plain")
                    .body(axum::body::Body::from("test content"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["path"], "/test-dir/test-file.txt");
        assert_eq!(json["size"], 12);
    }
}
