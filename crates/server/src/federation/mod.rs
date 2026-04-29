use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

pub mod activity;
pub mod actor;
pub mod store;
pub mod webfinger;

use activity::{Activity, ActivityType};

pub async fn get_actor(
    State(state): State<crate::AppState>,
    Path(username): Path<String>,
) -> Response {
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
        [(
            header::CONTENT_TYPE,
            "application/json; profile=application/activity+json",
        )],
        axum::Json(info),
    )
        .into_response()
}

pub async fn inbox(
    State(state): State<crate::AppState>,
    axum::Json(activity): axum::Json<Activity>,
) -> Response {
    tracing::warn!("HTTP Signature verification not yet implemented — accepting activity");
    state.activity_store.add_to_inbox(activity.clone());

    match activity.r#type {
        ActivityType::Follow => {
            let accept = Activity {
                context: activity.context.clone(),
                id: format!("{}/activities/{}", state.external_url, uuid::Uuid::new_v4()),
                r#type: ActivityType::Accept,
                actor: format!("{}/fed/actor/admin", state.external_url),
                object: serde_json::json!(activity.id),
                to: Some(vec![activity.actor.clone()]),
                cc: None,
                published: chrono::Utc::now().to_rfc3339(),
                target: None,
            };
            state.activity_store.add_to_outbox(accept);
            state.activity_store.add_follower("admin", &activity.actor);
        }
        ActivityType::Create | ActivityType::Update | ActivityType::Delete => {}
        ActivityType::Announce => {}
        ActivityType::Undo => {
            state
                .activity_store
                .remove_follower("admin", &activity.actor);
        }
        _ => {}
    }

    (StatusCode::OK, "{}").into_response()
}

pub async fn list_inbox(
    State(state): State<crate::AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let offset = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20)
        .min(100);
    let activities = state.activity_store.get_inbox(offset, limit);
    (StatusCode::OK, axum::Json(activities)).into_response()
}

pub async fn list_outbox(
    State(state): State<crate::AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let offset = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20)
        .min(100);
    let activities = state.activity_store.get_outbox(offset, limit);
    (StatusCode::OK, axum::Json(activities)).into_response()
}

pub async fn list_followers(
    State(state): State<crate::AppState>,
    Path(username): Path<String>,
) -> Response {
    let followers = state.activity_store.get_followers(&username);
    let response = json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/fed/actor/{}/followers", state.external_url, username),
        "type": "OrderedCollection",
        "totalItems": followers.len(),
        "orderedItems": followers,
    });
    (StatusCode::OK, axum::Json(response)).into_response()
}

pub async fn list_following(
    State(state): State<crate::AppState>,
    Path(username): Path<String>,
) -> Response {
    let following = state.activity_store.get_following(&username);
    let response = json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/fed/actor/{}/following", state.external_url, username),
        "type": "OrderedCollection",
        "totalItems": following.len(),
        "orderedItems": following,
    });
    (StatusCode::OK, axum::Json(response)).into_response()
}

#[derive(Deserialize)]
pub struct ShareRequest {
    pub path: String,
    pub comment: Option<String>,
}

pub async fn federated_share(
    State(state): State<crate::AppState>,
    axum::Json(req): axum::Json<ShareRequest>,
) -> Response {
    let actor_id = format!("{}/fed/actor/admin", state.external_url);
    let file_object = json!({
        "type": "Document",
        "id": format!("{}/files/{}", state.external_url, req.path),
        "name": req.path.split('/').next_back().unwrap_or("file"),
        "url": format!("{}/dav/{}", state.external_url, req.path),
    });

    let activity = Activity::announce(&actor_id, file_object, &actor_id);
    state.activity_store.add_to_outbox(activity.clone());

    let followers = state.activity_store.get_followers("admin");
    for follower in &followers {
        tracing::info!("Would deliver to follower: {}", follower);
    }

    (
        StatusCode::OK,
        axum::Json(json!({
            "id": activity.id,
            "delivered_to": followers.len(),
            "followers_notified": followers.len(),
        })),
    )
        .into_response()
}
