use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::make_app;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

// ── CalDAV Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_caldav_mkcalendar() {
    let app = make_app();

    // MKCALENDAR must reach the WebDAV handler via the fallback (no explicit
    // route on this path), which dispatches to the CalDAV handler.
    let resp = app
        .oneshot(
            Request::builder()
                .method("MKCALENDAR")
                .uri("/dav/cal/mkcal-test/sub")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "MKCALENDAR should return 201 Created"
    );
    let location = resp
        .headers()
        .get("location")
        .expect("Location header should be set")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        location.starts_with("/dav/cal/"),
        "Location should start with /dav/cal/, got: {}",
        location
    );
    assert!(
        location.ends_with('/'),
        "Location should end with /, got: {}",
        location
    );
}

#[tokio::test]
async fn test_caldav_propfind_depth1() {
    let app = make_app();

    // Create a WebDAV directory (via fallback -> WebDAV handler)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/test-profind-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Create files in the directory
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/test-profind-dir/file1.txt")
                .body(Body::from("hello"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/test-profind-dir/file2.txt")
                .body(Body::from("world"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // PROPFIND depth 1
    let resp = app
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/test-profind-dir")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert!(
        body.contains("D:multistatus"),
        "Should contain D:multistatus"
    );
    assert!(body.contains("file1.txt"), "Should list file1.txt");
    assert!(body.contains("file2.txt"), "Should list file2.txt");
}

#[tokio::test]
async fn test_caldav_put_vevent() {
    let app = make_app();

    // Create calendar via explicit PUT route on /dav/cal/
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/dav/cal/")
                .body(Body::empty())
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
    let calendar_id = location
        .trim_start_matches("/dav/cal/")
        .trim_end_matches('/')
        .to_string();

    let uid = "test-vevent-001";
    let ical_data = "\
BEGIN:VCALENDAR\r
VERSION:2.0\r
PRODID:-//Ferro//Test//EN\r
BEGIN:VEVENT\r
UID:test-vevent-001\r
DTSTART:20260701T100000Z\r
DTEND:20260701T110000Z\r
SUMMARY:Test Meeting\r
END:VEVENT\r
END:VCALENDAR\r\n";

    let uri = format!("/dav/cal/{}/{}.ics", calendar_id, uid);
    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&uri)
                .header("Content-Type", "text/calendar; charset=utf-8")
                .body(Body::from(ical_data.as_bytes().to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "PUT VEVENT should return 201 Created"
    );
    assert!(
        resp.headers().contains_key("etag"),
        "Response should contain ETag header"
    );
}

#[tokio::test]
async fn test_caldav_report_calendar_multiget() {
    let app = make_app();

    // Create calendar
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/dav/cal/")
                .body(Body::empty())
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
    let calendar_id = location
        .trim_start_matches("/dav/cal/")
        .trim_end_matches('/')
        .to_string();

    let uid = "multiget-event-001";
    let ical_data = "\
BEGIN:VCALENDAR\r
VERSION:2.0\r
PRODID:-//Ferro//Test//EN\r
BEGIN:VEVENT\r
UID:multiget-event-001\r
DTSTART:20260701T100000Z\r
DTEND:20260701T110000Z\r
SUMMARY:Multiget Test\r
END:VEVENT\r
END:VCALENDAR\r\n";

    // Create event via explicit PUT route
    let uri = format!("/dav/cal/{}/{}.ics", calendar_id, uid);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&uri)
                .body(Body::from(ical_data.as_bytes().to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // REPORT calendar-multiget via fallback -> WebDAV -> CalDAV dispatch
    let href = format!("/dav/cal/{}/{}.ics", calendar_id, uid);
    let report_body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<C:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <D:href>{}</D:href>
</C:calendar-multiget>"#,
        href
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method("REPORT")
                .uri("/dav/cal/report-test/sub")
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(Body::from(report_body.into_bytes()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::MULTI_STATUS,
        "REPORT should return 207 Multi-Status"
    );
    let body = body_string(resp).await;
    eprintln!(
        "REPORT body (first 2000 chars): {}",
        &body[..body.len().min(2000)]
    );
    assert!(
        body.contains("D:multistatus"),
        "Should contain D:multistatus"
    );
    assert!(
        body.contains("multiget-event-001"),
        "Should contain event UID"
    );
    assert!(
        body.contains("C:calendar-data"),
        "Should contain calendar-data"
    );
    assert!(
        body.contains("HTTP/1.1 200 OK"),
        "Should have a 200 status for the event href"
    );
}

#[tokio::test]
async fn test_caldav_delete_event() {
    let app = make_app();

    // Create calendar
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/dav/cal/")
                .body(Body::empty())
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
    let calendar_id = location
        .trim_start_matches("/dav/cal/")
        .trim_end_matches('/')
        .to_string();

    let uid = "delete-event-001";
    let ical_data = "\
BEGIN:VCALENDAR\r
VERSION:2.0\r
PRODID:-//Ferro//Test//EN\r
BEGIN:VEVENT\r
UID:delete-event-001\r
DTSTART:20260701T100000Z\r
DTEND:20260701T110000Z\r
SUMMARY:To Be Deleted\r
END:VEVENT\r
END:VCALENDAR\r\n";

    let uri = format!("/dav/cal/{}/{}.ics", calendar_id, uid);

    // Create event
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&uri)
                .body(Body::from(ical_data.as_bytes().to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Delete event
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "DELETE should return 204"
    );

    // Verify GET returns 404
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "GET after DELETE should return 404"
    );
}

// ── CardDAV Tests ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_carddav_mkcol() {
    let app = make_app();

    // MKCOL on a CardDAV-path via fallback -> WebDAV handler
    let resp = app
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/dav/card/mkcol-test/sub")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "MKCOL should return 201 Created"
    );
}

#[tokio::test]
async fn test_carddav_put_vcard() {
    let app = make_app();

    // Create address book via explicit PUT route on /dav/card/
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/dav/card/")
                .body(Body::empty())
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
    let book_id = location
        .trim_start_matches("/dav/card/")
        .trim_end_matches('/')
        .to_string();

    let uid = "test-contact-001";
    let vcard_data = "\
BEGIN:VCARD\r
VERSION:3.0\r
FN:John Doe\r
N:Doe;John;;;\r
EMAIL:john.doe@example.com\r
TEL:+1-555-0100\r
END:VCARD\r\n";

    let uri = format!("/dav/card/{}/{}.vcf", book_id, uid);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&uri)
                .header("Content-Type", "text/vcard; charset=utf-8")
                .body(Body::from(vcard_data.as_bytes().to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "PUT VCARD should return 201 Created"
    );
    assert!(
        resp.headers().contains_key("etag"),
        "Response should contain ETag header"
    );

    // Verify GET returns the vcard
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = String::from_utf8(body_bytes(resp).await.to_vec()).unwrap();
    assert!(body.contains("BEGIN:VCARD"), "Should contain VCARD data");
    assert!(body.contains("John Doe"), "Should contain contact name");
}

#[tokio::test]
async fn test_carddav_report_addressbook_multiget() {
    let app = make_app();

    // Create address book
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/dav/card/")
                .body(Body::empty())
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
    let book_id = location
        .trim_start_matches("/dav/card/")
        .trim_end_matches('/')
        .to_string();

    let uid = "multiget-contact-001";
    let vcard_data = "\
BEGIN:VCARD\r
VERSION:3.0\r
FN:Jane Smith\r
N:Smith;Jane;;;\r
EMAIL:jane.smith@example.com\r
END:VCARD\r\n";

    // Create contact
    let uri = format!("/dav/card/{}/{}.vcf", book_id, uid);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&uri)
                .body(Body::from(vcard_data.as_bytes().to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // REPORT addressbook-multiget via fallback -> WebDAV -> CardDAV dispatch
    let href = format!("/dav/card/{}/{}.vcf", book_id, uid);
    let report_body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<A:addressbook-multiget xmlns:D="DAV:" xmlns:A="urn:ietf:params:xml:ns:carddav">
  <D:prop>
    <D:getetag/>
    <A:address-data/>
  </D:prop>
  <D:href>{}</D:href>
</A:addressbook-multiget>"#,
        href
    );

    let resp = app
        .oneshot(
            Request::builder()
                .method("REPORT")
                .uri("/dav/card/report-test/sub")
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(Body::from(report_body.into_bytes()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::MULTI_STATUS,
        "REPORT should return 207 Multi-Status"
    );
    let body = body_string(resp).await;
    eprintln!(
        "CardDAV REPORT body (first 2000 chars): {}",
        &body[..body.len().min(2000)]
    );
    assert!(
        body.contains("D:multistatus"),
        "Should contain D:multistatus"
    );
    assert!(
        body.contains("multiget-contact-001"),
        "Should contain contact UID"
    );
    assert!(
        body.contains("A:address-data"),
        "Should contain address-data"
    );
    // The address-data value may contain the full vCard payload
    // Check that at least the contact appears in a 200 response
    assert!(
        body.contains("HTTP/1.1 200 OK"),
        "Should have a 200 status for the contact href"
    );
}
