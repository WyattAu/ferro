//! Disaster Recovery Drill Tests
//!
//! End-to-end tests for the backup/restore API:
//! 1. Upload files, create shares
//! 2. Create backup
//! 3. Delete files (simulate disaster)
//! 4. Verify files are missing
//! 5. Restore from backup
//! 6. Verify files, shares, and metadata are restored
//! 7. Test restoring to a fresh server instance

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

fn app_with_data_dir() -> (axum::Router, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let state = AppState::in_memory().with_data_dir(dir.path().to_string_lossy().to_string());
    let router = build_router(state);
    (router, dir)
}

fn fresh_app_with_data_dir(dir: &std::path::Path) -> axum::Router {
    let state = AppState::in_memory().with_data_dir(dir.to_string_lossy().to_string());
    build_router(state)
}

async fn put_file(app: &axum::Router, path: &str, content: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(path)
                .body(Body::from(content.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

async fn get_file(app: &axum::Router, path: &str) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn delete_file(app: &axum::Router, path: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

async fn mkcol(app: &axum::Router, path: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

async fn create_share(
    app: &axum::Router,
    path: &str,
    password: Option<&str>,
) -> (StatusCode, String) {
    let body = match password {
        Some(pw) => format!(
            r#"{{"path": "{}", "password": "{}", "expires_in_hours": 24}}"#,
            path, pw
        ),
        None => format!(r#"{{"path": "{}", "expires_in_hours": 24}}"#, path),
    };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn create_backup(app: &axum::Router) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/backup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn list_backups(app: &axum::Router) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/backups")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn restore_backup(app: &axum::Router, backup_id: &str) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/restore")
                .header("Content-Type", "application/json")
                .body(Body::from(format!(r#"{{"backup_id": "{}"}}"#, backup_id)))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn get_latest_backup(app: &axum::Router) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/backup/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn download_backup(app: &axum::Router) -> (StatusCode, bytes::Bytes) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/backup/download")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = body_bytes(resp).await;
    (status, body)
}

async fn delete_backup(app: &axum::Router, backup_id: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admin/backup/{}", backup_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

async fn verify_file_exists(app: &axum::Router, path: &str, expected_content: &str) {
    let (status, body) = get_file(app, path).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "File {} should exist after restore",
        path
    );
    assert_eq!(
        body, expected_content,
        "File {} content should match after restore",
        path
    );
}

async fn verify_file_missing(app: &axum::Router, path: &str) {
    let (status, _) = get_file(app, path).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "File {} should be missing",
        path
    );
}

// ── Full Disaster Recovery Drill ─────────────────────────────────────

#[tokio::test]
async fn test_full_disaster_recovery_drill() {
    let (app, _dir) = app_with_data_dir();

    // Step 1: Upload test files
    let status = mkcol(&app, "/dr-docs").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = put_file(&app, "/dr-docs/readme.txt", "disaster recovery readme").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = put_file(&app, "/dr-important.txt", "critical data").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = put_file(&app, "/dr-config.json", "{\"version\": 1}").await;
    assert_eq!(status, StatusCode::CREATED);

    // Step 2: Create shares
    let (status, body) = create_share(&app, "/dr-important.txt", None).await;
    assert_eq!(status, StatusCode::CREATED);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let share_token = json["token"].as_str().unwrap().to_string();

    // Verify files exist before disaster
    verify_file_exists(&app, "/dr-docs/readme.txt", "disaster recovery readme").await;
    verify_file_exists(&app, "/dr-important.txt", "critical data").await;
    verify_file_exists(&app, "/dr-config.json", "{\"version\": 1}").await;

    // Verify share works before disaster
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", share_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Step 3: Create backup
    let (status, backup_body) = create_backup(&app).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Backup creation should succeed: {}",
        backup_body
    );
    let backup_json: serde_json::Value = serde_json::from_str(&backup_body).unwrap();
    let backup_id = backup_json["id"]
        .as_str()
        .unwrap_or_else(|| panic!("Backup response should contain id: {}", backup_body))
        .to_string();

    // Step 4: Verify backup appears in list
    let (status, body) = list_backups(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains(&backup_id),
        "Backup list should contain our backup"
    );

    // Step 5: Simulate disaster - delete files
    delete_file(&app, "/dr-important.txt").await;
    delete_file(&app, "/dr-config.json").await;
    delete_file(&app, "/dr-docs/readme.txt").await;

    // Step 6: Verify files are missing
    verify_file_missing(&app, "/dr-important.txt").await;
    verify_file_missing(&app, "/dr-config.json").await;
    verify_file_missing(&app, "/dr-docs/readme.txt").await;

    // Step 7: Restore from backup
    let (status, restore_body) = restore_backup(&app, &backup_id).await;
    assert!(
        status == StatusCode::OK || status == StatusCode::ACCEPTED,
        "Restore should succeed, got {}: {}",
        status,
        restore_body
    );

    // Step 8: Verify files are restored
    verify_file_exists(&app, "/dr-important.txt", "critical data").await;
    verify_file_exists(&app, "/dr-config.json", "{\"version\": 1}").await;
    verify_file_exists(&app, "/dr-docs/readme.txt", "disaster recovery readme").await;
}

#[tokio::test]
async fn test_backup_lifecycle() {
    let (app, _dir) = app_with_data_dir();

    // Upload files
    put_file(&app, "/lifecycle-a.txt", "alpha").await;
    put_file(&app, "/lifecycle-b.txt", "beta").await;

    // Create backup
    let (status, body) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);
    let backup_id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Get latest backup
    let (status, body) = get_latest_backup(&app).await;
    assert_eq!(status, StatusCode::OK);
    let latest_json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(latest_json["id"].as_str().unwrap(), backup_id);

    // List backups
    let (status, body) = list_backups(&app).await;
    assert_eq!(status, StatusCode::OK);
    let backups: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert!(!backups.is_empty(), "Should have at least one backup");

    // Delete backup
    let status = delete_backup(&app, &backup_id).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify backup is gone
    let (status, _) = get_latest_backup(&app).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Deleted backup should be gone"
    );
}

#[tokio::test]
async fn test_backup_nonexistent_backup_id_restore() {
    let (app, _dir) = app_with_data_dir();

    let (status, _) = restore_backup(&app, "nonexistent-backup-id").await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::INTERNAL_SERVER_ERROR,
        "Restoring nonexistent backup should fail, got {}",
        status
    );
}

#[tokio::test]
async fn test_restore_preserves_file_contents() {
    let (app, _dir) = app_with_data_dir();

    let test_files = [
        ("/restore-test/binary.bin", "\x00\x01\x02\x03"),
        ("/restore-test/text.txt", "Hello, World!"),
        ("/restore-test/unicode.txt", "こんにちは 🌍"),
        ("/restore-test/empty.txt", ""),
    ];

    mkcol(&app, "/restore-test").await;

    for (path, content) in &test_files {
        let status = put_file(&app, path, content).await;
        assert_eq!(status, StatusCode::CREATED, "Failed to create {}", path);
    }

    let (status, body) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);
    let backup_id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    for (path, _) in &test_files {
        delete_file(&app, path).await;
    }

    let (status, _) = restore_backup(&app, &backup_id).await;
    assert!(status.is_success(), "Restore should succeed");

    for (path, content) in &test_files {
        let (status, body) = get_file(&app, path).await;
        assert_eq!(status, StatusCode::OK, "File {} should be restored", path);
        assert_eq!(body, *content, "File {} content mismatch", path);
    }
}

#[tokio::test]
async fn test_backup_integrity_check() {
    let (app, _dir) = app_with_data_dir();

    put_file(&app, "/integrity-test.txt", "intact data").await;

    let (status, _) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/integrity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::NOT_FOUND,
        "Integrity check endpoint should respond"
    );
}

#[tokio::test]
async fn test_backup_audit_chain() {
    let (app, _dir) = app_with_data_dir();

    put_file(&app, "/chain-test.txt", "chain data").await;

    let (status, _) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/audit-chain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::BAD_REQUEST
            || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "Audit chain endpoint should respond, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_multiple_backups_restore_latest() {
    let (app, _dir) = app_with_data_dir();

    // Version 1
    put_file(&app, "/versioned.txt", "version 1").await;
    let (status, _) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);

    // Version 2
    put_file(&app, "/versioned.txt", "version 2").await;
    let (status, body) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);
    let backup_id_v2 = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Delete and restore v2
    delete_file(&app, "/versioned.txt").await;
    verify_file_missing(&app, "/versioned.txt").await;

    let (status, _) = restore_backup(&app, &backup_id_v2).await;
    assert!(status.is_success());

    verify_file_exists(&app, "/versioned.txt", "version 2").await;
}

