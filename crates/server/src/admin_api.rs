use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::AppState;

/// Return server statistics (version, uptime, file counts).
pub async fn admin_stats(State(state): State<AppState>) -> Response {
    let version = env!("CARGO_PKG_VERSION");
    let uptime = state.started_at.elapsed().as_secs();

    let mut file_count = 0u64;
    let mut collection_count = 0u64;
    let mut total_bytes = 0u64;

    if let Ok(entries) = state.storage.list_all("/", 10000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_bytes += meta.size;
            }
        }
    }

    let auth_type = if state.oidc.is_some() {
        "oidc"
    } else if state.admin_user.is_some() {
        "basic"
    } else {
        "none"
    };

    let wasm_workers_loaded = 0u32;

    let body = serde_json::json!({
        "version": version,
        "uptime_seconds": uptime,
        "total_files": file_count,
        "total_directories": collection_count,
        "total_bytes": total_bytes,
        "storage_backend": "memory",
        "auth_type": auth_type,
        "wasm_workers_loaded": wasm_workers_loaded,
        "search_enabled": state.search.is_some(),
        "features": {
            "s3": cfg!(feature = "s3"),
            "gcs": cfg!(feature = "gcs"),
            "azure": cfg!(feature = "azure"),
            "oidc": state.oidc.is_some(),
            "cedar": state.cedar.is_some(),
        }
    });

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Query parameters for the admin storage endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct StorageQueryParams {
    pub limit: Option<usize>,
}

/// Return detailed storage statistics.
pub async fn admin_storage(
    State(state): State<AppState>,
    Query(_params): Query<StorageQueryParams>,
) -> Response {
    let mut file_count = 0u64;
    let mut collection_count = 0u64;
    let mut total_bytes = 0u64;
    let mut largest_file_path: String = String::new();
    let mut largest_file_size: u64 = 0;
    let mut recent_files: Vec<serde_json::Value> = Vec::new();

    if let Ok(entries) = state.storage.list_all("/", 10000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_bytes += meta.size;

                if meta.size > largest_file_size {
                    largest_file_size = meta.size;
                    largest_file_path = meta.path.clone();
                }

                recent_files.push(serde_json::json!({
                    "path": meta.path,
                    "size": meta.size,
                    "modified_at": meta.modified_at.to_rfc3339(),
                }));
            }
        }
    }

    recent_files.sort_by(|a, b| {
        let a_time = a["modified_at"].as_str().unwrap_or("");
        let b_time = b["modified_at"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });
    recent_files.truncate(10);

    let largest_file = if largest_file_path.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::json!({
            "path": largest_file_path,
            "size": largest_file_size,
        })
    };

    let body = serde_json::json!({
        "backend": "memory",
        "total_bytes": total_bytes,
        "file_count": file_count,
        "directory_count": collection_count,
        "largest_file": largest_file,
        "recent_files": recent_files,
    });

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Query parameters for the admin audit endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct AuditQueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Return paginated audit log entries.
pub async fn admin_audit(
    State(state): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Response {
    let limit: usize = params.limit.unwrap_or(100);
    let offset: usize = params.offset.unwrap_or(0);
    let total = state.audit_log.len().await;
    let entries = state.audit_log.recent_with_offset(limit, offset).await;

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "entries": entries,
            "total": total,
            "limit": limit,
            "offset": offset,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {

    use crate::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use base64::Engine;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn admin_test_app() -> axum::Router {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        crate::build_router(state)
    }

    #[allow(dead_code)] // Test helper
    fn no_auth_test_app() -> axum::Router {
        crate::build_router(AppState::in_memory())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn seed_files(app: &axum::Router, creds: &str) {
        for i in 0..5 {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(&format!("/test{}.txt", i))
                        .header("Authorization", format!("Basic {}", creds))
                        .body(Body::from(format!("content {}", i)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_admin_stats_requires_auth() {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        let app = crate::build_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/admin/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_admin_stats_reports_correct_counts() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        seed_files(&app, &creds).await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/storage")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;

        assert_eq!(json["backend"], "memory");
        assert!(json.get("total_bytes").is_some());
        assert_eq!(json["file_count"], 5);
        assert!(json.get("directory_count").is_some());
        assert!(json.get("largest_file").is_some());
        assert!(json.get("recent_files").is_some());
        assert!(json["recent_files"].is_array());
    }

    #[tokio::test]
    async fn test_admin_audit_with_auth() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/audit-test.txt")
                    .header("Authorization", format!("Basic {}", &creds))
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/audit")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;

        assert!(json.get("entries").is_some());
        assert!(json.get("total").is_some());
        assert!(json.get("limit").is_some());
        assert!(json.get("offset").is_some());
        assert!(json["entries"].is_array());
    }
}
