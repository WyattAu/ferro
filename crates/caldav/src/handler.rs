use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use bytes::Bytes;
use ferro_dav::store::CalFilter;

use crate::calendar::CalendarManager;
use crate::error::CalDavError;
use crate::ical;
use crate::report::{self, ReportType};

#[derive(Clone)]
pub struct CalDavState {
    pub manager: CalendarManager,
    pub principal: String,
}

pub async fn handle_options() -> impl IntoResponse {
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

pub async fn handle_propfind(
    State(state): State<CalDavState>,
    Path(calendar): Path<String>,
) -> Result<impl IntoResponse, CalDavError> {
    let _cal = state
        .manager
        .get_calendar(&state.principal, &calendar)
        .await
        .ok_or_else(|| CalDavError::NotFound(format!("Calendar not found: {}", calendar)))?;

    let xml = build_calendar_properties_response(&calendar);
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    Ok((StatusCode::MULTI_STATUS, headers, xml))
}

pub async fn handle_get(
    State(state): State<CalDavState>,
    Path((calendar, uid)): Path<(String, String)>,
) -> Result<impl IntoResponse, CalDavError> {
    let event = state
        .manager
        .get_event(&calendar, &uid)
        .await
        .ok_or_else(|| CalDavError::NotFound(format!("Event not found: {}/{}", calendar, uid)))?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("text/calendar; charset=utf-8"),
    );
    headers.insert(
        "ETag",
        HeaderValue::from_str(&event.etag).map_err(|e| CalDavError::BadRequest(e.to_string()))?,
    );
    Ok((StatusCode::OK, headers, event.data))
}

pub async fn handle_put(
    State(state): State<CalDavState>,
    Path((calendar, uid)): Path<(String, String)>,
    body: Bytes,
) -> Result<impl IntoResponse, CalDavError> {
    let ical = String::from_utf8(body.to_vec())
        .map_err(|_| CalDavError::InvalidData("Invalid UTF-8 in request body".to_string()))?;

    ical::parse_ical(&ical)
        .map_err(|e| CalDavError::InvalidData(format!("Invalid iCalendar: {}", e)))?;

    let existing = state.manager.get_event(&calendar, &uid).await;
    let event = match existing {
        Some(_) => state
            .manager
            .update_event(&calendar, &uid, &ical)
            .await?,
        None => state.manager.create_event(&calendar, &ical).await?,
    };

    let status = if existing.is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::CREATED
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        "ETag",
        HeaderValue::from_str(&event.etag).map_err(|e| CalDavError::BadRequest(e.to_string()))?,
    );
    Ok((status, headers, Bytes::new()))
}

pub async fn handle_delete(
    State(state): State<CalDavState>,
    Path((calendar, uid)): Path<(String, String)>,
) -> Result<impl IntoResponse, CalDavError> {
    state
        .manager
        .delete_event(&calendar, &uid)
        .await
        .map(|_| StatusCode::NO_CONTENT.into_response())
}

pub async fn handle_mkcalendar(
    State(state): State<CalDavState>,
    Path(calendar): Path<String>,
) -> Result<impl IntoResponse, CalDavError> {
    let cal = state
        .manager
        .create_calendar(&state.principal, &calendar, "#0082c9")
        .await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "Location",
        HeaderValue::from_str(&format!("/calendars/{}/", cal.uid))
            .map_err(|e| CalDavError::BadRequest(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, headers, Bytes::new()))
}

pub async fn handle_report(
    State(state): State<CalDavState>,
    Path(_calendar): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, CalDavError> {
    let report_type = report::parse_report(&body)?;

    match report_type {
        ReportType::CalendarQuery {
            time_range_start,
            time_range_end,
        } => {
            let start = time_range_start.as_deref().and_then(ical::parse_timestamp);
            let end = time_range_end.as_deref().and_then(ical::parse_timestamp);

            let filter = CalFilter { start, end };

            let calendars = state.manager.list_calendars(&state.principal).await;
            let mut all_items = Vec::new();

            for cal in &calendars {
                let items = state.manager.query_events(&cal.uid, &filter).await;
                all_items.extend(items);
            }

            let xml = report::build_report_response(&calendars, &all_items);
            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/xml; charset=utf-8"),
            );
            Ok((StatusCode::MULTI_STATUS, headers, xml))
        }
        ReportType::CalendarMultiget { hrefs } => {
            let mut items = Vec::new();

            for href in &hrefs {
                let path = href.trim_matches('/').trim_start_matches("calendars/");
                let parts: Vec<&str> = path.splitn(2, '/').collect();
                if parts.len() != 2 {
                    items.push((href.clone(), None));
                    continue;
                }
                let cal = parts[0];
                let uid = parts[1].strip_suffix(".ics").unwrap_or(parts[1]);

                let event = state.manager.get_event(cal, uid).await;
                items.push((href.clone(), event));
            }

            let xml = report::build_multiget_response(&items);
            let mut headers = HeaderMap::new();
            headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/xml; charset=utf-8"),
            );
            Ok((StatusCode::MULTI_STATUS, headers, xml))
        }
    }
}

fn build_calendar_properties_response(calendar: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
<D:response>
<D:href>/calendars/{}</D:href>
<D:propstat>
<D:prop>
<D:resourcetype><C:calendar/></D:resourcetype>
<D:displayname>{}</D:displayname>
<C:getctag>0</C:getctag>
</D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>
</D:multistatus>"#,
        calendar, calendar
    )
    .into_bytes()
}
