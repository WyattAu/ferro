use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub mod actor;
pub mod webfinger;

pub async fn get_actor(State(state): State<crate::AppState>, Path(username): Path<String>) -> Response {
    let base_url = &state.external_url;
    let actor = actor::Actor::new(base_url, &username, &username);

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/activity+json")],
        axum::Json(actor),
    )
        .into_response()
}

pub async fn nodeinfo(State(_state): State<crate::AppState>) -> Response {
    let info = json!({
        "version": "2.1",
        "software": {
            "name": "ferro",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "protocols": ["activitypub", "webfinger", "dav"],
        "services": {
            "inbound": ["activitypub", "webdav", "caldav", "carddav"],
            "outbound": ["activitypub"],
        },
        "usage": {
            "users": 1,
            "localPosts": 0,
        },
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json; profile=application/activity+json")],
        axum::Json(info),
    )
        .into_response()
}