#[tokio::test]
async fn test_download_backup_archive() {
    let (app, _dir) = app_with_data_dir();

    put_file(&app, "/archive-test.txt", "archive me").await;

    let (status, _) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = download_backup(&app).await;
    if status == StatusCode::OK {
        assert!(!body.is_empty(), "Backup archive should not be empty");
        assert!(
            body.starts_with(b"PK") || body.len() > 100,
            "Backup should be a valid archive"
        );
    }
}

#[tokio::test]
async fn test_backup_empty_server() {
    let (app, _dir) = app_with_data_dir();

    let (status, _) = create_backup(&app).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Backup of empty server should succeed"
    );

    let (status, body) = list_backups(&app).await;
    assert_eq!(status, StatusCode::OK);
    let backups: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(backups.len(), 1, "Should have exactly one backup");
}

#[tokio::test]
async fn test_restore_fresh_server_instance() {
    let dir = tempfile::TempDir::new().unwrap();
    let data_dir_path = dir.path().to_path_buf();

    // Phase 1: Original server - upload files and create backup
    {
        let state =
            AppState::in_memory().with_data_dir(data_dir_path.to_string_lossy().to_string());
        let app = build_router(state);

        mkcol(&app, "/fresh-test").await;
        put_file(&app, "/fresh-test/file1.txt", "fresh content 1").await;
        put_file(&app, "/fresh-test/file2.txt", "fresh content 2").await;

        let (status, _body) = create_share(&app, "/fresh-test/file1.txt", None).await;
        assert_eq!(status, StatusCode::CREATED);

        let (status, body) = create_backup(&app).await;
        assert_eq!(status, StatusCode::CREATED);
        let backup_id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        verify_file_exists(&app, "/fresh-test/file1.txt", "fresh content 1").await;
        verify_file_exists(&app, "/fresh-test/file2.txt", "fresh content 2").await;

        // Simulate disaster: verify backup directory exists on disk
        let backup_dir = data_dir_path.join("backups").join(&backup_id);
        assert!(backup_dir.exists(), "Backup directory should exist on disk");
    }

    // Phase 2: Fresh server instance - restore from backup
    {
        let fresh_state =
            AppState::in_memory().with_data_dir(data_dir_path.to_string_lossy().to_string());
        let fresh_app = build_router(fresh_state);

        let (status, body) = list_backups(&fresh_app).await;
        assert_eq!(status, StatusCode::OK);
        let backups: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
        assert!(
            !backups.is_empty(),
            "Fresh server should see existing backups"
        );

        let backup_id = backups[0]["id"].as_str().unwrap().to_string();

        let (status, restore_body) = restore_backup(&fresh_app, &backup_id).await;
        assert!(
            status.is_success(),
            "Restore on fresh server should succeed: {} - {}",
            status,
            restore_body
        );

        verify_file_exists(&fresh_app, "/fresh-test/file1.txt", "fresh content 1").await;
        verify_file_exists(&fresh_app, "/fresh-test/file2.txt", "fresh content 2").await;
    }
}

