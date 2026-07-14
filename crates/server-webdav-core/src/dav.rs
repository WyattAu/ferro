use axum::Extension;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;

use crate::WebDavCoreState;

async fn caldav_state<S: WebDavCoreState>(state: &S) -> ferro_dav::caldav::CalDavState {
    ferro_dav::caldav::CalDavState {
        store: state.calendar_store().clone(),
        principal: "default".to_string(),
    }
}

async fn carddav_state<S: WebDavCoreState>(state: &S) -> ferro_dav::carddav::CardDavState {
    ferro_dav::carddav::CardDavState {
        store: state.address_book_store().clone(),
        principal: "default".to_string(),
    }
}

/// Dispatch CalDAV-specific methods (MKCALENDAR, REPORT, GET, PUT, DELETE on
/// events) that the generic WebDAV handler doesn't support. Called from
/// webdav::handle_any when the path starts with /dav/cal/.
pub async fn dispatch_caldav<S: WebDavCoreState>(state: S, method: &Method, path: &str, body: Bytes) -> Response {
    let cal_state = caldav_state(&state).await;

    // Parse path segments to determine operation.
    // /dav/cal/                       → list/create calendar
    // /dav/cal/:calendar              → delete calendar
    // /dav/cal/:calendar/             → calendar properties (PROPFIND)
    // /dav/cal/:calendar/:event.ics   → get/put/delete event
    let segments: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let is_event_path =
        segments.len() == 4 && segments[0] == "dav" && segments[1] == "cal" && segments[3].ends_with(".ics");

    match method.as_str() {
        "MKCALENDAR" => ferro_dav::caldav::create_calendar_handler(axum::extract::State(cal_state)).await,
        "REPORT" => {
            if is_sync_collection_report(&body) {
                return handle_caldav_sync_collection(&state, &body, &segments).await;
            }
            if is_multiget_report(&body) {
                ferro_dav::caldav::handle_multiget(axum::extract::State(cal_state), Extension(body)).await
            } else {
                ferro_dav::caldav::handle_report(axum::extract::State(cal_state), Extension(body)).await
            }
        }
        "GET" if is_event_path => {
            let calendar = segments[2].to_string();
            let uid = segments[3].strip_suffix(".ics").unwrap_or(segments[3]).to_string();
            ferro_dav::caldav::get_event(axum::extract::State(cal_state), Path((calendar, uid))).await
        }
        "PUT" if is_event_path => {
            let calendar = segments[2].to_string();
            let uid = segments[3].strip_suffix(".ics").unwrap_or(segments[3]).to_string();
            ferro_dav::caldav::put_event(axum::extract::State(cal_state), Path((calendar, uid)), Extension(body)).await
        }
        "DELETE" if is_event_path => {
            let calendar = segments[2].to_string();
            let uid = segments[3].strip_suffix(".ics").unwrap_or(segments[3]).to_string();
            ferro_dav::caldav::delete_event(axum::extract::State(cal_state), Path((calendar, uid))).await
        }
        _ => axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

/// Dispatch CardDAV-specific methods (REPORT, GET, PUT, DELETE on contacts)
/// that the generic WebDAV handler doesn't support. Called from
/// webdav::handle_any when the path starts with /dav/card/.
pub async fn dispatch_carddav<S: WebDavCoreState>(state: S, method: &Method, path: &str, body: Bytes) -> Response {
    let card_state = carddav_state(&state).await;

    // Parse path segments to determine operation.
    // /dav/card/                       → list/create address book
    // /dav/card/:book                  → delete address book
    // /dav/card/:book/                 → address book properties (PROPFIND)
    // /dav/card/:book/:contact.vcf     → get/put/delete contact
    let segments: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let is_contact_path =
        segments.len() == 4 && segments[0] == "dav" && segments[1] == "card" && segments[3].ends_with(".vcf");

    match method.as_str() {
        "REPORT" => {
            if is_sync_collection_report(&body) {
                return handle_carddav_sync_collection(&state, &body, &segments).await;
            }
            if is_multiget_report(&body) {
                ferro_dav::carddav::handle_multiget(axum::extract::State(card_state), Extension(body)).await
            } else {
                ferro_dav::carddav::handle_report(axum::extract::State(card_state), Extension(body)).await
            }
        }
        "GET" if is_contact_path => {
            let book = segments[2].to_string();
            let uid = segments[3].strip_suffix(".vcf").unwrap_or(segments[3]).to_string();
            ferro_dav::carddav::get_contact(axum::extract::State(card_state), Path((book, uid))).await
        }
        "PUT" if is_contact_path => {
            let book = segments[2].to_string();
            let uid = segments[3].strip_suffix(".vcf").unwrap_or(segments[3]).to_string();
            ferro_dav::carddav::put_contact(axum::extract::State(card_state), Path((book, uid)), Extension(body)).await
        }
        "DELETE" if is_contact_path => {
            let book = segments[2].to_string();
            let uid = segments[3].strip_suffix(".vcf").unwrap_or(segments[3]).to_string();
            ferro_dav::carddav::delete_contact(axum::extract::State(card_state), Path((book, uid))).await
        }
        _ => axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

pub async fn caldav_options() -> impl IntoResponse {
    ferro_dav::caldav::options_handler().await
}

pub async fn caldav_list<S: WebDavCoreState>(State(state): State<S>) -> Response {
    ferro_dav::caldav::list_calendars(axum::extract::State(caldav_state(&state).await)).await
}

pub async fn caldav_create<S: WebDavCoreState>(State(state): State<S>) -> Response {
    ferro_dav::caldav::create_calendar_handler(axum::extract::State(caldav_state(&state).await)).await
}

pub async fn carddav_options() -> impl IntoResponse {
    ferro_dav::carddav::options_handler().await
}

pub async fn carddav_list<S: WebDavCoreState>(State(state): State<S>) -> Response {
    ferro_dav::carddav::list_address_books(axum::extract::State(carddav_state(&state).await)).await
}

pub async fn carddav_create<S: WebDavCoreState>(State(state): State<S>) -> Response {
    ferro_dav::carddav::create_address_book_handler(axum::extract::State(carddav_state(&state).await)).await
}

/// Handle a CalDAV sync-collection REPORT request.
/// Parses the sync-token from the XML body, compares against the current sync clock,
/// and returns all resources if the token is stale or missing.
async fn handle_caldav_sync_collection<S: WebDavCoreState>(state: &S, body: &Bytes, segments: &[&str]) -> Response {
    let req = match ferro_dav::xml_ext::parse_sync_collection(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid sync-collection request: {}", e),
            )
                .into_response();
        }
    };

    // Extract calendar ID from path segments: ["dav", "cal", "{calendar}", ...]
    let calendar_id = if segments.len() >= 3 { segments[2] } else { "" };

    // Get the current sync clock value
    let clock_value = state.sync_clock().load(std::sync::atomic::Ordering::SeqCst);

    // Parse the client's sync-token to get the clock value
    let client_token = req
        .sync_token
        .as_ref()
        .and_then(|t| t.rsplit('/').next().and_then(|n| n.parse::<u64>().ok()));

    // If client has a valid token that matches or exceeds current, return empty
    if let Some(token) = client_token
        && token >= clock_value
    {
        let xml = ferro_dav::xml_ext::build_dav_multistatus(&[]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/xml; charset=utf-8"),
        );
        let token_value = format!("http://ferro.local/sync/token/{}", clock_value);
        if let Ok(val) = HeaderValue::from_str(&token_value) {
            headers.insert("Sync-Token", val);
        }
        return (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response();
    }

    // Token is stale or missing: return all events in the calendar
    let cal_state = caldav_state(state).await;
    let events = cal_state.store.list_events(calendar_id).await;
    let mut responses = Vec::new();

    for event in &events {
        let mut props = Vec::new();
        if req.want_getetag {
            props.push(ferro_dav::xml_ext::DavProp {
                name: "D:getetag".to_string(),
                namespace: None,
                value: Some(event.etag.clone()),
            });
        }
        if req.want_calendar_data {
            props.push(ferro_dav::xml_ext::DavProp {
                name: "C:calendar-data".to_string(),
                namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                value: Some(event.ical_data.clone()),
            });
        }
        // Always include sync-token in each response
        props.push(ferro_dav::xml_ext::DavProp {
            name: "D:sync-token".to_string(),
            namespace: None,
            value: Some(format!("http://ferro.local/sync/token/{}", clock_value)),
        });

        responses.push(ferro_dav::xml_ext::DavResponse {
            href: format!("/dav/cal/{}/{}.ics", calendar_id, event.uid),
            propstats: vec![ferro_dav::xml_ext::PropStat { status: 200, props }],
        });
    }

    let xml = ferro_dav::xml_ext::build_dav_multistatus(&responses);
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    let token_value = format!("http://ferro.local/sync/token/{}", clock_value);
    if let Ok(val) = HeaderValue::from_str(&token_value) {
        headers.insert("Sync-Token", val);
    }
    (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response()
}

/// Handle a CardDAV sync-collection REPORT request.
/// Parses the sync-token from the XML body, compares against the current sync clock,
/// and returns all resources if the token is stale or missing.
async fn handle_carddav_sync_collection<S: WebDavCoreState>(state: &S, body: &Bytes, segments: &[&str]) -> Response {
    let req = match ferro_dav::xml_ext::parse_sync_collection(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid sync-collection request: {}", e),
            )
                .into_response();
        }
    };

    // Extract address book ID from path segments: ["dav", "card", "{book}", ...]
    let book_id = if segments.len() >= 3 { segments[2] } else { "" };

    // Get the current sync clock value
    let clock_value = state.sync_clock().load(std::sync::atomic::Ordering::SeqCst);

    // Parse the client's sync-token to get the clock value
    let client_token = req
        .sync_token
        .as_ref()
        .and_then(|t| t.rsplit('/').next().and_then(|n| n.parse::<u64>().ok()));

    // If client has a valid token that matches or exceeds current, return empty
    if let Some(token) = client_token
        && token >= clock_value
    {
        let xml = ferro_dav::xml_ext::build_dav_multistatus(&[]);
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/xml; charset=utf-8"),
        );
        let token_value = format!("http://ferro.local/sync/token/{}", clock_value);
        if let Ok(val) = HeaderValue::from_str(&token_value) {
            headers.insert("Sync-Token", val);
        }
        return (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response();
    }

    // Token is stale or missing: return all contacts in the address book
    let card_state = carddav_state(state).await;
    let contacts = card_state.store.list_contacts(book_id).await;
    let mut responses = Vec::new();

    for contact in &contacts {
        let mut props = Vec::new();
        if req.want_getetag {
            props.push(ferro_dav::xml_ext::DavProp {
                name: "D:getetag".to_string(),
                namespace: None,
                value: Some(contact.etag.clone()),
            });
        }
        if req.want_address_data {
            props.push(ferro_dav::xml_ext::DavProp {
                name: "A:address-data".to_string(),
                namespace: Some("urn:ietf:params:xml:ns:carddav".to_string()),
                value: Some(contact.vcard_data.clone()),
            });
        }
        // Always include sync-token in each response
        props.push(ferro_dav::xml_ext::DavProp {
            name: "D:sync-token".to_string(),
            namespace: None,
            value: Some(format!("http://ferro.local/sync/token/{}", clock_value)),
        });

        responses.push(ferro_dav::xml_ext::DavResponse {
            href: format!("/dav/card/{}/{}.vcf", book_id, contact.uid),
            propstats: vec![ferro_dav::xml_ext::PropStat { status: 200, props }],
        });
    }

    let xml = ferro_dav::xml_ext::build_dav_multistatus(&responses);
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    let token_value = format!("http://ferro.local/sync/token/{}", clock_value);
    if let Ok(val) = HeaderValue::from_str(&token_value) {
        headers.insert("Sync-Token", val);
    }
    (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response()
}

/// Check if a REPORT request body is a multiget request.
/// Multiget requests have root elements `calendar-multiget` or `addressbook-multiget`.
/// We verify the match is preceded by `<` to avoid false positives from adversarial
/// content containing these strings outside of XML element context.
fn is_multiget_report(body: &[u8]) -> bool {
    if body.len() < 20 || body.len() > 10 * 1024 * 1024 {
        return false;
    }
    let body_str = String::from_utf8_lossy(body);
    body_str.contains("<calendar-multiget") || body_str.contains("<addressbook-multiget")
}

/// Check if a REPORT request body is a sync-collection request.
fn is_sync_collection_report(body: &[u8]) -> bool {
    if body.is_empty() || body.len() > 10 * 1024 * 1024 {
        return false;
    }
    let body_str = String::from_utf8_lossy(body);
    body_str.contains("<sync-collection")
}

/// Unified CalDAV handler registered with `any()` on all CalDAV paths.
/// Dispatches based on method + path depth to the correct CalDAV operation.
/// This avoids matchit 0.7.3 MethodNotAllowed errors from partial route matching.
///
/// Path patterns routed here:
///   /dav/cal/:calendar         → delete calendar (DELETE), props via PROPFIND
///   /dav/cal/:calendar/        → calendar properties (GET → PROPFIND-like)
///   /dav/cal/:calendar/:uid    → event CRUD (GET/PUT/DELETE), MKCALENDAR, REPORT
pub async fn caldav_calendar_or_event<S: WebDavCoreState>(
    method: Method,
    State(state): State<S>,
    uri: axum::http::Uri,
    headers: axum::http::HeaderMap,
    body: axum::body::Body,
) -> Response {
    let path = uri.path().to_string();
    let segments: Vec<&str> = path.trim_start_matches('/').trim_end_matches('/').split('/').collect();
    // segments: ["dav", "cal", ...]  (leading '/' stripped)
    let depth = segments.len(); // 3 = calendar level, 4 = event level

    let cal_state = caldav_state(&state).await;

    // For CalDAV-specific methods, read body and dispatch.
    // For everything else (MKCOL, COPY, MOVE, LOCK…), delegate to WebDAV.
    match method.as_str() {
        "OPTIONS" => ferro_dav::caldav::options_handler().await.into_response(),
        // ── Calendar-level operations ──────────────────────────────────
        "GET" if depth == 3 => ferro_dav::caldav::list_calendars(axum::extract::State(cal_state)).await,
        "PUT" if depth == 3 => ferro_dav::caldav::create_calendar_handler(axum::extract::State(cal_state)).await,
        "DELETE" if depth == 3 => {
            let calendar = segments[2].to_string();
            ferro_dav::caldav::delete_calendar_handler(axum::extract::State(cal_state), Path(calendar)).await
        }
        "PROPFIND" => match crate::handlers::propfind::handle_propfind(state, &path, &headers).await {
            Ok(resp) => resp,
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("PROPFIND error: {}", e),
            )
                .into_response(),
        },
        // ── Event-level operations (need body) ─────────────────────────
        "MKCALENDAR" => ferro_dav::caldav::create_calendar_handler(axum::extract::State(cal_state)).await,
        "REPORT" => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (axum::http::StatusCode::BAD_REQUEST, format!("body read: {}", e)).into_response();
                }
            };
            if is_sync_collection_report(&body_bytes) {
                return handle_caldav_sync_collection(&state, &body_bytes, &segments).await;
            }
            if is_multiget_report(&body_bytes) {
                ferro_dav::caldav::handle_multiget(axum::extract::State(cal_state), Extension(body_bytes)).await
            } else {
                ferro_dav::caldav::handle_report(axum::extract::State(cal_state), Extension(body_bytes)).await
            }
        }
        "GET" if depth == 4 => {
            let calendar = segments[2].to_string();
            let uid = segments[3].strip_suffix(".ics").unwrap_or(segments[3]).to_string();
            ferro_dav::caldav::get_event(axum::extract::State(cal_state), Path((calendar, uid))).await
        }
        "PUT" if depth == 4 => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (axum::http::StatusCode::BAD_REQUEST, format!("body read: {}", e)).into_response();
                }
            };
            let calendar = segments[2].to_string();
            let uid = segments[3].strip_suffix(".ics").unwrap_or(segments[3]).to_string();
            ferro_dav::caldav::put_event(
                axum::extract::State(cal_state),
                Path((calendar, uid)),
                Extension(body_bytes),
            )
            .await
        }
        "DELETE" if depth == 4 => {
            let calendar = segments[2].to_string();
            let uid = segments[3].strip_suffix(".ics").unwrap_or(segments[3]).to_string();
            ferro_dav::caldav::delete_event(axum::extract::State(cal_state), Path((calendar, uid))).await
        }
        // ── Fall through: WebDAV operations (MKCOL, COPY, MOVE, LOCK…) ──
        _ => crate::webdav::handle_any(method, uri, State(state), None, headers, body).await,
    }
}

