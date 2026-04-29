use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::AppState;

const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];

/// Response returned after a successful WASM module upload.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub module_path: String,
    pub size: usize,
    pub filename: String,
}

/// Information about a single WASM module.
#[derive(Debug, Serialize)]
pub struct ModuleInfo {
    pub filename: String,
    pub module_path: String,
    pub size: u64,
    pub uploaded_at: String,
}

/// Response for listing all WASM modules.
#[derive(Debug, Serialize)]
pub struct ListModulesResponse {
    pub modules: Vec<ModuleInfo>,
}

/// Check that data starts with the WASM magic bytes.
pub fn validate_wasm_magic_bytes(data: &[u8]) -> bool {
    data.len() >= 4 && data[..4] == WASM_MAGIC
}

/// Validate a filename is a simple `.wasm` file name without path separators.
pub fn validate_filename(filename: &str) -> bool {
    let path = std::path::Path::new(filename);
    if path.extension().map(|e| e != "wasm").unwrap_or(true) {
        return false;
    }
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }
    if filename.contains('/') || filename.contains('\\') {
        return false;
    }
    filename == path.file_name().unwrap_or_default().to_str().unwrap_or("")
}

/// POST /api/workers/upload — upload a WASM module via multipart form.
pub async fn upload_wasm_module(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    let workers_dir = match &state.workers_dir {
        Some(dir) => dir.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({
                    "error": "WASM module storage not configured. Set --data-dir to enable uploads.",
                })),
            )
                .into_response();
        }
    };

    if let Err(e) = tokio::fs::create_dir_all(&workers_dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": format!("Failed to create workers directory: {}", e),
            })),
        )
            .into_response();
    }

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = match field.file_name() {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => continue,
        };

        if !validate_filename(&file_name) {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "Invalid filename. Only .wasm files without path separators are allowed.",
                })),
            )
                .into_response();
        }

        let data = match field.bytes().await {
            Ok(bytes) => bytes.to_vec(),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "error": format!("Failed to read file data: {}", e),
                    })),
                )
                    .into_response();
            }
        };

        if !validate_wasm_magic_bytes(&data) {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "Invalid WASM file: missing magic bytes (0x00 0x61 0x73 0x6D)",
                })),
            )
                .into_response();
        }

        let unique_name = format!("{}-{}", uuid::Uuid::new_v4(), file_name);
        let dest_path = workers_dir.join(&unique_name);

        match tokio::fs::write(&dest_path, &data).await {
            Ok(_) => {
                let body = UploadResponse {
                    module_path: dest_path.to_string_lossy().to_string(),
                    size: data.len(),
                    filename: unique_name,
                };
                return (StatusCode::CREATED, axum::Json(body)).into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": format!("Failed to write module: {}", e),
                    })),
                )
                    .into_response();
            }
        }
    }

    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({
            "error": "No file field found in upload. Use multipart form with a 'file' field.",
        })),
    )
        .into_response()
}

/// GET /api/workers/modules — list uploaded WASM modules.
pub async fn list_wasm_modules(State(state): State<AppState>) -> Response {
    let workers_dir = match &state.workers_dir {
        Some(dir) => dir.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({
                    "error": "WASM module storage not configured.",
                })),
            )
                .into_response();
        }
    };

    let mut modules = Vec::new();

    if let Ok(mut entries) = tokio::fs::read_dir(&workers_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                let modified = entry
                    .metadata()
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%dT%H:%M:%SZ")
                            .to_string()
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                modules.push(ModuleInfo {
                    filename: filename.clone(),
                    module_path: path.to_string_lossy().to_string(),
                    size,
                    uploaded_at: modified,
                });
            }
        }
    }

    modules.sort_by(|a, b| a.filename.cmp(&b.filename));

    (StatusCode::OK, axum::Json(ListModulesResponse { modules })).into_response()
}

