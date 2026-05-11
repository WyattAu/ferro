use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::{AppState, build_router};
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
    use http_body_util::BodyExt;
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = body_bytes(response).await;
    serde_json::from_slice(&bytes).unwrap()
}

fn test_app() -> axum::Router {
    build_router(AppState::in_memory())
}

fn auth_app() -> axum::Router {
    build_router(
        AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string())),
    )
}

// 1. Config endpoint
#[tokio::test]
async fn test_get_config_returns_version_and_features() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;

    assert!(json.get("version").is_some());
    assert_eq!(json["auth_enabled"], false);
    assert_eq!(json["search_enabled"], false);
    assert_eq!(json["wasm_enabled"], false);
    assert_eq!(json["cedar_enabled"], false);
    assert_eq!(json["metadata_persistent"], false);
    assert_eq!(json["cas_enabled"], false);
    assert_eq!(json["storage"], "configured");
    assert_eq!(json["wopi_configured"], false);
}

#[tokio::test]
async fn test_get_config_deprecated_endpoint() {
    let app = test_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("deprecation").unwrap(), "true");
    let v1_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let deprecated_json = body_json(resp).await;
    let v1_json = body_json(v1_resp).await;
    assert_eq!(deprecated_json, v1_json);
}

// 2. Health check — liveness
#[tokio::test]
async fn test_healthz_returns_ok() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert_eq!(body, "ok");
}

// 3. Health check — readiness
#[tokio::test]
async fn test_readyz_returns_json_status() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");
    assert!(json["subsystems"]["storage"].is_string());
    assert!(json["subsystems"]["metadata"].is_string());
}

// 4. Well-known health check
#[tokio::test]
async fn test_well_known_ferro_returns_version_and_subsystems() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/ferro")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");
    assert!(json.get("version").is_some());
    assert!(json.get("uptime_seconds").is_some());
    assert!(json["subsystems"]["storage"].is_string());
    assert!(json["subsystems"]["auth"].is_string());
    assert!(json["subsystems"]["search"].is_string());
    assert!(json["subsystems"]["wasm"].is_string());
    assert!(json["subsystems"]["metadata"].is_string());
    assert!(json["subsystems"]["cas"].is_string());
}

// 5. File listing — root directory
#[tokio::test]
async fn test_list_files_root_empty() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["entries"].is_array());
}

#[tokio::test]
async fn test_list_files_with_path_query() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/rest-list-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files?path=/rest-list-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["entries"].is_array());
}

// 6. Create directory via REST mkdir
#[tokio::test]
async fn test_mkdir_creates_directory() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/mkdir")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"path": "/newfolder"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let location = resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let json = body_json(resp).await;
    assert_eq!(json["path"], "/newfolder");
    assert!(json.get("created_at").is_some());
    assert_eq!(location, "/newfolder");
}

#[tokio::test]
async fn test_mkdir_duplicate_returns_conflict() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/mkdir")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"path": "/dup-dir"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/mkdir")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"path": "/dup-dir"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

// 7. Upload file via REST PUT
#[tokio::test]
async fn test_upload_file_returns_201_with_metadata() {
    let app = test_app();
    let content = "hello ferro world";
    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/test.txt")
                .header("content-type", "text/plain")
                .body(Body::from(content))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert!(resp.headers().get("etag").is_some());
    assert!(resp.headers().get("location").is_some());
    let json = body_json(resp).await;
    assert_eq!(json["path"], "/test.txt");
    assert_eq!(json["size"], content.len() as u64);
    assert!(json.get("etag").is_some());
    assert!(json.get("content_hash").is_some());
    assert!(json.get("created_at").is_some());
    assert!(json.get("modified_at").is_some());
}

#[tokio::test]
async fn test_upload_file_nested_path() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/nested-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/nested-dir/deep.txt")
                .body(Body::from("deep content"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["path"], "/nested-dir/deep.txt");
}

// 8. Download file via REST GET
#[tokio::test]
async fn test_download_file_content_matches_upload() {
    let app = test_app();
    let content = "ferro integration test content";

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/download-test.txt")
                .header("content-type", "text/plain")
                .body(Body::from(content))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/download-test.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().get("etag").is_some());
    assert!(resp.headers().get("content-length").is_some());
    let body = body_string(resp).await;
    assert_eq!(body, content);
}

#[tokio::test]
async fn test_download_file_returns_content_disposition() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/my-report.pdf")
                .body(Body::from("pdf data"))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/my-report.pdf")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let disposition = resp
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(disposition.contains("my-report.pdf"));
}

// 9. File metadata — collection returns JSON
#[tokio::test]
async fn test_get_collection_returns_json_metadata() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/meta-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/meta-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["is_collection"], true);
    assert!(json.get("name").is_some());
    assert!(json.get("path").is_some());
    assert!(json.get("etag").is_some());
    assert!(json.get("modified_at").is_some());
    assert!(json.get("created_at").is_some());
}

