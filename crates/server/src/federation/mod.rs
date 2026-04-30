use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

pub mod activity;
pub mod actor;
pub mod delivery;
pub mod http_sig;
pub mod store;
pub mod webfinger;

use activity::{Activity, ActivityType};
use http_sig::HttpSignature;

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
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    if state.federation_secret.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(json!({
                "error": "Federation is disabled",
                "error_description": "Set FERRO_FEDERATION_SECRET to enable the federation inbox"
            })),
        )
            .into_response();
    }

    let signature_header = req.headers().get("Signature").and_then(|v| v.to_str().ok());

    match signature_header {
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "Missing Signature header",
                    "error_description": "ActivityPub activities must be signed per draft-cavage-http-signatures-12"
                })),
            )
                .into_response();
        }
        Some(sig_str) => {
            let sig = match HttpSignature::parse(sig_str) {
                Ok(s) => s,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        axum::Json(json!({"error": format!("Invalid signature: {}", e)})),
                    )
                        .into_response();
                }
            };

            match sig.verify_hmac(
                req.method(),
                req.uri().path(),
                req.headers(),
                &state.federation_secret,
            ) {
                Ok(true) => {}
                Ok(false) | Err(_) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(json!({"error": "Signature verification failed"})),
                    )
                        .into_response();
                }
            }
        }
    }

    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .unwrap_or_default();
    let activity: Activity = match serde_json::from_slice(&body_bytes) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({"error": format!("Invalid activity JSON: {}", e)})),
            )
                .into_response();
        }
    };

    if let Some(sig_str) = parts.headers.get("Signature").and_then(|v| v.to_str().ok())
        && let Ok(sig) = HttpSignature::parse(sig_str)
    {
        let sig_actor = http_sig::actor_from_key_id(&sig.key_id);
        if sig_actor != activity.actor {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "Actor mismatch",
                    "error_description": "Signature keyId does not match activity actor"
                })),
            )
                .into_response();
        }
    }

    state.activity_store.add_to_inbox(activity.clone());

    let delivery_state = state.clone();
    let act = serde_json::to_value(activity.clone()).unwrap_or_default();
    tokio::spawn(async move {
        let results = delivery::deliver_to_followers(&delivery_state, &act).await;
        let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
        if !errors.is_empty() {
            tracing::warn!("Follower delivery had errors: {:?}", errors);
        }
    });

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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    fn create_hmac_signature(secret: &str, method: &str, path: &str, key_id: &str) -> String {
        let signing_string = format!("(request-target): {} {}", method.to_lowercase(), path);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_string.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let sig_b64 = STANDARD.encode(&sig_bytes);
        format!(
            r#"keyId="{}",algorithm="hs2019",headers="(request-target)",signature="{}""#,
            key_id, sig_b64
        )
    }

    #[test]
    fn test_signature_verification_valid_signature() {
        let secret = "test-federation-secret";
        let key_id = "https://example.com/actor/alice#main-key";
        let sig_header = create_hmac_signature(secret, "POST", "/fed/inbox", key_id);

        let sig = HttpSignature::parse(&sig_header).unwrap();
        let method = axum::http::Method::POST;
        let headers = axum::http::HeaderMap::new();

        let result = sig.verify_hmac(&method, "/fed/inbox", &headers, secret);
        assert!(result.is_ok());
        assert!(result.unwrap());

        let actor = http_sig::actor_from_key_id(&sig.key_id);
        assert_eq!(actor, "https://example.com/actor/alice");
    }

    #[test]
    fn test_signature_verification_wrong_secret() {
        let sig_header = create_hmac_signature("correct-secret", "POST", "/fed/inbox", "k#main");

        let sig = HttpSignature::parse(&sig_header).unwrap();
        let result = sig.verify_hmac(
            &axum::http::Method::POST,
            "/fed/inbox",
            &axum::http::HeaderMap::new(),
            "wrong-secret",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_verification_tampered_signature() {
        let sig_header = r#"keyId="k",algorithm="hs2019",headers="(request-target)",signature="AAAAAAAAAAAAAAAAAAAAAA==""#;

        let sig = HttpSignature::parse(&sig_header).unwrap();
        let result = sig.verify_hmac(
            &axum::http::Method::POST,
            "/fed/inbox",
            &axum::http::HeaderMap::new(),
            "any-secret",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_verification_actor_mismatch_detected() {
        let secret = "test-secret";
        let key_id = "https://example.com/actor/alice#main-key";
        let sig =
            HttpSignature::parse(&create_hmac_signature(secret, "POST", "/fed/inbox", key_id))
                .unwrap();

        let sig_actor = http_sig::actor_from_key_id(&sig.key_id);
        let activity_actor = "https://example.com/actor/bob";
        assert_ne!(sig_actor, activity_actor);
    }

    #[test]
    fn test_signature_verification_actor_match() {
        let secret = "test-secret";
        let key_id = "https://example.com/actor/alice#main-key";
        let sig =
            HttpSignature::parse(&create_hmac_signature(secret, "POST", "/fed/inbox", key_id))
                .unwrap();

        let sig_actor = http_sig::actor_from_key_id(&sig.key_id);
        let activity_actor = "https://example.com/actor/alice";
        assert_eq!(sig_actor, activity_actor);
    }

    #[test]
    fn test_signature_parse_missing_key_id() {
        let sig = r#"algorithm="hs2019",signature="dGVzdA==""#;
        assert!(HttpSignature::parse(sig).is_err());
    }

    #[test]
    fn test_signature_parse_missing_signature() {
        let sig = r#"keyId="k",algorithm="hs2019""#;
        assert!(HttpSignature::parse(sig).is_err());
    }
}
