pub mod activity;
pub mod actor;
pub mod delivery;
pub mod http_sig;
pub mod store;
pub mod webfinger;

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;

use activity::{Activity, ActivityType};
use http_sig::HttpSignature;

#[derive(Clone)]
pub struct FederationState {
    pub activity_store: std::sync::Arc<store::ActivityStore>,
    pub external_url: String,
    pub federation_secret: String,
}

pub async fn resolve_actor(actor_url: &str) -> Result<actor::Actor, String> {
    let client = delivery::federation_client();
    let response = client
        .get(actor_url)
        .header("Accept", "application/activity+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch actor {}: {}", actor_url, e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Actor fetch failed with status {} for {}",
            status, actor_url
        ));
    }

    response
        .json::<actor::Actor>()
        .await
        .map_err(|e| format!("Failed to parse actor JSON from {}: {}", actor_url, e))
}

async fn deliver_accept(state: &FederationState, accept: &Activity, remote_actor_url: &str) {
    let remote_actor = match resolve_actor(remote_actor_url).await {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!("Failed to resolve remote actor {}: {}", remote_actor_url, e);
            return;
        }
    };

    let accept_value = match serde_json::to_value(accept) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to serialize Accept: {}", e);
            return;
        }
    };

    if let Err(e) = delivery::deliver_to_inbox(
        &remote_actor.inbox,
        &accept_value,
        &state.federation_secret,
        &state.external_url,
    )
    .await
    {
        tracing::warn!("Failed to deliver Accept to {}: {}", remote_actor.inbox, e);
    }
}

pub async fn get_actor(
    State(state): State<FederationState>,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Response {
    let base_url = &state.external_url;
    let actor = match actor::Actor::new(base_url, &username, &username) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("failed to generate actor key pair: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to generate actor",
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/activity+json")],
        axum::Json(actor),
    )
        .into_response()
}

pub async fn nodeinfo(State(_state): State<FederationState>) -> Response {
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
    State(state): State<FederationState>,
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

    if let Err(e) = state.activity_store.add_to_inbox(activity.clone()) {
        tracing::warn!("inbox: {e}");
    }

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
            state.activity_store.add_to_outbox(accept.clone()).ok();
            state
                .activity_store
                .add_follower("admin", &activity.actor)
                .ok();

            let deliver_state = state.clone();
            let accept_clone = accept;
            let remote_actor = activity.actor.clone();
            tokio::spawn(async move {
                deliver_accept(&deliver_state, &accept_clone, &remote_actor).await;
            });
        }
        ActivityType::Create | ActivityType::Update | ActivityType::Delete => {}
        ActivityType::Announce => {}
        ActivityType::Undo => {
            if let serde_json::Value::String(target_url) = &activity.object {
                state
                    .activity_store
                    .remove_follower("admin", target_url)
                    .ok();
            }
        }
        _ => {}
    }

    (StatusCode::OK, "{}").into_response()
}