/// Unified CardDAV handler registered with `any()` on all CardDAV paths.
/// Same approach as caldav_calendar_or_event for consistency.
pub async fn carddav_book_or_contact<S: WebDavCoreState>(
    method: Method,
    State(state): State<S>,
    uri: axum::http::Uri,
    headers: axum::http::HeaderMap,
    body: axum::body::Body,
) -> Response {
    let path = uri.path().to_string();
    let segments: Vec<&str> = path.trim_start_matches('/').trim_end_matches('/').split('/').collect();
    let depth = segments.len(); // 3 = book level, 4 = contact level

    let card_state = carddav_state(&state).await;

    match method.as_str() {
        "OPTIONS" => ferro_dav::carddav::options_handler().await.into_response(),
        "GET" if depth == 3 => ferro_dav::carddav::list_address_books(axum::extract::State(card_state)).await,
        "PUT" if depth == 3 => ferro_dav::carddav::create_address_book_handler(axum::extract::State(card_state)).await,
        "DELETE" if depth == 3 => {
            let book = segments[2].to_string();
            ferro_dav::carddav::delete_address_book_handler(axum::extract::State(card_state), Path(book)).await
        }
        "PROPFIND" => match crate::handlers::propfind::handle_propfind(state, &path, &headers).await {
            Ok(resp) => resp,
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("PROPFIND error: {}", e),
            )
                .into_response(),
        },
        "REPORT" => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (axum::http::StatusCode::BAD_REQUEST, format!("body read: {}", e)).into_response();
                }
            };
            if is_sync_collection_report(&body_bytes) {
                return handle_carddav_sync_collection(&state, &body_bytes, &segments).await;
            }
            if is_multiget_report(&body_bytes) {
                ferro_dav::carddav::handle_multiget(axum::extract::State(card_state), Extension(body_bytes)).await
            } else {
                ferro_dav::carddav::handle_report(axum::extract::State(card_state), Extension(body_bytes)).await
            }
        }
        "GET" if depth == 4 => {
            let book = segments[2].to_string();
            let uid = segments[3].strip_suffix(".vcf").unwrap_or(segments[3]).to_string();
            ferro_dav::carddav::get_contact(axum::extract::State(card_state), Path((book, uid))).await
        }
        "PUT" if depth == 4 => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (axum::http::StatusCode::BAD_REQUEST, format!("body read: {}", e)).into_response();
                }
            };
            let book = segments[2].to_string();
            let uid = segments[3].strip_suffix(".vcf").unwrap_or(segments[3]).to_string();
            ferro_dav::carddav::put_contact(
                axum::extract::State(card_state),
                Path((book, uid)),
                Extension(body_bytes),
            )
            .await
        }
        "DELETE" if depth == 4 => {
            let book = segments[2].to_string();
            let uid = segments[3].strip_suffix(".vcf").unwrap_or(segments[3]).to_string();
            ferro_dav::carddav::delete_contact(axum::extract::State(card_state), Path((book, uid))).await
        }
        _ => crate::webdav::handle_any(method, uri, State(state), None, headers, body).await,
    }
}
