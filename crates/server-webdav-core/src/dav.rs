use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::Method;
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
pub async fn dispatch_caldav<S: WebDavCoreState>(
    state: S,
    method: &Method,
    path: &str,
    body: Bytes,
) -> Response {
    let cal_state = caldav_state(&state).await;

    // Parse path segments to determine operation.
    // /dav/cal/                       → list/create calendar
    // /dav/cal/:calendar              → delete calendar
    // /dav/cal/:calendar/             → calendar properties (PROPFIND)
    // /dav/cal/:calendar/:event.ics   → get/put/delete event
    let segments: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let is_event_path = segments.len() == 4
        && segments[0] == "dav"
        && segments[1] == "cal"
        && segments[3].ends_with(".ics");

    match method.as_str() {
        "MKCALENDAR" => {
            ferro_dav::caldav::create_calendar_handler(axum::extract::State(cal_state)).await
        }
        "REPORT" => {
            if is_multiget_report(&body) {
                ferro_dav::caldav::handle_multiget(axum::extract::State(cal_state), Extension(body))
                    .await
            } else {
                ferro_dav::caldav::handle_report(axum::extract::State(cal_state), Extension(body))
                    .await
            }
        }
        "GET" if is_event_path => {
            let calendar = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".ics")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::caldav::get_event(axum::extract::State(cal_state), Path((calendar, uid)))
                .await
        }
        "PUT" if is_event_path => {
            let calendar = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".ics")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::caldav::put_event(
                axum::extract::State(cal_state),
                Path((calendar, uid)),
                Extension(body),
            )
            .await
        }
        "DELETE" if is_event_path => {
            let calendar = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".ics")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::caldav::delete_event(axum::extract::State(cal_state), Path((calendar, uid)))
                .await
        }
        _ => axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

/// Dispatch CardDAV-specific methods (REPORT, GET, PUT, DELETE on contacts)
/// that the generic WebDAV handler doesn't support. Called from
/// webdav::handle_any when the path starts with /dav/card/.
pub async fn dispatch_carddav<S: WebDavCoreState>(
    state: S,
    method: &Method,
    path: &str,
    body: Bytes,
) -> Response {
    let card_state = carddav_state(&state).await;

    // Parse path segments to determine operation.
    // /dav/card/                       → list/create address book
    // /dav/card/:book                  → delete address book
    // /dav/card/:book/                 → address book properties (PROPFIND)
    // /dav/card/:book/:contact.vcf     → get/put/delete contact
    let segments: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let is_contact_path = segments.len() == 4
        && segments[0] == "dav"
        && segments[1] == "card"
        && segments[3].ends_with(".vcf");

    match method.as_str() {
        "REPORT" => {
            if is_multiget_report(&body) {
                ferro_dav::carddav::handle_multiget(
                    axum::extract::State(card_state),
                    Extension(body),
                )
                .await
            } else {
                ferro_dav::carddav::handle_report(axum::extract::State(card_state), Extension(body))
                    .await
            }
        }
        "GET" if is_contact_path => {
            let book = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".vcf")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::carddav::get_contact(axum::extract::State(card_state), Path((book, uid)))
                .await
        }
        "PUT" if is_contact_path => {
            let book = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".vcf")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::carddav::put_contact(
                axum::extract::State(card_state),
                Path((book, uid)),
                Extension(body),
            )
            .await
        }
        "DELETE" if is_contact_path => {
            let book = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".vcf")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::carddav::delete_contact(axum::extract::State(card_state), Path((book, uid)))
                .await
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
    ferro_dav::caldav::create_calendar_handler(axum::extract::State(caldav_state(&state).await))
        .await
}

pub async fn carddav_options() -> impl IntoResponse {
    ferro_dav::carddav::options_handler().await
}

pub async fn carddav_list<S: WebDavCoreState>(State(state): State<S>) -> Response {
    ferro_dav::carddav::list_address_books(axum::extract::State(carddav_state(&state).await)).await
}

pub async fn carddav_create<S: WebDavCoreState>(State(state): State<S>) -> Response {
    ferro_dav::carddav::create_address_book_handler(axum::extract::State(
        carddav_state(&state).await,
    ))
    .await
}