pub async fn list_inbox(
    State(state): State<FederationState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
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
    State(state): State<FederationState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
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
    State(state): State<FederationState>,
    axum::extract::Path(username): axum::extract::Path<String>,
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
    State(state): State<FederationState>,
    axum::extract::Path(username): axum::extract::Path<String>,
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
pub struct FollowRemoteRequest {
    pub actor_url: String,
}

pub async fn follow_remote(
    State(state): State<FederationState>,
    axum::Json(req): axum::Json<FollowRemoteRequest>,
) -> Response {
    if state.federation_secret.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(json!({
                "error": "Federation is disabled",
                "error_description": "Set FERRO_FEDERATION_SECRET to enable federation"
            })),
        )
            .into_response();
    }

    let remote_actor = match resolve_actor(&req.actor_url).await {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                axum::Json(json!({"error": format!("Failed to resolve actor: {}", e)})),
            )
                .into_response();
        }
    };

    let local_actor_url = format!("{}/fed/actor/admin", state.external_url);
    let follow = Activity::follow(&local_actor_url, &remote_actor.id);
    state.activity_store.add_to_outbox(follow.clone()).ok();

    let follow_value = match serde_json::to_value(&follow) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": format!("Failed to serialize activity: {}", e)})),
            )
                .into_response();
        }
    };

    match delivery::deliver_to_inbox(
        &remote_actor.inbox,
        &follow_value,
        &state.federation_secret,
        &state.external_url,
    )
    .await
    {
        Ok(()) => (
            StatusCode::OK,
            axum::Json(json!({
                "status": "follow_sent",
                "activity_id": follow.id,
                "target": remote_actor.id,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(json!({"error": format!("Failed to deliver Follow: {}", e)})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct ShareRequest {
    pub path: String,
    pub comment: Option<String>,
}

pub async fn federated_share(
    State(state): State<FederationState>,
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
    state.activity_store.add_to_outbox(activity.clone()).ok();

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

pub fn routes(state: FederationState) -> Router {
    Router::new()
        .route(
            "/.well-known/webfinger",
            axum::routing::get(webfinger::webfinger),
        )
        .route("/fed/actor/{username}", axum::routing::get(get_actor))
        .route(
            "/fed/actor/{username}/followers",
            axum::routing::get(list_followers),
        )
        .route(
            "/fed/actor/{username}/following",
            axum::routing::get(list_following),
        )
        .route("/fed/inbox", axum::routing::post(inbox).get(list_inbox))
        .route("/fed/outbox", axum::routing::get(list_outbox))
        .route("/fed/nodeinfo", axum::routing::get(nodeinfo))
        .route("/fed/share", axum::routing::post(federated_share))
        .route("/fed/follow", axum::routing::post(follow_remote))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use activity::{Activity, ActivityType};
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    fn create_hmac_signature(secret: &str, method: &str, path: &str, key_id: &str) -> String {
        let signing_string = format!("(request-target): {} {}", method.to_lowercase(), path);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_string.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let sig_b64 = STANDARD.encode(sig_bytes);
        format!(
            r#"keyId="{}",algorithm="hs2019",headers="(request-target)",signature="{}""#,
            key_id, sig_b64
        )
    }

    fn make_state() -> FederationState {
        FederationState {
            activity_store: std::sync::Arc::new(store::ActivityStore::new()),
            external_url: "https://local.example.com".to_string(),
            federation_secret: "test-secret".to_string(),
        }
    }

    fn make_follow_activity(remote_actor: &str) -> Activity {
        Activity {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: format!("{}/activities/{}", remote_actor, uuid::Uuid::new_v4()),
            r#type: ActivityType::Follow,
            actor: remote_actor.to_string(),
            object: serde_json::json!("https://local.example.com/fed/actor/admin"),
            to: Some(vec![
                "https://local.example.com/fed/actor/admin".to_string(),
            ]),
            cc: None,
            published: chrono::Utc::now().to_rfc3339(),
            target: None,
        }
    }

    async fn build_inbox_request(
        state: &FederationState,
        activity: &Activity,
    ) -> axum::http::Request<axum::body::Body> {
        let body = serde_json::to_vec(activity).unwrap();
        let sig_header = create_hmac_signature(
            &state.federation_secret,
            "POST",
            "/fed/inbox",
            &format!("{}#main-key", activity.actor),
        );

        axum::http::Request::builder()
            .method("POST")
            .uri("/fed/inbox")
            .header("Signature", &sig_header)
            .header("Content-Type", "application/activity+json")
            .body(axum::body::Body::from(body))
            .unwrap()
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

        let sig = HttpSignature::parse(sig_header).unwrap();
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

    #[tokio::test]
    async fn test_inbox_follow_creates_accept_and_stores_follower() {
        let state = make_state();
        let remote_actor = "https://remote.example.com/actor/bob";
        let activity = make_follow_activity(remote_actor);
        let req = build_inbox_request(&state, &activity).await;

        let response = inbox(State(state.clone()), req).await;
        assert_eq!(response.status(), StatusCode::OK);

        let followers = state.activity_store.get_followers("admin");
        assert_eq!(followers.len(), 1);
        assert_eq!(followers[0], remote_actor);

        let outbox = state.activity_store.get_outbox(0, 10);
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].r#type, ActivityType::Accept);
        assert_eq!(outbox[0].object, serde_json::json!(activity.id));
        assert_eq!(outbox[0].to, Some(vec![remote_actor.to_string()]));
    }

    #[tokio::test]
    async fn test_inbox_undo_removes_follower() {
        let state = make_state();
        let remote_actor = "https://remote.example.com/actor/bob";

        state
            .activity_store
            .add_follower("admin", remote_actor)
            .unwrap();
        assert_eq!(state.activity_store.get_followers("admin").len(), 1);

        let undo = Activity {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: format!("{}/activities/undo-{}", remote_actor, uuid::Uuid::new_v4()),
            r#type: ActivityType::Undo,
            actor: remote_actor.to_string(),
            object: serde_json::json!(remote_actor),
            to: Some(vec![
                "https://local.example.com/fed/actor/admin".to_string(),
            ]),
            cc: None,
            published: chrono::Utc::now().to_rfc3339(),
            target: None,
        };
        let req = build_inbox_request(&state, &undo).await;

        let response = inbox(State(state.clone()), req).await;
        assert_eq!(response.status(), StatusCode::OK);

        let followers = state.activity_store.get_followers("admin");
        assert!(followers.is_empty());
    }

    #[tokio::test]
    async fn test_inbox_rejects_missing_signature() {
        let state = make_state();
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/fed/inbox")
            .header("Content-Type", "application/activity+json")
            .body(axum::body::Body::from("{}"))
            .unwrap();

        let response = inbox(State(state), req).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_inbox_rejects_invalid_signature() {
        let state = make_state();
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/fed/inbox")
            .header(
                "Signature",
                r#"keyId="k",algorithm="hs2019",headers="(request-target)",signature="AAAAAA==""#,
            )
            .header("Content-Type", "application/activity+json")
            .body(axum::body::Body::from("{}"))
            .unwrap();

        let response = inbox(State(state), req).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_inbox_disabled_federation() {
        let state = FederationState {
            activity_store: std::sync::Arc::new(store::ActivityStore::new()),
            external_url: "https://local.example.com".to_string(),
            federation_secret: String::new(),
        };
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/fed/inbox")
            .body(axum::body::Body::from("{}"))
            .unwrap();

        let response = inbox(State(state), req).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_actor_creation() {
        let actor = actor::Actor::new("https://files.example.com", "admin", "Admin").unwrap();
        assert_eq!(actor.preferred_username, "admin");
        assert_eq!(actor.r#type, "Service");
        assert!(actor.inbox.contains("/fed/inbox"));
        assert!(actor.outbox.contains("/fed/outbox"));
        assert!(actor.followers.contains("/fed/followers"));
        assert!(actor.following.contains("/fed/following"));
    }

    #[test]
    fn test_activity_store_follow_workflow() {
        let store = store::ActivityStore::new();
        let remote_actor = "https://remote.example.com/actor/bob";

        assert!(store.get_followers("admin").is_empty());

        store.add_follower("admin", remote_actor).unwrap();
        let followers = store.get_followers("admin");
        assert_eq!(followers.len(), 1);
        assert_eq!(followers[0], remote_actor);

        store.remove_follower("admin", remote_actor).unwrap();
        assert!(store.get_followers("admin").is_empty());
    }

    #[test]
    fn test_activity_types_serialization() {
        let types = vec![
            ActivityType::Create,
            ActivityType::Update,
            ActivityType::Delete,
            ActivityType::Announce,
            ActivityType::Follow,
            ActivityType::Accept,
            ActivityType::Reject,
            ActivityType::Like,
            ActivityType::Undo,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: ActivityType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_follower_delivery_collects_errors() {
        let state = make_state();
        store::ActivityStore::add_follower(
            &state.activity_store,
            "admin",
            "https://bad.example.com/fed/actor/alice",
        )
        .unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let results = rt.block_on(async {
            delivery::deliver_to_followers(
                &state,
                &serde_json::json!({"type": "Create", "actor": "https://local.example.com/fed/actor/admin"}),
            )
            .await
        });

        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.is_err()));
    }
}
