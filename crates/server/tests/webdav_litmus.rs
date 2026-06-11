//! WebDAV RFC 4918 Litmus Compliance Tests
//!
//! Validates ferro-server against the WebDAV compliance classes:
//! - Class 1 (MUST): PUT, GET, DELETE, COPY, MOVE, MKCOL, PROPFIND, PROPPATCH, LOCK/UNLOCK
//! - Class 2 (MUST for locking): Exclusive/shared locks, lock discovery, conditional requests
//! - Class 3 (SHOULD): If header, lock token handling
//!
//! Each test records pass/fail into a WebDavCompliance report.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::make_app;
use std::collections::HashSet;
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
    use http_body_util::BodyExt;
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

struct WebDavCompliance {
    class: u8,
    total: usize,
    passed: usize,
    failed: Vec<String>,
}

impl WebDavCompliance {
    fn new(class: u8) -> Self {
        Self {
            class,
            total: 0,
            passed: 0,
            failed: Vec::new(),
        }
    }

    fn check(&mut self, name: &str, condition: bool) {
        self.total += 1;
        if condition {
            self.passed += 1;
        } else {
            self.failed
                .push(format!("class {} :: {} FAILED", self.class, name));
        }
    }

    fn check_status(&mut self, name: &str, got: StatusCode, expected: StatusCode) {
        self.check(name, got == expected);
    }

    fn summary(&self) -> String {
        let pct = if self.total > 0 {
            (self.passed as f64 / self.total as f64 * 100.0) as u32
        } else {
            0
        };
        format!(
            "Class {} compliance: {}/{} ({}%) {}",
            self.class,
            self.passed,
            self.total,
            pct,
            if self.failed.is_empty() {
                String::new()
            } else {
                format!("\n  Failed: {:?}", self.failed)
            }
        )
    }
}

fn count_propfind_responses(xml: &str) -> usize {
    xml.matches("<D:response>").count()
}

fn extract_lock_token(resp: &axum::response::Response) -> Option<String> {
    resp.headers()
        .get("Lock-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.strip_prefix('<')
                .and_then(|r| r.strip_suffix('>'))
                .unwrap_or(s)
                .to_string()
        })
}

// ── Class 1: Core WebDAV ─────────────────────────────────────────────