/// Check if a REPORT request body is a multiget request.
/// Multiget requests have root elements `calendar-multiget` or `addressbook-multiget`.
fn is_multiget_report(body: &[u8]) -> bool {
    if body.len() < 20 || body.len() > 10 * 1024 * 1024 {
        return false;
    }
    let body_str = String::from_utf8_lossy(body);
    body_str.contains("calendar-multiget") || body_str.contains("addressbook-multiget")
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
    let segments: Vec<&str> = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .split('/')
        .collect();
    // segments: ["dav", "cal", ...]  (leading '/' stripped)
    let depth = segments.len(); // 3 = calendar level, 4 = event level

    let cal_state = caldav_state(&state).await;

    // For CalDAV-specific methods, read body and dispatch.
    // For everything else (MKCOL, COPY, MOVE, LOCK…), delegate to WebDAV.
    match method.as_str() {
        "OPTIONS" => ferro_dav::caldav::options_handler().await.into_response(),
        // ── Calendar-level operations ──────────────────────────────────
        "GET" if depth == 3 => {
            ferro_dav::caldav::list_calendars(axum::extract::State(cal_state)).await
        }
        "PUT" if depth == 3 => {
            ferro_dav::caldav::create_calendar_handler(axum::extract::State(cal_state)).await
        }
        "DELETE" if depth == 3 => {
            let calendar = segments[2].to_string();
            ferro_dav::caldav::delete_calendar_handler(
                axum::extract::State(cal_state),
                Path(calendar),
            )
            .await
        }
        "PROPFIND" => match crate::webdav::handle_propfind(state, &path, &headers).await {
            Ok(resp) => resp,
            Err(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("PROPFIND error: {}", e),
            )
                .into_response(),
        },
        // ── Event-level operations (need body) ─────────────────────────
        "MKCALENDAR" => {
            ferro_dav::caldav::create_calendar_handler(axum::extract::State(cal_state)).await
        }
        "REPORT" => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("body read: {}", e),
                    )
                        .into_response();
                }
            };
            if is_multiget_report(&body_bytes) {
                ferro_dav::caldav::handle_multiget(
                    axum::extract::State(cal_state),
                    Extension(body_bytes),
                )
                .await
            } else {
                ferro_dav::caldav::handle_report(
                    axum::extract::State(cal_state),
                    Extension(body_bytes),
                )
                .await
            }
        }
        "GET" if depth == 4 => {
            let calendar = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".ics")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::caldav::get_event(axum::extract::State(cal_state), Path((calendar, uid)))
                .await
        }
        "PUT" if depth == 4 => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("body read: {}", e),
                    )
                        .into_response();
                }
            };
            let calendar = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".ics")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::caldav::put_event(
                axum::extract::State(cal_state),
                Path((calendar, uid)),
                Extension(body_bytes),
            )
            .await
        }
        "DELETE" if depth == 4 => {
            let calendar = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".ics")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::caldav::delete_event(axum::extract::State(cal_state), Path((calendar, uid)))
                .await
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
    let segments: Vec<&str> = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .split('/')
        .collect();
    let depth = segments.len(); // 3 = book level, 4 = contact level

    let card_state = carddav_state(&state).await;

    match method.as_str() {
        "OPTIONS" => ferro_dav::carddav::options_handler().await.into_response(),
        "GET" if depth == 3 => {
            ferro_dav::carddav::list_address_books(axum::extract::State(card_state)).await
        }
        "PUT" if depth == 3 => {
            ferro_dav::carddav::create_address_book_handler(axum::extract::State(card_state)).await
        }
        "DELETE" if depth == 3 => {
            let book = segments[2].to_string();
            ferro_dav::carddav::delete_address_book_handler(
                axum::extract::State(card_state),
                Path(book),
            )
            .await
        }
        "PROPFIND" => match crate::webdav::handle_propfind(state, &path, &headers).await {
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
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("body read: {}", e),
                    )
                        .into_response();
                }
            };
            if is_multiget_report(&body_bytes) {
                ferro_dav::carddav::handle_multiget(
                    axum::extract::State(card_state),
                    Extension(body_bytes),
                )
                .await
            } else {
                ferro_dav::carddav::handle_report(
                    axum::extract::State(card_state),
                    Extension(body_bytes),
                )
                .await
            }
        }
        "GET" if depth == 4 => {
            let book = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".vcf")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::carddav::get_contact(axum::extract::State(card_state), Path((book, uid)))
                .await
        }
        "PUT" if depth == 4 => {
            let body_bytes = match body.collect().await {
                Ok(b) => b.to_bytes(),
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("body read: {}", e),
                    )
                        .into_response();
                }
            };
            let book = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".vcf")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::carddav::put_contact(
                axum::extract::State(card_state),
                Path((book, uid)),
                Extension(body_bytes),
            )
            .await
        }
        "DELETE" if depth == 4 => {
            let book = segments[2].to_string();
            let uid = segments[3]
                .strip_suffix(".vcf")
                .unwrap_or(segments[3])
                .to_string();
            ferro_dav::carddav::delete_contact(axum::extract::State(card_state), Path((book, uid)))
                .await
        }
        _ => crate::webdav::handle_any(method, uri, State(state), None, headers, body).await,
    }
}
