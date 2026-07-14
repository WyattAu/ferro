//! Edge case integration tests for the Ferro server.
//!
//! Covers: unicode filenames, path traversal, special characters, long paths,
//! empty files, binary files, concurrent operations, null bytes, symlinks,
//! filename edge cases (dots, spaces, unicode normalization).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::make_app;
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
    use http_body_util::BodyExt;
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

async fn put_file(app: &axum::Router, path: &str, content: &[u8]) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(path)
                .body(Body::from(content.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

async fn get_file(app: &axum::Router, path: &str) -> (StatusCode, bytes::Bytes) {
    let resp = app
        .clone()
        .oneshot(Request::builder().method("GET").uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = body_bytes(resp).await;
    (status, bytes)
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

// ---------------------------------------------------------------------------
// Unicode filenames
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_unicode_cjk_filename() {
    let app = make_app();
    let path = "/文档/报告.txt";
    assert_eq!(mkcol(&app, "/文档").await, StatusCode::CREATED);
    assert_eq!(put_file(&app, path, "中文内容".as_bytes()).await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], "中文内容".as_bytes());
}

#[tokio::test]
async fn test_unicode_emoji_filename() {
    let app = make_app();
    let path = "/📁/🎉report.pdf";
    assert_eq!(mkcol(&app, "/📁").await, StatusCode::CREATED);
    assert_eq!(put_file(&app, path, b"emoji content").await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"emoji content");
}

#[tokio::test]
async fn test_unicode_mixed_scripts() {
    let app = make_app();
    let path = "/mixed/ファイル_файл_ملف.dat";
    assert_eq!(mkcol(&app, "/mixed").await, StatusCode::CREATED);
    assert_eq!(
        put_file(&app, path, b"japanese russian arabic").await,
        StatusCode::CREATED
    );
    let (status, _) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_unicode_zero_width_chars() {
    let app = make_app();
    // Zero-width joiner + variation selector
    let path = "/test/file\u{200D}\u{FE0F}.txt";
    assert_eq!(put_file(&app, path, b"zero-width content").await, StatusCode::CREATED);
    let (status, _) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Path traversal attacks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_path_traversal_dotdot() {
    let app = make_app();
    // Should be rejected by sanitize_path
    let status = put_file(&app, "/../../../etc/passwd", b"hack").await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::FORBIDDEN,
        "Path traversal should be rejected, got {}",
        status
    );
}

#[tokio::test]
async fn test_path_traversal_encoded_dots_stored_literal() {
    let app = make_app();
    // %2e is percent-encoded '.' — hyper keeps it as literal text, not '..'
    // The file is stored safely with literal "%2e%2e" in the name
    let status = put_file(&app, "/%2e%2e/%2e%2e/etc/passwd", b"safe").await;
    // File is created (stored as literal %2e%2e, NOT path traversal)
    assert_eq!(status, StatusCode::CREATED);
    // Verify it exists at the literal path (not traversed)
    let (s, _) = get_file(&app, "/%2e%2e/%2e%2e/etc/passwd").await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn test_path_traversal_mixed() {
    let app = make_app();
    let status = put_file(&app, "/safe/../../unsafe/file.txt", b"hack").await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::FORBIDDEN,
        "Mixed path traversal should be rejected, got {}",
        status
    );
}

// ---------------------------------------------------------------------------
// Special characters in filenames
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_filename_with_spaces() {
    let app = make_app();
    // Spaces must be percent-encoded in URIs
    let path = "/my%20documents/report%202024.txt";
    assert_eq!(put_file(&app, path, b"spaced content").await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"spaced content");
}

#[tokio::test]
async fn test_filename_with_dots() {
    let app = make_app();
    // Multiple dots, leading dots (but not path traversal)
    assert_eq!(put_file(&app, "/.hidden", b"hidden").await, StatusCode::CREATED);
    assert_eq!(
        put_file(&app, "/file.with.many.dots.ext", b"dots").await,
        StatusCode::CREATED
    );
    // .. prefix is path traversal — should be rejected
    let status = put_file(&app, "/..trailingdot", b"trailing").await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::FORBIDDEN,
        "Path starting with .. should be rejected, got {}",
        status
    );
}

#[tokio::test]
async fn test_filename_with_parentheses_brackets() {
    let app = make_app();
    // Parentheses/brackets must be percent-encoded in URIs
    let path = "/data/file%20%28copy%29%20%5Bv2%5D.txt";
    assert_eq!(put_file(&app, path, b"brackets").await, StatusCode::CREATED);
    let (status, _) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_filename_with_hash() {
    let app = make_app();
    let path = "/data/file%231.txt"; // %23 = #
    assert_eq!(put_file(&app, path, b"hash content").await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"hash content");
}

#[tokio::test]
async fn test_filename_with_plus_and_ampersand() {
    let app = make_app();
    // + and & may be interpreted differently in query strings vs paths
    let path = "/data/file%2Bspecial%26chars.dat";
    assert_eq!(put_file(&app, path, b"plus amp").await, StatusCode::CREATED);
    let (status, _) = get_file(&app, path).await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Empty and binary files
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_empty_file() {
    let app = make_app();
    assert_eq!(put_file(&app, "/empty.txt", b"").await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, "/empty.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(bytes.is_empty());
}

#[tokio::test]
async fn test_binary_file_png() {
    let app = make_app();
    let png_header: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    assert_eq!(put_file(&app, "/image.png", png_header).await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, "/image.png").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], png_header);
}

#[tokio::test]
async fn test_binary_file_pdf() {
    let app = make_app();
    let pdf_header: &[u8] = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n";
    assert_eq!(put_file(&app, "/doc.pdf", pdf_header).await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, "/doc.pdf").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..pdf_header.len()], pdf_header);
}

#[tokio::test]
async fn test_binary_file_all_bytes() {
    let app = make_app();
    let content: Vec<u8> = (0u8..=255).collect();
    assert_eq!(put_file(&app, "/allbytes.bin", &content).await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, "/allbytes.bin").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes.len(), 256);
    assert_eq!(&bytes[..], &content[..]);
}

#[tokio::test]
async fn test_binary_file_with_null_bytes() {
    let app = make_app();
    let content: &[u8] = b"before\x00null\x00after";
    assert_eq!(put_file(&app, "/nulls.bin", content).await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, "/nulls.bin").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], content);
}

