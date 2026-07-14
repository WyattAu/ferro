pub use ferro_server_activitypub::FederationState;
pub use ferro_server_activitypub::store::ActivityStore;
pub use ferro_server_activitypub::*;

use axum::extract::State;
use axum::response::Response;

fn fed_state(s: &impl ferro_server_state::ServerState) -> FederationState {
    FederationState {
        activity_store: s.activity_store().clone(),
        external_url: s.external_url().to_string(),
        federation_secret: s.federation_secret().to_string(),
    }
}

pub async fn get_actor(State(s): State<crate::AppState>, path: axum::extract::Path<String>) -> Response {
    ferro_server_activitypub::get_actor(State(fed_state(&s)), path).await
}

pub async fn nodeinfo(State(s): State<crate::AppState>) -> Response {
    ferro_server_activitypub::nodeinfo(State(fed_state(&s))).await
}

pub async fn inbox(State(s): State<crate::AppState>, req: axum::http::Request<axum::body::Body>) -> Response {
    ferro_server_activitypub::inbox(State(fed_state(&s)), req).await
}

pub async fn list_inbox(
    State(s): State<crate::AppState>,
    q: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    ferro_server_activitypub::list_inbox(State(fed_state(&s)), q).await
}

pub async fn list_outbox(
    State(s): State<crate::AppState>,
    q: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    ferro_server_activitypub::list_outbox(State(fed_state(&s)), q).await
}

pub async fn list_followers(State(s): State<crate::AppState>, path: axum::extract::Path<String>) -> Response {
    ferro_server_activitypub::list_followers(State(fed_state(&s)), path).await
}

pub async fn list_following(State(s): State<crate::AppState>, path: axum::extract::Path<String>) -> Response {
    ferro_server_activitypub::list_following(State(fed_state(&s)), path).await
}

pub async fn webfinger(
    State(s): State<crate::AppState>,
    q: axum::extract::Query<ferro_server_activitypub::webfinger::WebFingerQuery>,
) -> Response {
    ferro_server_activitypub::webfinger::webfinger(State(fed_state(&s)), q).await
}

pub async fn federated_share(
    State(s): State<crate::AppState>,
    body: axum::Json<ferro_server_activitypub::ShareRequest>,
) -> Response {
    ferro_server_activitypub::federated_share(State(fed_state(&s)), body).await
}
