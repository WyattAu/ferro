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

#[tokio::test]
async fn test_propfind_root_returns_multistatus_with_collection() {
    let app = make_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MULTI_STATUS);

    let ct = response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert!(
        ct.contains("application/xml"),
        "Expected application/xml content type, got {}",
        ct
    );

    let xml = body_string(response).await;
    assert!(xml.contains("<D:multistatus"), "Response should be a multistatus XML");
    assert!(
        xml.contains("<D:collection/>"),
        "Root should be reported as a collection"
    );
    assert!(
        xml.contains("<D:resourcetype>"),
        "Response should include resourcetype element"
    );
}

#[tokio::test]
async fn test_mkcol_creates_directory_201() {
    let app = make_app();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/newdir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/newdir")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MULTI_STATUS);
    let xml = body_string(response).await;
    assert!(
        xml.contains("<D:collection/>"),
        "MKCOL-created path should be a collection"
    );
}

#[tokio::test]
async fn test_put_file_returns_201() {
    let app = make_app();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/hello.txt")
                .header("Content-Type", "text/plain")
                .body(Body::from("hello ferro"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(
        response.headers().get("ETag").is_some(),
        "PUT response should include ETag"
    );
}

#[tokio::test]
async fn test_get_file_returns_content() {
    let app = make_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/getme.txt")
                .header("Content-Type", "text/plain")
                .body(Body::from("retrievable content"))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/getme.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_bytes(response).await;
    assert_eq!(&body[..], b"retrievable content");
}

#[tokio::test]
async fn test_delete_then_get_returns_404() {
    let app = make_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/deleteme.txt")
                .body(Body::from("temporary"))
                .unwrap(),
        )
        .await
        .unwrap();

    let del = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/deleteme.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del.status(), StatusCode::NO_CONTENT);

    let get = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/deleteme.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_move_file_old_404_new_200() {
    let app = make_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/src.txt")
                .body(Body::from("move me"))
                .unwrap(),
        )
        .await
        .unwrap();

    let move_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/src.txt")
                .header("Destination", "/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(move_resp.status(), StatusCode::CREATED);
    assert_eq!(
        move_resp.headers().get("Location").unwrap().to_str().unwrap(),
        "/dst.txt"
    );

    let old = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/src.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old.status(), StatusCode::NOT_FOUND);

    let new = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(new.status(), StatusCode::OK);
    let body = body_bytes(new).await;
    assert_eq!(&body[..], b"move me");
}

#[tokio::test]
async fn test_copy_file_both_exist() {
    let app = make_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/original.txt")
                .body(Body::from("copy source"))
                .unwrap(),
        )
        .await
        .unwrap();

    let copy_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/original.txt")
                .header("Destination", "/duplicate.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(copy_resp.status(), StatusCode::CREATED);

    let src = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/original.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(src.status(), StatusCode::OK);
    let src_body = body_bytes(src).await;
    assert_eq!(&src_body[..], b"copy source");

    let dst = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/duplicate.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dst.status(), StatusCode::OK);
    let dst_body = body_bytes(dst).await;
    assert_eq!(&dst_body[..], b"copy source");
}

#[tokio::test]
async fn test_put_get_roundtrip_content_matches() {
    let app = make_app();

    let content = b"The quick brown fox jumps over the lazy dog.\nLine two.\nLine three.";

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/roundtrip.bin")
                .header("Content-Type", "application/octet-stream")
                .body(Body::from(content.as_ref()))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/roundtrip.bin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let cl = response.headers().get("Content-Length").unwrap().to_str().unwrap();
    assert_eq!(
        cl,
        content.len().to_string().as_str(),
        "Content-Length should match uploaded size"
    );

    let etag = response.headers().get("ETag").unwrap().to_str().unwrap().to_string();

    let body = body_bytes(response).await;
    assert_eq!(
        &body[..],
        content.as_ref(),
        "Downloaded content must match uploaded content"
    );

    let cond = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/roundtrip.bin")
                .header("If-None-Match", &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        cond.status(),
        StatusCode::NOT_MODIFIED,
        "Conditional GET with matching ETag should return 304"
    );
}

#[tokio::test]
async fn test_propfind_depth_infinity_lists_recursively() {
    let app = make_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/tree")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/tree/sub")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/tree/a.txt")
                .body(Body::from("a"))
                .unwrap(),
        )
        .await
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/tree/sub/b.txt")
                .body(Body::from("b"))
                .unwrap(),
        )
        .await
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/tree/sub/c.txt")
                .body(Body::from("c"))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/tree")
                .header("Depth", "infinity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MULTI_STATUS);
    let xml = body_string(response).await;

    assert!(xml.contains("/tree/a.txt"), "Depth:infinity should list /tree/a.txt");
    assert!(
        xml.contains("/tree/sub/b.txt"),
        "Depth:infinity should list /tree/sub/b.txt"
    );
    assert!(
        xml.contains("/tree/sub/c.txt"),
        "Depth:infinity should list /tree/sub/c.txt"
    );
    assert!(
        xml.contains("/tree/sub"),
        "Depth:infinity should list /tree/sub collection"
    );

    let count = xml.matches("<D:response>").count();
    assert!(
        count >= 4,
        "Expected at least 4 responses (tree, sub, a.txt, b.txt, c.txt), got {}",
        count
    );
}

#[tokio::test]
async fn test_put_overwrite_updates_content() {
    let app = make_app();

    let put1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/overwrite.txt")
                .body(Body::from("original content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put1.status(), StatusCode::CREATED);
    let etag1 = put1.headers().get("ETag").unwrap().to_str().unwrap().to_string();

    let get1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/overwrite.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get1.status(), StatusCode::OK);
    let body1 = body_bytes(get1).await;
    assert_eq!(&body1[..], b"original content");

    let put2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/overwrite.txt")
                .body(Body::from("updated content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        put2.status(),
        StatusCode::NO_CONTENT,
        "Overwrite PUT should return 204 No Content"
    );
    let etag2 = put2.headers().get("ETag").unwrap().to_str().unwrap().to_string();
    assert_ne!(etag1, etag2, "ETag must change after overwrite");

    let get2 = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/overwrite.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get2.status(), StatusCode::OK);
    let body2 = body_bytes(get2).await;
    assert_eq!(&body2[..], b"updated content", "Content should reflect the overwrite");
}