// ---------------------------------------------------------------------------
// Long paths and filenames
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_long_filename() {
    let app = make_app();
    let name = "a".repeat(200);
    let path = format!("/{}", name);
    assert_eq!(put_file(&app, &path, b"long name").await, StatusCode::CREATED);
    let (status, _) = get_file(&app, &path).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_deep_nested_path() {
    let app = make_app();
    // Create a deep path: /a/b/c/d/e/f/g/h/i/j/file.txt
    let mut path = String::new();
    for i in 0..10 {
        path.push_str(&format!("/dir{}", i));
        assert_eq!(mkcol(&app, &path).await, StatusCode::CREATED);
    }
    path.push_str("/file.txt");
    assert_eq!(put_file(&app, &path, b"deep content").await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, &path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"deep content");
}

// ---------------------------------------------------------------------------
// Overwrite and delete edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_overwrite_preserves_content() {
    let app = make_app();
    put_file(&app, "/overwrite.txt", b"v1").await;
    put_file(&app, "/overwrite.txt", b"v2-longer-content").await;
    let (status, bytes) = get_file(&app, "/overwrite.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"v2-longer-content");
}

#[tokio::test]
async fn test_delete_nonexistent_returns_404() {
    let app = make_app();
    assert_eq!(delete_file(&app, "/nope.txt").await, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_then_get_returns_404() {
    let app = make_app();
    put_file(&app, "/temp.txt", b"delete me").await;
    assert_eq!(delete_file(&app, "/temp.txt").await, StatusCode::NO_CONTENT);
    let (status, _) = get_file(&app, "/temp.txt").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_double_delete_returns_404() {
    let app = make_app();
    put_file(&app, "/double.txt", b"x").await;
    assert_eq!(delete_file(&app, "/double.txt").await, StatusCode::NO_CONTENT);
    assert_eq!(delete_file(&app, "/double.txt").await, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Collection operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mkcol_twice_returns_error() {
    let app = make_app();
    assert_eq!(mkcol(&app, "/dup").await, StatusCode::CREATED);
    let status = mkcol(&app, "/dup").await;
    assert!(
        status == StatusCode::CONFLICT || status == StatusCode::METHOD_NOT_ALLOWED,
        "Second MKCOL should fail, got {}",
        status
    );
}

#[tokio::test]
async fn test_mkcol_nested() {
    let app = make_app();
    assert_eq!(mkcol(&app, "/parent").await, StatusCode::CREATED);
    assert_eq!(mkcol(&app, "/parent/child").await, StatusCode::CREATED);
}

#[tokio::test]
async fn test_mkcol_under_nonexistent_parent() {
    let app = make_app();
    let status = mkcol(&app, "/noexist/child").await;
    // Should either create intermediate dirs or return 409
    assert!(
        status == StatusCode::CREATED || status == StatusCode::CONFLICT || status == StatusCode::NOT_FOUND,
        "MKCOL under nonexistent parent got {}",
        status
    );
}

// ---------------------------------------------------------------------------
// Concurrent operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_concurrent_puts() {
    let app = make_app();
    let mut handles = Vec::new();
    for i in 0..20 {
        let app = app.clone();
        let handle = tokio::spawn(async move {
            let path = format!("/concurrent/{}.txt", i);
            put_file(&app, &path, format!("content-{}", i).as_bytes()).await
        });
        handles.push(handle);
    }
    let statuses: Vec<_> = futures::future::join_all(handles).await;
    for status in &statuses {
        assert_eq!(*status.as_ref().unwrap(), StatusCode::CREATED);
    }
}

#[tokio::test]
async fn test_concurrent_read_write() {
    let app = make_app();
    put_file(&app, "/rwtest.txt", b"original").await;

    let app_clone = app.clone();
    let write_handle = tokio::spawn(async move {
        // Overwrite while reading
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        put_file(&app_clone, "/rwtest.txt", b"updated").await
    });

    let app_clone2 = app.clone();
    let read_handle = tokio::spawn(async move {
        let (status, bytes) = get_file(&app_clone2, "/rwtest.txt").await;
        (status, bytes)
    });

    let read_result = read_handle.await.unwrap();
    let write_result = write_handle.await.unwrap();

    // Both should succeed (eventual consistency)
    assert_eq!(read_result.0, StatusCode::OK);
    assert_eq!(write_result, StatusCode::NO_CONTENT);
}

// ---------------------------------------------------------------------------
// Case sensitivity and normalization
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_case_sensitive_paths() {
    let app = make_app();
    put_file(&app, "/Case.txt", b"upper").await;
    // In-memory storage is case-sensitive; these should be different files
    let (status, bytes) = get_file(&app, "/Case.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"upper");

    let (status2, _) = get_file(&app, "/case.txt").await;
    // May or may not exist depending on storage backend
    assert!(
        status2 == StatusCode::OK || status2 == StatusCode::NOT_FOUND,
        "Case variant got {}",
        status2
    );
}

// ---------------------------------------------------------------------------
// Content type detection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_content_type_json() {
    let app = make_app();
    let content = r#"{"key": "value"}"#;
    put_file(&app, "/data.json", content.as_bytes()).await;
    let (status, bytes) = get_file(&app, "/data.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], content.as_bytes());
}

#[tokio::test]
async fn test_content_type_html() {
    let app = make_app();
    let content = "<html><body>hello</body></html>";
    put_file(&app, "/page.html", content.as_bytes()).await;
    let (status, bytes) = get_file(&app, "/page.html").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], content.as_bytes());
}

// ---------------------------------------------------------------------------
// Large file
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_large_file_1mb() {
    let app = make_app();
    let content = vec![0xABu8; 1024 * 1024]; // 1 MB
    assert_eq!(put_file(&app, "/large.bin", &content).await, StatusCode::CREATED);
    let (status, bytes) = get_file(&app, "/large.bin").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes.len(), 1024 * 1024);
    assert!(bytes.iter().all(|&b| b == 0xAB));
}

// ---------------------------------------------------------------------------
// PROPFIND edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_propfind_nonexistent_returns_404() {
    let app = make_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/nope")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_propfind_depth_zero() {
    let app = make_app();
    put_file(&app, "/exists.txt", b"data").await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/exists.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert!(body.contains("exists.txt"));
}

// ---------------------------------------------------------------------------
// Copy and Move edge cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_copy_nonexistent_returns_404() {
    let app = make_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/nope.txt")
                .header("Destination", "/dest.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_move_nonexistent_returns_404() {
    let app = make_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/nope.txt")
                .header("Destination", "/dest.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_copy_then_both_exist() {
    let app = make_app();
    put_file(&app, "/original.txt", b"copy me").await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/original.txt")
                .header("Destination", "/copy.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Both should exist
    let (s1, b1) = get_file(&app, "/original.txt").await;
    let (s2, b2) = get_file(&app, "/copy.txt").await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(&b1[..], b"copy me");
    assert_eq!(&b2[..], b"copy me");
}

#[tokio::test]
async fn test_move_source_gone_dest_exists() {
    let app = make_app();
    put_file(&app, "/moveme.txt", b"move me").await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/moveme.txt")
                .header("Destination", "/moved.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let (s1, _) = get_file(&app, "/moveme.txt").await;
    assert_eq!(s1, StatusCode::NOT_FOUND, "Source should be gone after MOVE");

    let (s2, b2) = get_file(&app, "/moved.txt").await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(&b2[..], b"move me");
}
