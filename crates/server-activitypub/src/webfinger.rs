use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::FederationState;

#[derive(Deserialize)]
pub struct WebFingerQuery {
    pub resource: String,
}

#[derive(Serialize)]
pub struct WebFingerResponse {
    pub subject: String,
    pub aliases: Vec<String>,
    pub links: Vec<WebFingerLink>,
}

#[derive(Serialize)]
pub struct WebFingerLink {
    pub rel: String,
    pub r#type: String,
    pub href: String,
}

pub async fn webfinger(State(state): State<FederationState>, Query(params): Query<WebFingerQuery>) -> Response {
    let resource = params.resource.strip_prefix("acct:").unwrap_or(&params.resource);
    let parts: Vec<&str> = resource.splitn(2, '@').collect();
    if parts.len() != 2 {
        return (StatusCode::BAD_REQUEST, "Invalid resource").into_response();
    }

    let base_url = &state.external_url;
    let actor_id = format!("{}/fed/actor/{}", base_url, parts[0]);

    let response = WebFingerResponse {
        subject: params.resource.clone(),
        aliases: vec![actor_id.clone()],
        links: vec![WebFingerLink {
            rel: "self".to_string(),
            r#type: "application/activity+json".to_string(),
            href: actor_id,
        }],
    };

    (StatusCode::OK, axum::Json(response)).into_response()
}