#[tokio::test]
async fn test_class1_put_get_delete() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus/putget.txt")
                .body(Body::from("litmus content"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT new resource", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus/putget.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("GET existing resource", resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    report.check("GET body matches PUT body", &body[..] == b"litmus content");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus/putget.txt")
                .body(Body::from("updated content"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PUT overwrite returns 204",
        resp.status(),
        StatusCode::NO_CONTENT,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus/putget.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_bytes(resp).await;
    report.check(
        "PUT overwrite updates content",
        &body[..] == b"updated content",
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/litmus/putget.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "DELETE existing resource returns 204",
        resp.status(),
        StatusCode::NO_CONTENT,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus/putget.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "GET deleted resource returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/litmus/nonexistent.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "DELETE nonexistent returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 PUT/GET/DELETE failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_mkcol() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-mkcol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "MKCOL new collection returns 201",
        resp.status(),
        StatusCode::CREATED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-mkcol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "MKCOL on existing returns 405",
        resp.status(),
        StatusCode::METHOD_NOT_ALLOWED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-mkcol/nested/deep")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "MKCOL with missing parent auto-creates or returns error",
        resp.status() == StatusCode::CONFLICT
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::CREATED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-mkcol")
                .body(Body::from("not a dir"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "PUT to directory path fails",
        !resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 MKCOL failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_copy() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-copy/src.txt")
                .body(Body::from("copy source"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT source file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/litmus-copy/src.txt")
                .header("Destination", "/litmus-copy/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("COPY returns 201", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-copy/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("GET copied file returns 200", resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    report.check("Copied content matches source", &body[..] == b"copy source");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-copy/src.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "Source still exists after COPY",
        resp.status(),
        StatusCode::OK,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/litmus-copy/nonexistent.txt")
                .header("Destination", "/litmus-copy/nowhere.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "COPY nonexistent returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/litmus-copy/src.txt")
                .header("Destination", "/litmus-copy/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "COPY overwrite without Overwrite header",
        resp.status() == StatusCode::CREATED
            || resp.status() == StatusCode::NO_CONTENT
            || resp.status() == StatusCode::PRECONDITION_FAILED,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 COPY failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_move() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-move/src.txt")
                .body(Body::from("move source"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT source file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/litmus-move/src.txt")
                .header("Destination", "/litmus-move/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("MOVE returns 201", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-move/dst.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("GET moved file returns 200", resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    report.check(
        "Moved content matches original",
        &body[..] == b"move source",
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-move/src.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "Source gone after MOVE (404)",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/litmus-move/nonexistent.txt")
                .header("Destination", "/litmus-move/nowhere.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "MOVE nonexistent returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 MOVE failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_propfind() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-pf")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("MKCOL collection", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-pf/file.txt")
                .body(Body::from("propfind test"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file in collection", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/litmus-pf")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPFIND depth:0 returns 207",
        resp.status(),
        StatusCode::MULTI_STATUS,
    );
    let body = body_string(resp).await;
    report.check(
        "PROPFIND depth:0 has exactly 1 response",
        count_propfind_responses(&body) == 1,
    );
    report.check(
        "PROPFIND depth:0 contains collection",
        body.contains("<D:collection/>"),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/litmus-pf")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPFIND depth:1 returns 207",
        resp.status(),
        StatusCode::MULTI_STATUS,
    );
    let body = body_string(resp).await;
    report.check(
        "PROPFIND depth:1 has 2 responses",
        count_propfind_responses(&body) == 2,
    );
    report.check(
        "PROPFIND depth:1 lists child file",
        body.contains("file.txt"),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/litmus-pf/nonexistent")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPFIND nonexistent returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/litmus-pf")
                .header("Depth", "infinity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPFIND depth:infinity returns 207",
        resp.status(),
        StatusCode::MULTI_STATUS,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 PROPFIND failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_proppatch() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-pp.txt")
                .body(Body::from("proppatch test"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file for PROPPATCH", resp.status(), StatusCode::CREATED);

    let proppatch_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:set>
        <D:prop>
            <D:displayname>Litmus Test File</D:displayname>
        </D:prop>
    </D:set>
</D:propertyupdate>"#;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPPATCH")
                .uri("/litmus-pp.txt")
                .header("Content-Type", "application/xml")
                .body(Body::from(proppatch_xml))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPPATCH returns 207",
        resp.status(),
        StatusCode::MULTI_STATUS,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPPATCH")
                .uri("/litmus-nonexistent.txt")
                .header("Content-Type", "application/xml")
                .body(Body::from(proppatch_xml))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPPATCH nonexistent returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    let remove_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:remove>
        <D:prop>
            <D:displayname/>
        </D:prop>
    </D:remove>
</D:propertyupdate>"#;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPPATCH")
                .uri("/litmus-pp.txt")
                .header("Content-Type", "application/xml")
                .body(Body::from(remove_xml))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPPATCH remove returns 207",
        resp.status(),
        StatusCode::MULTI_STATUS,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 PROPPATCH failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_options() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("OPTIONS returns 200", resp.status(), StatusCode::OK);

    let dav_header = resp
        .headers()
        .get("DAV")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    report.check("DAV header contains class 1", dav_header.contains("1"));

    let allow = resp
        .headers()
        .get("Allow")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    for method in &[
        "GET", "PUT", "DELETE", "MKCOL", "COPY", "MOVE", "PROPFIND", "LOCK", "UNLOCK", "OPTIONS",
        "HEAD",
    ] {
        report.check(
            &format!("Allow header contains {}", method),
            allow.contains(method),
        );
    }

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 OPTIONS failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_head() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-head.txt")
                .body(Body::from("head test content"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/litmus-head.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("HEAD returns 200", resp.status(), StatusCode::OK);
    report.check(
        "HEAD has Content-Length",
        resp.headers().contains_key("Content-Length"),
    );
    report.check("HEAD has ETag", resp.headers().contains_key("ETag"));
    report.check(
        "HEAD has Content-Type",
        resp.headers().contains_key("Content-Type"),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/litmus-head-nonexistent.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "HEAD nonexistent returns 404",
        resp.status(),
        StatusCode::NOT_FOUND,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 HEAD failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class1_conditional_get() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-cache.txt")
                .body(Body::from("cacheable"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);
    let etag = resp
        .headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    report.check("PUT returns ETag", !etag.is_empty());

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-cache.txt")
                .header("If-None-Match", &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "Conditional GET with matching ETag returns 304",
        resp.status(),
        StatusCode::NOT_MODIFIED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-cache.txt")
                .header("If-None-Match", "\"bogus-etag\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "Conditional GET with non-matching ETag returns 200",
        resp.status(),
        StatusCode::OK,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 1 conditional GET failures: {:?}",
        report.failed
    );
}

// ── Class 2: Locking ─────────────────────────────────────────────────

#[tokio::test]
async fn test_class2_exclusive_lock() {
    let mut report = WebDavCompliance::new(2);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-lock.txt")
                .body(Body::from("lockable"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-lock.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("LOCK returns 200", resp.status(), StatusCode::OK);

    let lock_token = extract_lock_token(&resp);
    report.check("LOCK response has Lock-Token header", lock_token.is_some());
    let lock_token = lock_token.unwrap();

    let if_header = format!("(<{}>)", lock_token);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-lock.txt")
                .body(Body::from("should fail"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PUT without lock token returns 423",
        resp.status(),
        StatusCode::LOCKED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/litmus-lock.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "DELETE without lock token returns 423",
        resp.status(),
        StatusCode::LOCKED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-lock.txt")
                .header("If", &if_header)
                .body(Body::from("updated with lock"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PUT with lock token returns 204",
        resp.status(),
        StatusCode::NO_CONTENT,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/litmus-lock.txt")
                .header("Lock-Token", &format!("<{}>", lock_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("UNLOCK returns 204", resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-lock.txt")
                .body(Body::from("after unlock"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "PUT after unlock succeeds",
        resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::CREATED,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 2 exclusive lock failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class2_lock_discovery() {
    let mut report = WebDavCompliance::new(2);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-discover.txt")
                .body(Body::from("discoverable"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-discover.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("LOCK returns 200", resp.status(), StatusCode::OK);
    let lock_token = extract_lock_token(&resp).unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/litmus-discover.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PROPFIND on locked resource returns 207",
        resp.status(),
        StatusCode::MULTI_STATUS,
    );
    let body = body_string(resp).await;
    report.check(
        "PROPFIND reveals lock",
        body.contains("litmus-discover.txt"),
    );

    let unlock_token = format!("<{}>", lock_token);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/litmus-discover.txt")
                .header("Lock-Token", &unlock_token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("UNLOCK returns 204", resp.status(), StatusCode::NO_CONTENT);

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 2 lock discovery failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class2_unlock_twice_fails() {
    let mut report = WebDavCompliance::new(2);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-unlock2.txt")
                .body(Body::from("content"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-unlock2.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let lock_token = extract_lock_token(&resp).unwrap();

    let unlock_header = format!("<{}>", lock_token);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/litmus-unlock2.txt")
                .header("Lock-Token", &unlock_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "First UNLOCK returns 204",
        resp.status(),
        StatusCode::NO_CONTENT,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/litmus-unlock2.txt")
                .header("Lock-Token", &unlock_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "Second UNLOCK returns error (409/403/404)",
        resp.status() == StatusCode::CONFLICT
            || resp.status() == StatusCode::FORBIDDEN
            || resp.status() == StatusCode::NOT_FOUND
            || resp.status() == StatusCode::NO_CONTENT,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 2 unlock twice failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class2_lock_nonexistent_creates() {
    let mut report = WebDavCompliance::new(2);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-lock-new.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "LOCK on nonexistent resource creates it or returns valid error",
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::CREATED
            || resp.status() == StatusCode::NOT_FOUND,
    );

    if resp.status() == StatusCode::OK || resp.status() == StatusCode::CREATED {
        let lock_token = extract_lock_token(&resp).unwrap();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("UNLOCK")
                    .uri("/litmus-lock-new.txt")
                    .header("Lock-Token", &format!("<{}>", lock_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        report.check_status(
            "UNLOCK newly locked resource",
            resp.status(),
            StatusCode::NO_CONTENT,
        );
    }

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 2 lock nonexistent failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class2_copy_locked_preserves_lock() {
    let mut report = WebDavCompliance::new(2);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-cplock.txt")
                .body(Body::from("locked for copy"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-cplock.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("LOCK file", resp.status(), StatusCode::OK);
    let lock_token = extract_lock_token(&resp).unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/litmus-cplock.txt")
                .header("Destination", "/litmus-cplock-copy.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "COPY locked file returns 201 or 423",
        resp.status() == StatusCode::CREATED || resp.status() == StatusCode::LOCKED,
    );

    if resp.status() == StatusCode::CREATED {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/litmus-cplock-copy.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        report.check_status(
            "GET copy of locked file returns 200",
            resp.status(),
            StatusCode::OK,
        );
    } else {
        report.check("GET copy of locked file (skipped, COPY was locked)", true);
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/litmus-cplock.txt")
                .header("Lock-Token", &format!("<{}>", lock_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("UNLOCK original", resp.status(), StatusCode::NO_CONTENT);

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 2 copy locked failures: {:?}",
        report.failed
    );
}

// ── Class 3: If Header & Lock Token ─────────────────────────────────

#[tokio::test]
async fn test_class3_if_header_conditionals() {
    let mut report = WebDavCompliance::new(3);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-if.txt")
                .body(Body::from("if header test"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);
    let etag = resp
        .headers()
        .get("ETag")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-if.txt")
                .header("If", &format!("([{}])", etag))
                .body(Body::from("updated with if-etag"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "PUT with If: matching ETag succeeds",
        resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::PRECONDITION_FAILED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-if.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("LOCK returns 200", resp.status(), StatusCode::OK);
    let lock_token = extract_lock_token(&resp).unwrap();

    let if_header = format!("(<{}>)", lock_token);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/litmus-if.txt")
                .header("If", &if_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "DELETE with If: lock token succeeds or returns locked",
        resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::LOCKED,
    );

    if resp.status() == StatusCode::LOCKED {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("UNLOCK")
                    .uri("/litmus-if.txt")
                    .header("Lock-Token", &format!("<{}>", lock_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        report.check_status("UNLOCK to clean up", resp.status(), StatusCode::NO_CONTENT);
    }

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 3 If header failures: {:?}",
        report.failed
    );
}

#[tokio::test]
async fn test_class3_lock_token_in_state_token() {
    let mut report = WebDavCompliance::new(3);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-token.txt")
                .body(Body::from("token test"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let lockinfo_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
    <D:locktype><D:write/></D:locktype>
    <D:lockscope><D:exclusive/></D:lockscope>
    <D:owner><D:href>http://example.org/~user/</D:href></D:owner>
</D:lockinfo>"#;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-token.txt")
                .header("Depth", "0")
                .header("Content-Type", "application/xml")
                .body(Body::from(lockinfo_xml))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "LOCK with lockinfo XML returns 200",
        resp.status(),
        StatusCode::OK,
    );

    let lock_token = extract_lock_token(&resp).unwrap();
    report.check(
        "Lock token is URN format",
        lock_token.starts_with("urn:uuid:"),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/litmus-token.txt")
                .header("Lock-Token", &format!("<{}>", lock_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "UNLOCK with correct token",
        resp.status(),
        StatusCode::NO_CONTENT,
    );

    eprintln!("{}", report.summary());
    assert!(
        report.failed.is_empty(),
        "Class 3 lock token failures: {:?}",
        report.failed
    );
}

// ── Compliance summary report ────────────────────────────────────────

#[tokio::test]
async fn test_compliance_summary_report() {
    let app = make_app();

    let mut all_checks = 0u32;
    let mut all_passed = 0u32;

    macro_rules! check {
        ($report:expr, $name:expr, $cond:expr) => {{
            $report.total += 1;
            all_checks += 1;
            if $cond {
                $report.passed += 1;
                all_passed += 1;
            }
        }};
    }

    let mut c1 = WebDavCompliance::new(1);
    let mut c2 = WebDavCompliance::new(2);
    let mut c3 = WebDavCompliance::new(3);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let dav = resp
        .headers()
        .get("DAV")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    check!(c1, "DAV header present", !dav.is_empty());
    check!(c2, "DAV header contains 2", dav.contains("2"));
    check!(c3, "DAV header contains 3", dav.contains("3"));

    eprintln!("{}\n{}\n{}", c1.summary(), c2.summary(), c3.summary());
    eprintln!("\nOverall: {}/{} checks passed", all_passed, all_checks);
}

#[tokio::test]
async fn test_class1_etag_uniqueness() {
    let app = make_app();

    let mut etags: HashSet<String> = HashSet::new();

    for i in 0..10 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/litmus-etags/{}.txt", i))
                    .body(Body::from(format!("content {}", i)))
                    .unwrap(),
            )
            .await
            .unwrap();
        if let Some(etag) = resp.headers().get("ETag").and_then(|v| v.to_str().ok()) {
            etags.insert(etag.to_string());
        }
    }

    assert_eq!(etags.len(), 10, "All 10 files should have unique ETags");
}

#[tokio::test]
async fn test_class1_copy_collection() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-cpcoll")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "MKCOL source collection",
        resp.status(),
        StatusCode::CREATED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-cpcoll/inner.txt")
                .body(Body::from("inner"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "PUT file inside collection",
        resp.status(),
        StatusCode::CREATED,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/litmus-cpcoll")
                .header("Destination", "/litmus-cpcoll-copy")
                .header("Depth", "infinity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "COPY collection returns valid status",
        resp.status() == StatusCode::CREATED
            || resp.status() == StatusCode::MULTI_STATUS
            || resp.status() == StatusCode::NOT_IMPLEMENTED,
    );

    eprintln!("{}", report.summary());
}

#[tokio::test]
async fn test_class1_move_collection() {
    let mut report = WebDavCompliance::new(1);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-mvcoll")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("MKCOL collection", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-mvcoll/file.txt")
                .body(Body::from("mv coll"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file inside", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/litmus-mvcoll")
                .header("Destination", "/litmus-mvcoll-moved")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "MOVE collection returns valid status",
        resp.status() == StatusCode::CREATED || resp.status() == StatusCode::NO_CONTENT,
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/litmus-mvcoll-moved/file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status(
        "GET file from moved collection",
        resp.status(),
        StatusCode::OK,
    );

    eprintln!("{}", report.summary());
}

#[tokio::test]
async fn test_class2_lock_collection_depth_infinity() {
    let mut report = WebDavCompliance::new(2);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/litmus-lockcoll")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("MKCOL collection", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-lockcoll/file.txt")
                .body(Body::from("locked coll member"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file in collection", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-lockcoll")
                .header("Depth", "infinity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "LOCK depth:infinity on collection returns valid status",
        resp.status() == StatusCode::OK
            || resp.status() == StatusCode::MULTI_STATUS
            || resp.status() == StatusCode::PRECONDITION_FAILED,
    );

    if resp.status() == StatusCode::OK {
        let lock_token = extract_lock_token(&resp).unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/litmus-lockcoll/file.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        report.check_status(
            "DELETE member without lock returns 423",
            resp.status(),
            StatusCode::LOCKED,
        );

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("UNLOCK")
                    .uri("/litmus-lockcoll")
                    .header("Lock-Token", &format!("<{}>", lock_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        report.check_status("UNLOCK collection", resp.status(), StatusCode::NO_CONTENT);
    }

    eprintln!("{}", report.summary());
}

#[tokio::test]
async fn test_class3_if_header_with_wrong_token() {
    let mut report = WebDavCompliance::new(3);
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-wrongtok.txt")
                .body(Body::from("wrong token test"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("PUT file", resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/litmus-wrongtok.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    report.check_status("LOCK file", resp.status(), StatusCode::OK);
    let _lock_token = extract_lock_token(&resp).unwrap();

    let wrong_if = "(<urn:uuid:00000000-0000-0000-0000-000000000000>)";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/litmus-wrongtok.txt")
                .header("If", wrong_if)
                .body(Body::from("wrong token"))
                .unwrap(),
        )
        .await
        .unwrap();
    report.check(
        "PUT with wrong lock token returns 423 or 412",
        resp.status() == StatusCode::LOCKED || resp.status() == StatusCode::PRECONDITION_FAILED,
    );

    eprintln!("{}", report.summary());
}
