use crate::store::{CalFilter, DynCalendarStore};
use crate::xml_ext::{self, DavProp, DavResponse, PropStat};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;

/// Shared state for CalDAV Axum handlers.
#[derive(Clone)]
pub struct CalDavState {
    /// The calendar store backend.
    pub store: DynCalendarStore,
    /// The authenticated principal (user).
    pub principal: String,
}

/// Handle HTTP OPTIONS for CalDAV capability discovery.
pub async fn options_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        "DAV",
        "1, 2, calendar-access"
            .parse()
            .expect("static DAV header value"),
    );
    headers.insert(
        "Allow",
        "OPTIONS, GET, PUT, DELETE, PROPFIND, REPORT, MKCALENDAR"
            .parse()
            .expect("static Allow header value"),
    );
    (StatusCode::NO_CONTENT, headers)
}

fn dav_multistatus(body: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(body.into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

fn dav_response_with_etag(status: StatusCode, etag: &str) -> Response {
    Response::builder()
        .status(status)
        .header("ETag", etag)
        .body(Bytes::new().into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

fn dav_created(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", location)
        .body(Bytes::new().into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

fn dav_ok_with_content_type(content_type: &str, etag: &str, body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("ETag", etag)
        .body(body.into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

/// List all calendars for the authenticated principal.
pub async fn list_calendars(State(state): State<CalDavState>) -> Response {
    let calendars = state.store.list_calendars(&state.principal).await;
    let responses: Vec<DavResponse> = calendars
        .iter()
        .map(|cal| DavResponse {
            href: format!("/dav/cal/{}/", cal.id),
            propstats: vec![PropStat {
                status: 200,
                props: vec![
                    DavProp {
                        name: "D:resourcetype".to_string(),
                        namespace: None,
                        value: Some(
                            "<C:calendar xmlns:C=\"urn:ietf:params:xml:ns:caldav\"/>".to_string(),
                        ),
                    },
                    DavProp {
                        name: "D:displayname".to_string(),
                        namespace: None,
                        value: Some(xml_ext::escape_xml(&cal.name)),
                    },
                    DavProp {
                        name: "C:getctag".to_string(),
                        namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                        value: Some(cal.ctag.clone()),
                    },
                ],
            }],
        })
        .collect();

    dav_multistatus(xml_ext::build_dav_multistatus(&responses))
}

/// Retrieve properties of a specific calendar.
pub async fn calendar_properties(
    State(state): State<CalDavState>,
    Path(calendar): Path<String>,
) -> Response {
    let Some(cal) = state.store.get_calendar(&state.principal, &calendar).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let response = DavResponse {
        href: format!("/dav/cal/{}/", calendar),
        propstats: vec![PropStat {
            status: 200,
            props: vec![
                DavProp {
                    name: "D:resourcetype".to_string(),
                    namespace: None,
                    value: Some(
                        "<C:calendar xmlns:C=\"urn:ietf:params:xml:ns:caldav\"/>".to_string(),
                    ),
                },
                DavProp {
                    name: "D:displayname".to_string(),
                    namespace: None,
                    value: Some(xml_ext::escape_xml(&cal.name)),
                },
                DavProp {
                    name: "C:getctag".to_string(),
                    namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                    value: Some(cal.ctag.clone()),
                },
                DavProp {
                    name: "D:sync-token".to_string(),
                    namespace: None,
                    value: Some(cal.ctag.clone()),
                },
            ],
        }],
    };

    let body = xml_ext::build_dav_multistatus(&[response]);
    dav_multistatus(body)
}

/// Create a new calendar (MKCALENDAR).
pub async fn create_calendar_handler(State(state): State<CalDavState>) -> Response {
    match state
        .store
        .create_calendar(&state.principal, "New Calendar", "#0082c9")
        .await
    {
        Ok(cal) => dav_created(&format!("/dav/cal/{}/", cal.id)),
        Err(_) => StatusCode::CONFLICT.into_response(),
    }
}

/// Delete a calendar.
pub async fn delete_calendar_handler(
    State(state): State<CalDavState>,
    Path(calendar): Path<String>,
) -> Response {
    match state
        .store
        .delete_calendar(&state.principal, &calendar)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Retrieve a calendar event by calendar ID and UID.
pub async fn get_event(
    State(state): State<CalDavState>,
    Path((calendar, uid)): Path<(String, String)>,
) -> Response {
    let Some(event) = state.store.get_event(&calendar, &uid).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    dav_ok_with_content_type("text/calendar; charset=utf-8", &event.etag, event.ical_data)
}

/// Create or update a calendar event (PUT).
pub async fn put_event(
    State(state): State<CalDavState>,
    Path((calendar, uid)): Path<(String, String)>,
    Extension(body): Extension<Bytes>,
) -> Response {
    let ical = String::from_utf8_lossy(&body).to_string();

    if state.store.get_event(&calendar, &uid).await.is_some() {
        match state.store.update_event(&calendar, &uid, &ical).await {
            Ok(event) => dav_response_with_etag(StatusCode::NO_CONTENT, &event.etag),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        match state.store.create_event(&calendar, &ical).await {
            Ok(event) => dav_response_with_etag(StatusCode::CREATED, &event.etag),
            Err(_) => StatusCode::CONFLICT.into_response(),
        }
    }
}

/// Delete a calendar event.
pub async fn delete_event(
    State(state): State<CalDavState>,
    Path((calendar, uid)): Path<(String, String)>,
) -> Response {
    match state.store.delete_event(&calendar, &uid).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Handle a CalDAV REPORT request (calendar-query).
pub async fn handle_report(
    State(state): State<CalDavState>,
    Extension(body): Extension<Bytes>,
) -> Response {
    let time_range = xml_ext::parse_calendar_query_time_range(&body);

    let filter = if let Some((start_str, end_str)) = time_range {
        CalFilter {
            start: parse_ical_timestamp(&start_str),
            end: parse_ical_timestamp(&end_str),
        }
    } else {
        CalFilter {
            start: None,
            end: None,
        }
    };

    let calendars = state.store.list_calendars(&state.principal).await;
    let mut responses = Vec::new();

    for cal in &calendars {
        let events = state.store.query_events(&cal.id, &filter).await;
        for event in &events {
            responses.push(DavResponse {
                href: format!("/dav/cal/{}/{}.ics", cal.id, event.uid),
                propstats: vec![PropStat {
                    status: 200,
                    props: vec![
                        DavProp {
                            name: "D:getetag".to_string(),
                            namespace: None,
                            value: Some(event.etag.clone()),
                        },
                        DavProp {
                            name: "C:calendar-data".to_string(),
                            namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                            value: Some(event.ical_data.clone()),
                        },
                    ],
                }],
            });
        }
    }

    let xml_body = xml_ext::build_dav_multistatus(&responses);
    dav_multistatus(xml_body)
}

fn parse_ical_timestamp(s: &str) -> Option<chrono::DateTime<Utc>> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with('Z') {
        if let Some(without_z) = trimmed.strip_suffix('Z') {
            chrono::NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S")
                .ok()
                .map(|dt| dt.and_utc())
        } else {
            None
        }
    } else {
        chrono::NaiveDateTime::parse_from_str(trimmed, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    }
}

/// Handle a CalDAV calendar-multiget REPORT request (RFC 4791 Section 7.9).
/// Retrieves specific calendar objects by href.
pub async fn handle_multiget(
    State(state): State<CalDavState>,
    Extension(body): Extension<Bytes>,
) -> Response {
    let hrefs = xml_ext::parse_multiget_hrefs(&body);
    let mut responses = Vec::new();

    for href in &hrefs {
        // Parse href: expect "/dav/cal/{calendar}/{uid}.ics"
        let path = href.trim_matches('/').trim_start_matches("dav/cal/");
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() != 2 {
            continue;
        }
        let calendar = parts[0];
        let uid = parts[1].strip_suffix(".ics").unwrap_or(parts[1]);

        if let Some(event) = state.store.get_event(calendar, uid).await {
            responses.push(DavResponse {
                href: href.clone(),
                propstats: vec![PropStat {
                    status: 200,
                    props: vec![
                        DavProp {
                            name: "D:getetag".to_string(),
                            namespace: None,
                            value: Some(event.etag.clone()),
                        },
                        DavProp {
                            name: "C:calendar-data".to_string(),
                            namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                            value: Some(event.ical_data.clone()),
                        },
                    ],
                }],
            });
        } else {
            responses.push(DavResponse {
                href: href.clone(),
                propstats: vec![PropStat {
                    status: 404,
                    props: vec![
                        DavProp {
                            name: "D:getetag".to_string(),
                            namespace: None,
                            value: None,
                        },
                        DavProp {
                            name: "C:calendar-data".to_string(),
                            namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                            value: None,
                        },
                    ],
                }],
            });
        }
    }

    let xml_body = xml_ext::build_dav_multistatus(&responses);
    dav_multistatus(xml_body)
}