/// DELETE /api/workers/modules/:filename — delete a WASM module.
pub async fn delete_wasm_module(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Response {
    if !validate_filename(&filename) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "Invalid filename.",
            })),
        )
            .into_response();
    }

    let workers_dir = match &state.workers_dir {
        Some(dir) => dir.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(serde_json::json!({
                    "error": "WASM module storage not configured.",
                })),
            )
                .into_response();
        }
    };

    let file_path = workers_dir.join(&filename);

    if !file_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": format!("Module '{}' not found.", filename),
            })),
        )
            .into_response();
    }

    match tokio::fs::remove_file(&file_path).await {
        Ok(_) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "status": "deleted",
                "filename": filename,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": format!("Failed to delete module: {}", e),
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[test]
    fn test_validate_wasm_magic_bytes() {
        assert!(validate_wasm_magic_bytes(&[0x00, 0x61, 0x73, 0x6D]));
        assert!(validate_wasm_magic_bytes(&[
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00
        ]));
        assert!(!validate_wasm_magic_bytes(&[0x00, 0x00, 0x00, 0x00]));
        assert!(!validate_wasm_magic_bytes(&[0x7F, 0x45, 0x4C, 0x46]));
        assert!(!validate_wasm_magic_bytes(&[]));
        assert!(!validate_wasm_magic_bytes(&[0x00, 0x61, 0x73]));
        assert!(!validate_wasm_magic_bytes(&[0x00, 0x61]));
    }

    #[test]
    fn test_validate_filename_no_traversal() {
        assert!(!validate_filename("../../../etc/passwd"));
        assert!(!validate_filename("..\\..\\windows\\system32"));
        assert!(!validate_filename("foo/bar.wasm"));
        assert!(!validate_filename("foo\\bar.wasm"));
        assert!(!validate_filename("/absolute/path.wasm"));
        assert!(!validate_filename("normal.exe"));
        assert!(!validate_filename(""));
        assert!(!validate_filename("."));
        assert!(validate_filename("worker.wasm"));
        assert!(validate_filename("my-module_v2.wasm"));
        assert!(validate_filename("a.wasm"));
    }

    #[test]
    fn test_validate_filename_wasm_extension() {
        assert!(validate_filename("test.wasm"));
        assert!(!validate_filename("test.exe"));
        assert!(!validate_filename("test.wasm.exe"));
        assert!(!validate_filename("test"));
        assert!(!validate_filename("test.WASM"));
        assert!(!validate_filename("test.pdf"));
    }

    #[tokio::test]
    async fn test_upload_list_delete_flow() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir.clone()),
            ..crate::AppState::in_memory()
        };

        let wasm_bytes: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let boundary = "testboundary12345";
        let mut body_parts: Vec<u8> = Vec::new();
        body_parts.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body_parts.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"worker.wasm\"\r\n",
        );
        body_parts.extend_from_slice(b"Content-Type: application/wasm\r\n");
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(wasm_bytes);
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let app = axum::Router::new()
            .route(
                "/api/workers/upload",
                axum::routing::post(upload_wasm_module),
            )
            .with_state(state.clone());

        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/workers/upload")
                    .header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(axum::body::Body::from(body_parts))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let resp_body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&resp_body).unwrap();
        assert_eq!(json["size"], wasm_bytes.len() as i64);
        let uploaded_filename = json["filename"].as_str().unwrap().to_string();
        assert!(uploaded_filename.ends_with("worker.wasm"));

        let list_response = list_wasm_modules(State(state.clone())).await;
        assert_eq!(list_response.status(), StatusCode::OK);

        let delete_response =
            delete_wasm_module(State(state.clone()), Path(uploaded_filename)).await;
        assert_eq!(delete_response.status(), StatusCode::OK);

        let entries: Vec<_> = std::fs::read_dir(&workers_dir).unwrap().collect();
        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn test_upload_rejects_invalid_wasm() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir),
            ..crate::AppState::in_memory()
        };

        let boundary = "testboundary99999";
        let mut body_parts: Vec<u8> = Vec::new();
        body_parts.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body_parts.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"worker.wasm\"\r\n",
        );
        body_parts.extend_from_slice(b"Content-Type: application/wasm\r\n");
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(b"NOT_WASM_DATA_HERE");
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let app = axum::Router::new()
            .route(
                "/api/workers/upload",
                axum::routing::post(upload_wasm_module),
            )
            .with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/workers/upload")
                    .header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(axum::body::Body::from(body_parts))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_upload_no_workers_dir_returns_503() {
        let state = AppState {
            workers_dir: None,
            ..crate::AppState::in_memory()
        };

        let app = axum::Router::new()
            .route(
                "/api/workers/upload",
                axum::routing::post(upload_wasm_module),
            )
            .with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/workers/upload")
                    .header("Content-Type", "multipart/form-data; boundary=x")
                    .body(axum::body::Body::from("--x--\r\n"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_delete_rejects_path_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir),
            ..crate::AppState::in_memory()
        };

        let response =
            delete_wasm_module(State(state), Path("../../../etc/passwd".to_string())).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir),
            ..crate::AppState::in_memory()
        };

        let response = delete_wasm_module(State(state), Path("nonexistent.wasm".to_string())).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_modules_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir),
            ..crate::AppState::in_memory()
        };

        let response = list_wasm_modules(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_modules_no_workers_dir_returns_503() {
        let state = AppState {
            workers_dir: None,
            ..crate::AppState::in_memory()
        };

        let response = list_wasm_modules(State(state)).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_upload_with_valid_magic_bytes_stores_file() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir.clone()),
            ..crate::AppState::in_memory()
        };

        let wasm_bytes: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0xFF, 0xFF];

        let boundary = "uploadboundary42";
        let mut body_parts: Vec<u8> = Vec::new();
        body_parts.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body_parts.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"valid-but-broken.wasm\"\r\n");
        body_parts.extend_from_slice(b"Content-Type: application/wasm\r\n");
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(wasm_bytes);
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let app = axum::Router::new()
            .route(
                "/api/workers/upload",
                axum::routing::post(upload_wasm_module),
            )
            .with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/workers/upload")
                    .header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(axum::body::Body::from(body_parts))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let entries: Vec<_> = std::fs::read_dir(&workers_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_upload_path_traversal_filename_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let workers_dir = tmp.path().to_path_buf();

        let state = AppState {
            workers_dir: Some(workers_dir.clone()),
            ..crate::AppState::in_memory()
        };

        let wasm_bytes: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let boundary = "traversalboundary99";
        let mut body_parts: Vec<u8> = Vec::new();
        body_parts.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body_parts.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"../../etc/evil.wasm\"\r\n",
        );
        body_parts.extend_from_slice(b"Content-Type: application/wasm\r\n");
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(wasm_bytes);
        body_parts.extend_from_slice(b"\r\n");
        body_parts.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

        let app = axum::Router::new()
            .route(
                "/api/workers/upload",
                axum::routing::post(upload_wasm_module),
            )
            .with_state(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/workers/upload")
                    .header(
                        "Content-Type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(axum::body::Body::from(body_parts))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_no_workers_dir_returns_503() {
        let state = AppState {
            workers_dir: None,
            ..crate::AppState::in_memory()
        };

        let response = delete_wasm_module(State(state), Path("any.wasm".to_string())).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