// 10. Delete file
#[tokio::test]
async fn test_delete_file_returns_204() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/to-delete.txt")
                .body(Body::from("delete me"))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/files/to-delete.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let get_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/to-delete.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

// 11. Search — with search disabled returns empty results
#[tokio::test]
async fn test_search_without_engine_returns_empty_results() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["query"], "test");
    assert_eq!(json["results"], serde_json::json!([]));
    assert_eq!(json["total"], 0);
}

#[tokio::test]
async fn test_search_empty_query_returns_400() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/search?q=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// 12. Quota info
#[tokio::test]
async fn test_quota_unlimited_by_default() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/quota")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["unlimited"], true);
    assert_eq!(json["used_bytes"], 0);
    assert_eq!(json["quota_bytes"], 0);
    assert_eq!(json["used_percent"], 0.0);
    assert_eq!(json["file_count"], 0);
}

// 13. Error cases — nonexistent file returns 404
#[tokio::test]
async fn test_get_nonexistent_file_returns_404() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/nonexistent-file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let json = body_json(resp).await;
    assert_eq!(json["error"], "not_found");
}

#[tokio::test]
async fn test_delete_nonexistent_file_returns_404() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/files/no-such-file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// 14. Auth protected endpoint — no credentials returns 401
#[tokio::test]
async fn test_auth_protected_endpoint_without_credentials_returns_401() {
    let app = auth_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(resp.headers().get("WWW-Authenticate").is_some());
}

#[tokio::test]
async fn test_auth_valid_credentials_accepted() {
    use base64::Engine;
    let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");
    let app = auth_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files")
                .header("Authorization", format!("Basic {}", creds))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_public_paths_bypass_auth() {
    let app = auth_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// 15. Full CRUD lifecycle
#[tokio::test]
async fn test_full_rest_crud_lifecycle() {
    let app = test_app();

    // Create directory
    let mkdir_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/mkdir")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"path": "/lifecycle-dir"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mkdir_resp.status(), StatusCode::CREATED);

    // Upload file
    let upload_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/lifecycle-dir/doc.txt")
                .header("content-type", "text/plain")
                .body(Body::from("lifecycle content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upload_resp.status(), StatusCode::CREATED);
    let upload_json = body_json(upload_resp).await;
    let etag = upload_json["etag"].as_str().unwrap().to_string();

    // Download and verify
    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/lifecycle-dir/doc.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let body = body_string(get_resp).await;
    assert_eq!(body, "lifecycle content");

    // Conditional GET (If-None-Match)
    let cond_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/lifecycle-dir/doc.txt")
                .header("if-none-match", &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cond_resp.status(), StatusCode::NOT_MODIFIED);

    // List directory — file should appear
    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files?path=/lifecycle-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json = body_json(list_resp).await;
    assert!(!list_json["entries"].as_array().unwrap().is_empty());

    // Delete file
    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/files/lifecycle-dir/doc.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    // Verify deleted
    let gone_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/lifecycle-dir/doc.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gone_resp.status(), StatusCode::NOT_FOUND);
}

// 16. Storage stats
#[tokio::test]
async fn test_storage_stats_returns_usage() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/storage/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.get("files").is_some());
    assert!(json.get("collections").is_some());
    assert!(json.get("total_bytes").is_some());
    assert!(json.get("cas").is_some());
}

// 17. Copy file
#[tokio::test]
async fn test_copy_file() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/original.txt")
                .body(Body::from("copy me"))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/copy")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"source": "/original.txt", "destination": "/copied.txt"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");

    // Verify content matches
    let get_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/copied.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    assert_eq!(body_string(get_resp).await, "copy me");
}

// 18. Move file
#[tokio::test]
async fn test_move_file() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/move-source.txt")
                .body(Body::from("move me"))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/move")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"source": "/move-source.txt", "destination": "/move-dest.txt"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ok");

    // Source should be gone
    let gone = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/move-source.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);

    // Dest should exist
    let get_resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/move-dest.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    assert_eq!(body_string(get_resp).await, "move me");
}

// 19. Copy/Move missing params
#[tokio::test]
async fn test_copy_missing_params_returns_422() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/copy")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// 20. Method not allowed on file content
#[tokio::test]
async fn test_patch_file_returns_method_not_allowed() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/files/test.txt")
                .body(Body::from("patch"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// 21. Auth info endpoint
#[tokio::test]
async fn test_auth_info_anonymous() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["sub"], "anonymous");
    assert_eq!(json["iss"], "ferro");
    assert_eq!(json["auth_type"], "none");
}

// 22. Audit log endpoint
#[tokio::test]
async fn test_audit_log_returns_entries() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
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
}

// 23. Tags endpoint
#[tokio::test]
async fn test_tags_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tags")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// 24. Trash endpoint
#[tokio::test]
async fn test_trash_list() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/trash")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// 25. Upload file via legacy /api/ prefix (deprecated)
#[tokio::test]
async fn test_upload_via_deprecated_api_prefix() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/files/deprecated-upload.txt")
                .body(Body::from("legacy content"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["path"], "/deprecated-upload.txt");
}
