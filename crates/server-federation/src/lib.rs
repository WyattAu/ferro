//! Federation handler adapters for the Ferro server.
//!
//! These thin adapters convert a generic state type into `FederationState`
//! and delegate to the corresponding `ferro_server_activitypub` handlers.

use axum::extract::State;
use axum::response::Response;
use ferro_server_activitypub::FederationState;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for state types that can provide federation configuration.
pub trait FederationStateProvider {
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore>;
    fn external_url(&self) -> &str;
    fn federation_secret(&self) -> &str;
}

fn fed_state<S: FederationStateProvider>(s: &S) -> FederationState {
    FederationState {
        activity_store: s.activity_store().clone(),
        external_url: s.external_url().to_string(),
        federation_secret: s.federation_secret().to_string(),
    }
}

pub async fn get_actor<S: FederationStateProvider>(State(s): State<S>, path: axum::extract::Path<String>) -> Response {
    ferro_server_activitypub::get_actor(State(fed_state(&s)), path).await
}

pub async fn nodeinfo<S: FederationStateProvider>(State(s): State<S>) -> Response {
    ferro_server_activitypub::nodeinfo(State(fed_state(&s))).await
}

pub async fn inbox<S: FederationStateProvider>(
    State(s): State<S>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    ferro_server_activitypub::inbox(State(fed_state(&s)), req).await
}

pub async fn list_inbox<S: FederationStateProvider>(
    State(s): State<S>,
    q: axum::extract::Query<HashMap<String, String>>,
) -> Response {
    ferro_server_activitypub::list_inbox(State(fed_state(&s)), q).await
}

pub async fn list_outbox<S: FederationStateProvider>(
    State(s): State<S>,
    q: axum::extract::Query<HashMap<String, String>>,
) -> Response {
    ferro_server_activitypub::list_outbox(State(fed_state(&s)), q).await
}

pub async fn list_followers<S: FederationStateProvider>(
    State(s): State<S>,
    path: axum::extract::Path<String>,
) -> Response {
    ferro_server_activitypub::list_followers(State(fed_state(&s)), path).await
}

pub async fn list_following<S: FederationStateProvider>(
    State(s): State<S>,
    path: axum::extract::Path<String>,
) -> Response {
    ferro_server_activitypub::list_following(State(fed_state(&s)), path).await
}

pub async fn webfinger<S: FederationStateProvider>(
    State(s): State<S>,
    q: axum::extract::Query<ferro_server_activitypub::webfinger::WebFingerQuery>,
) -> Response {
    ferro_server_activitypub::webfinger::webfinger(State(fed_state(&s)), q).await
}

pub async fn federated_share<S: FederationStateProvider>(
    State(s): State<S>,
    body: axum::Json<ferro_server_activitypub::ShareRequest>,
) -> Response {
    ferro_server_activitypub::federated_share(State(fed_state(&s)), body).await
}
