use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;

/// Manifest describing a backup's contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: String,
    pub created_at: String,
    pub files: Vec<BackupEntry>,
    pub total_bytes: u64,
}

/// A single file entry within a backup manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub path: String,
    pub size: u64,
    pub etag: String,
    pub content_hash: String,
}

/// Summary info returned when listing or creating backups.
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub created_at: String,
    pub files: usize,
    pub bytes: u64,
}

/// Request body for restoring a backup.
#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub backup_id: String,
}

/// POST /api/admin/backup — create a new backup.
pub async fn create_backup(State(state): State<AppState>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Backups require --data-dir to be set",
            );
        }
    };

    let entries = match state.storage.list_all("/", 10000).await {
        Ok(e) => e,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to list files: {}", e),
            );
        }
    };

    let now = chrono::Utc::now();
    let backup_id = format!("backup-{}", now.format("%Y%m%d-%H%M%S"));
    let backup_dir = std::path::Path::new(&data_dir)
        .join("backups")
        .join(&backup_id);

    if let Err(e) = std::fs::create_dir_all(&backup_dir) {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to create backup directory: {}", e),
        );
    }

    let mut manifest = BackupManifest {
        id: backup_id.clone(),
        created_at: now.to_rfc3339(),
        files: Vec::new(),
        total_bytes: 0,
    };

    for meta in &entries {
        if meta.is_collection {
            continue;
        }

        match state.storage.get(&meta.path).await {
            Ok(content) => {
                let safe_path = meta.path.trim_start_matches('/').replace('/', "_");
                let file_path = backup_dir.join(&safe_path);

                if let Err(e) = std::fs::write(&file_path, &content) {
                    tracing::warn!("Failed to backup file {}: {}", meta.path, e);
                    continue;
                }

                manifest.total_bytes += meta.size;
                manifest.files.push(BackupEntry {
                    path: meta.path.clone(),
                    size: meta.size,
                    etag: meta.etag.clone(),
                    content_hash: meta.content_hash.as_str().to_string(),
                });
            }
            Err(e) => {
                tracing::warn!("Failed to read file {} for backup: {}", meta.path, e);
            }
        }
    }

    let manifest_path = backup_dir.join("manifest.json");
    match serde_json::to_string_pretty(&manifest) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&manifest_path, json) {
                return ApiError::internal(
                    ApiError::INTERNAL_ERROR,
                    format!("Failed to write manifest: {}", e),
                );
            }
        }
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to serialize manifest: {}", e),
            );
        }
    }

    let info = BackupInfo {
        id: backup_id,
        created_at: now.to_rfc3339(),
        files: manifest.files.len(),
        bytes: manifest.total_bytes,
    };

    (StatusCode::CREATED, axum::Json(info)).into_response()
}

/// GET /api/admin/backups — list available backups.
pub async fn list_backups(State(state): State<AppState>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::OK, axum::Json(serde_json::json!([]))).into_response();
        }
    };

    let backups_dir = std::path::Path::new(&data_dir).join("backups");
    let mut backups: Vec<BackupInfo> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&backups_dir) {
        for entry in entries.flatten() {
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            match std::fs::read_to_string(&manifest_path) {
                Ok(content) => match serde_json::from_str::<BackupManifest>(&content) {
                    Ok(manifest) => {
                        backups.push(BackupInfo {
                            id: manifest.id,
                            created_at: manifest.created_at,
                            files: manifest.files.len(),
                            bytes: manifest.total_bytes,
                        });
                    }
                    Err(_) => continue,
                },
                Err(_) => continue,
            }
        }
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    (StatusCode::OK, axum::Json(backups)).into_response()
}

/// POST /api/admin/restore — restore from a backup.
pub async fn restore_backup(
    State(state): State<AppState>,
    axum::Json(input): axum::Json<RestoreRequest>,
) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Restore requires --data-dir to be set",
            );
        }
    };

    let backup_dir = std::path::Path::new(&data_dir)
        .join("backups")
        .join(&input.backup_id);

    let manifest_path = backup_dir.join("manifest.json");
    let manifest_content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => {
            return ApiError::not_found(ApiError::NOT_FOUND, "Backup not found");
        }
    };

    let manifest: BackupManifest = match serde_json::from_str(&manifest_content) {
        Ok(m) => m,
        Err(_) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to parse backup manifest");
        }
    };

    let mut restored_files = 0u64;

    for entry in &manifest.files {
        let already_exists = state.storage.exists(&entry.path).await.unwrap_or(false);
        if already_exists {
            restored_files += 1;
            continue;
        }

        let safe_path = entry.path.trim_start_matches('/').replace('/', "_");
        let file_path = backup_dir.join(&safe_path);

        match std::fs::read(&file_path) {
            Ok(content) => {
                if let Err(e) = state
                    .storage
                    .put(&entry.path, bytes::Bytes::from(content), "backup-restore")
                    .await
                {
                    tracing::warn!("Failed to restore {}: {}", entry.path, e);
                } else {
                    restored_files += 1;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read backup file for {}: {}", entry.path, e);
            }
        }
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "restored_files": restored_files,
            "total_files": manifest.files.len(),
            "backup_id": input.backup_id,
        })),
    )
        .into_response()
}

/// DELETE /api/admin/backup/:id — delete a backup.
pub async fn delete_backup(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Delete requires --data-dir to be set",
            );
        }
    };

    let backup_dir = std::path::Path::new(&data_dir).join("backups").join(&id);

    if !backup_dir.exists() {
        return ApiError::not_found(ApiError::NOT_FOUND, "Backup not found");
    }

    if let Err(e) = std::fs::remove_dir_all(&backup_dir) {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to delete backup: {}", e),
        );
    }

    (StatusCode::NO_CONTENT, "").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use crate::build_router;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn backup_test_app() -> (axum::Router, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().to_string_lossy().to_string();
        let state = AppState::in_memory().with_data_dir(data_dir);
        (build_router(state), dir)
    }

    #[tokio::test]
    async fn test_backup_requires_data_dir() {
        let app = build_router(AppState::in_memory());
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_backup_and_restore_roundtrip() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/backup-test/file1.txt")
                    .body(axum::body::Body::from("hello backup"))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/backup-test/file2.txt")
                    .body(axum::body::Body::from("world backup"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(backup_resp.status(), StatusCode::CREATED);
        let backup_json = body_json(backup_resp).await;
        let backup_id = backup_json["id"].as_str().unwrap().to_string();
        assert_eq!(backup_json["files"], 2);

        let list_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backups")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let list_json = body_json(list_resp).await;
        assert_eq!(list_json.as_array().unwrap().len(), 1);

        let restore_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/restore")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "backup_id": backup_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(restore_resp.status(), StatusCode::OK);
        let restore_json = body_json(restore_resp).await;
        assert_eq!(restore_json["restored_files"], 2);
        assert_eq!(restore_json["total_files"], 2);
    }

    #[tokio::test]
    async fn test_restore_idempotent() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/idem-test/file.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_id = body_json(backup_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        let restore1 = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/restore")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "backup_id": backup_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let restore1_json = body_json(restore1).await;
        assert_eq!(restore1_json["restored_files"], 1);

        let restore2 = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/restore")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "backup_id": backup_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let restore2_json = body_json(restore2).await;
        assert_eq!(restore2_json["restored_files"], 1);
    }

    #[tokio::test]
    async fn test_delete_backup() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/del-test/file.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_id = body_json(backup_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        let del_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(&format!("/api/admin/backup/{}", backup_id))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

        let list_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backups")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let list_json = body_json(list_resp).await;
        assert_eq!(list_json.as_array().unwrap().len(), 0);
    }
}
