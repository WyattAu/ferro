use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::api::normalize_api_path;
use ferro_server::security::validate_path;
use ferro_server::{build_router, make_app, AppState};
use tower::ServiceExt;

async fn body_string(response: axum::response::Response) -> String {
    use http_body_util::BodyExt;
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap_or_default()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = body_string(response).await;
    serde_json::from_str(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn test_path_traversal_blocked_on_all_endpoints() {
    let app = make_app();

    let traversal_paths = [
        ("PUT", "/api/v1/files/../etc/passwd"),
        ("GET", "/api/v1/files/../../tmp"),
        ("DELETE", "/api/v1/files/../../../var"),
        ("PUT", "/api/v1/files/./etc/hosts"),
        ("PUT", "/api/v1/files/foo/../bar"),
    ];

    for (method, uri) in &traversal_paths {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                .method(*method)
                .uri(*uri)
                .body(Body::from("traversal attempt"))
                .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::NOT_FOUND,
            "Expected 400 or 404 for {} {}, got {}",
            method,
            uri,
            resp.status()
        );
    }
}

#[tokio::test]
async fn test_path_traversal_blocked_on_webdav_endpoints() {
    let app = make_app();

    let webdav_traversal = [
        ("MKCOL", "/../escaped"),
        ("PROPFIND", "/foo/../../etc"),
        ("COPY", "/a"),
        ("MOVE", "/a"),
    ];

    for (method, uri) in &webdav_traversal {
        let mut builder = Request::builder().method(*method).uri(*uri);
        if *method == "COPY" {
            builder = builder.header("Destination", "/../../tmp/b");
        } else if *method == "MOVE" {
            builder = builder.header("Destination", "/../../tmp/b");
        }
        let resp = app
            .clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(
            resp.status() == StatusCode::BAD_REQUEST
                || resp.status() == StatusCode::NOT_FOUND
                || resp.status() == StatusCode::FORBIDDEN,
            "Expected 400/404/403 for WebDAV {} {}, got {}",
            method,
            uri,
            resp.status()
        );
    }
}

#[tokio::test]
async fn test_url_decoded_path_still_reachable() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/..%2F..%2F..%2Fetc")
                .body(Body::from("encoded traversal"))
                .unwrap(),
        )
        .await
        .unwrap();

    let path_val = resp.status();
    let created = path_val == StatusCode::CREATED;
    if created {
        let get = app
            .clone()
            .oneshot(
                Request::builder()
                .uri("/etc/passwd")
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::NOT_FOUND,
            "File stored under decoded path should not be accessible at the original traversal target");
    }
}

#[tokio::test]
async fn test_slash_encoded_path_normalization() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/docs%2Freport.pdf")
                .body(Body::from("slash encoded"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED,
        "Slash-encoded path should work: got {}", resp.status());
}

#[tokio::test]
async fn test_normalize_api_path_rejects_all_traversal_variants() {
    let bad_paths = [
        "../../../etc/passwd",
        "/../../etc/passwd",
        "foo/../bar",
        "foo/./bar",
        "foo//bar",
        "./foo",
        "foo/.",
        "..",
        ".",
    ];

    for path in &bad_paths {
        assert!(
            normalize_api_path(path).is_err(),
            "normalize_api_path should reject '{}'",
            path
        );
    }
}

#[tokio::test]
async fn test_validate_path_rejects_traversal() {
    assert!(validate_path("docs/../../etc/passwd").is_err());
    assert!(validate_path("a/../b/../../c").is_err());
    assert!(validate_path("/../foo").is_err());
    assert!(validate_path("foo/../bar").is_err());
}

#[tokio::test]
async fn test_reserved_filenames_blocked_on_upload() {
    let app = make_app();

    let reserved_names = ["CON", "PRN", "AUX", "NUL", "COM3", "LPT1"];

    for name in &reserved_names {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/files/{}", name))
                .body(Body::from("reserved"))
                .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            resp.status() == StatusCode::BAD_REQUEST,
            "Reserved filename '{}' should be blocked, got {}",
            name,
            resp.status()
        );
    }
}

#[tokio::test]
async fn test_content_type_mismatch_blocked() {
    let app = make_app();

    let png_data = b"\x89PNG\r\n\x1a\nfake png content here";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/fake.pdf")
                .header("content-type", "application/pdf")
                .body(Body::from(png_data.as_ref()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"], "content_type_mismatch");
}

#[tokio::test]
async fn test_octet_stream_skips_content_type_check() {
    let app = make_app();

    let png_data = b"\x89PNG\r\n\x1a\nfake png";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/skip-check.bin")
                .header("content-type", "application/octet-stream")
                .body(Body::from(png_data.as_ref()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_control_chars_in_filename_blocked() {
    use ferro_server::security::validate_filename;

    assert!(validate_filename("file\x00name.txt").is_err());
    assert!(validate_filename("file\x01name.txt").is_err());
    assert!(validate_filename("file\x1fname.txt").is_err());
}

#[tokio::test]
async fn test_csrf_token_uniqueness_and_length() {
    use ferro_server::security::{generate_csrf_token, verify_csrf_token};

    let tokens: Vec<String> = (0..100).map(|_| generate_csrf_token()).collect();

    for token in &tokens {
        assert_eq!(token.len(), 64);
        assert!(verify_csrf_token(token, token));
    }

    let unique: std::collections::HashSet<_> = tokens.into_iter().collect();
    assert_eq!(unique.len(), 100, "All CSRF tokens should be unique");
}

#[tokio::test]
async fn test_default_passwords_rejected_on_change() {
    let app = build_router(
        AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("changeme".to_string())),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/files")
                .header("Authorization", "Basic YWRtaW46Y2hhbmdlbWU=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = body_json(resp).await;
    assert_eq!(json["error_code"], "PASSWORD_CHANGE_REQUIRED");
}

#[tokio::test]
async fn test_empty_filename_blocked() {
    use ferro_server::security::validate_filename;

    assert!(validate_filename("").is_err());
    assert!(validate_filename("   ").is_err());
    assert!(validate_filename("...").is_err());
}

#[tokio::test]
async fn test_auth_account_lockout_at_limit() {
    use ferro_server::security::AuthAttemptTracker;
    use std::time::Duration;

    let tracker = AuthAttemptTracker::new(5, Duration::from_secs(60));

    for i in 0..4 {
        assert!(
            !tracker.record_failure("10.0.0.1", "admin"),
            "Failure {} should not trigger lockout",
            i + 1
        );
    }
    assert!(
        tracker.record_failure("10.0.0.1", "admin"),
        "5th failure should trigger lockout"
    );
    assert!(tracker.is_locked_out("10.0.0.1", "admin"));
    assert!(!tracker.is_locked_out("10.0.0.1", "otheruser"));
}
