use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::AppState;
use ferro_server_state::ServerState;
use ferro_dav::store::CalFilter;

#[derive(Debug, Deserialize)]
pub struct EventRangeQuery {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CalendarEventResponse {
    pub uid: String,
    pub calendar_id: String,
    pub ical_data: String,
    pub etag: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub calendar_id: String,
    pub ical_data: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    pub ical_data: String,
}

pub async fn list_events(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<EventRangeQuery>,
) -> impl IntoResponse {
    let calendars = state.calendar_store().list_calendars("default").await;

    let start = query.start.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    let end = query.end.as_deref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let filter = CalFilter { start, end };
    let mut all_events = Vec::new();

    for cal in &calendars {
        let events = state.calendar_store().query_events(&cal.id, &filter).await;
        for event in events {
            all_events.push(CalendarEventResponse {
                uid: event.uid,
                calendar_id: event.calendar_id,
                ical_data: event.ical_data,
                etag: event.etag,
                created_at: event.created_at.to_rfc3339(),
                updated_at: event.updated_at.to_rfc3339(),
            });
        }
    }

    Json(serde_json::json!({
        "events": all_events,
        "calendars": calendars,
    }))
}

pub async fn create_event(
    State(state): State<AppState>,
    Json(req): Json<CreateEventRequest>,
) -> impl IntoResponse {
    let calendars = state.calendar_store().list_calendars("default").await;
    let calendar_id = if req.calendar_id.is_empty() {
        if let Some(cal) = calendars.first() {
            cal.id.clone()
        } else {
            match state
                .calendar_store
                .create_calendar("default", "Default", "#3b82f6")
                .await
            {
                Ok(cal) => cal.id,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e.to_string()})),
                    )
                        .into_response();
                }
            }
        }
    } else {
        req.calendar_id
    };

    match state
        .calendar_store
        .create_event(&calendar_id, &req.ical_data)
        .await
    {
        Ok(event) => (
            StatusCode::CREATED,
            Json(CalendarEventResponse {
                uid: event.uid,
                calendar_id: event.calendar_id,
                ical_data: event.ical_data,
                etag: event.etag,
                created_at: event.created_at.to_rfc3339(),
                updated_at: event.updated_at.to_rfc3339(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn update_event(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Json(req): Json<UpdateEventRequest>,
) -> impl IntoResponse {
    let calendars = state.calendar_store().list_calendars("default").await;

    for cal in &calendars {
        if state
            .calendar_store
            .get_event(&cal.id, &uid)
            .await
            .is_some()
        {
            match state
                .calendar_store
                .update_event(&cal.id, &uid, &req.ical_data)
                .await
            {
                Ok(updated) => {
                    return Json(CalendarEventResponse {
                        uid: updated.uid,
                        calendar_id: updated.calendar_id,
                        ical_data: updated.ical_data,
                        etag: updated.etag,
                        created_at: updated.created_at.to_rfc3339(),
                        updated_at: updated.updated_at.to_rfc3339(),
                    })
                    .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e.to_string()})),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Event not found"})),
    )
        .into_response()
}

pub async fn delete_event(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> impl IntoResponse {
    let calendars = state.calendar_store().list_calendars("default").await;

    for cal in &calendars {
        if state
            .calendar_store
            .get_event(&cal.id, &uid)
            .await
            .is_some()
        {
            match state.calendar_store().delete_event(&cal.id, &uid).await {
                Ok(()) => return StatusCode::NO_CONTENT.into_response(),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e.to_string()})),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Event not found"})),
    )
        .into_response()
}