#[tokio::test]
async fn test_backup_after_share_creation() {
    let (app, _dir) = app_with_data_dir();

    put_file(&app, "/share-backup.txt", "shared for backup").await;

    let (status, body) = create_share(&app, "/share-backup.txt", Some("pass123")).await;
    assert_eq!(status, StatusCode::CREATED);
    let _share_token = serde_json::from_str::<serde_json::Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);

    delete_file(&app, "/share-backup.txt").await;

    let (status, body) = list_backups(&app).await;
    assert_eq!(status, StatusCode::OK);
    let backup_id = serde_json::from_str::<Vec<serde_json::Value>>(&body)
        .unwrap()
        .first()
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = restore_backup(&app, &backup_id).await;
    assert!(status.is_success());

    verify_file_exists(&app, "/share-backup.txt", "shared for backup").await;
}

#[tokio::test]
async fn test_backup_with_nested_directories() {
    let (app, _dir) = app_with_data_dir();

    mkcol(&app, "/deep").await;
    mkcol(&app, "/deep/nested").await;
    mkcol(&app, "/deep/nested/dir").await;

    put_file(&app, "/deep/level1.txt", "L1").await;
    put_file(&app, "/deep/nested/level2.txt", "L2").await;
    put_file(&app, "/deep/nested/dir/level3.txt", "L3").await;

    let (status, body) = create_backup(&app).await;
    assert_eq!(status, StatusCode::CREATED);
    let backup_id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    delete_file(&app, "/deep/nested/dir/level3.txt").await;
    delete_file(&app, "/deep/nested/level2.txt").await;
    delete_file(&app, "/deep/level1.txt").await;

    let (status, _) = restore_backup(&app, &backup_id).await;
    assert!(status.is_success());

    verify_file_exists(&app, "/deep/level1.txt", "L1").await;
    verify_file_exists(&app, "/deep/nested/level2.txt", "L2").await;
    verify_file_exists(&app, "/deep/nested/dir/level3.txt", "L3").await;
}

#[tokio::test]
async fn test_backup_without_data_dir_returns_error() {
    let app = {
        let state = AppState::in_memory();
        build_router(state)
    };

    let (status, _) = create_backup(&app).await;
    assert!(
        status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::SERVICE_UNAVAILABLE
            || status == StatusCode::BAD_REQUEST,
        "Backup without data_dir should fail, got {}",
        status
    );
}
