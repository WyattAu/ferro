use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> Bytes {
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

fn create_hmac_signature(secret: &str, method: &str, path: &str, key_id: &str) -> String {
    use base64::Engine;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let signing_string = format!("(request-target): {} {}", method.to_lowercase(), path);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signing_string.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    format!(
        r#"keyId="{}",algorithm="hs2019",headers="(request-target)",signature="{}""#,
        key_id, sig_b64
    )
}

#[tokio::test]
async fn test_webfinger_returns_jrd_json() {
    let app = ferro_server::make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/webfinger?resource=acct:alice@example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["subject"], "acct:alice@example.com");
    assert!(json["aliases"].is_array());
    assert!(
        json["aliases"][0]
            .as_str()
            .unwrap()
            .contains("/fed/actor/alice")
    );
    assert!(json["links"].is_array());
    let links = json["links"].as_array().unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["rel"], "self");
    assert_eq!(links[0]["type"], "application/activity+json");
    assert!(
        links[0]["href"]
            .as_str()
            .unwrap()
            .contains("/fed/actor/alice")
    );
}

#[tokio::test]
async fn test_webfinger_invalid_resource() {
    let app = ferro_server::make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/webfinger?resource=invalid-no-at-sign")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_actor_endpoint_returns_activitypub_actor() {
    let app = ferro_server::make_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    let resp = reqwest::get(format!("http://{}/fed/actor/admin", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["type"], "Service");
    assert_eq!(json["preferred_username"], "admin");
}

#[tokio::test]
async fn test_actor_endpoint_different_username() {
    let app = ferro_server::make_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    let resp = reqwest::get(format!("http://{}/fed/actor/alice", addr))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["preferred_username"], "alice");
    assert!(json["id"].as_str().unwrap().contains("/fed/actor/alice"));
}

#[tokio::test]
async fn test_inbox_disabled_returns_503() {
    let app = ferro_server::make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/fed/inbox")
                .header("Content-Type", "application/activity+json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_inbox_accepts_valid_signed_follow_activity() {
    let state = ferro_server::AppState::in_memory()
        .with_federation_secret("test-integration-secret".to_string());
    let app = ferro_server::build_router(state);

    let remote_actor = "https://remote.example.com/actor/bob";
    let activity = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/activities/test-1", remote_actor),
        "type": "Follow",
        "actor": remote_actor,
        "object": "https://local.example.com/fed/actor/admin",
        "to": ["https://local.example.com/fed/actor/admin"],
        "published": "2024-01-01T00:00:00+00:00"
    });

    let sig = create_hmac_signature(
        "test-integration-secret",
        "POST",
        "/fed/inbox",
        &format!("{}#main-key", remote_actor),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/fed/inbox")
                .header("Content-Type", "application/activity+json")
                .header("Signature", &sig)
                .body(Body::from(serde_json::to_vec(&activity).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_inbox_rejects_missing_signature() {
    let state = ferro_server::AppState::in_memory()
        .with_federation_secret("test-integration-secret".to_string());
    let app = ferro_server::build_router(state);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/fed/inbox")
                .header("Content-Type", "application/activity+json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_inbox_rejects_bad_signature() {
    let state = ferro_server::AppState::in_memory()
        .with_federation_secret("test-integration-secret".to_string());
    let app = ferro_server::build_router(state);

    let activity = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": "https://remote.example.com/activities/test-2",
        "type": "Follow",
        "actor": "https://remote.example.com/actor/bob",
        "object": "https://local.example.com/fed/actor/admin",
        "to": ["https://local.example.com/fed/actor/admin"],
        "published": "2024-01-01T00:00:00+00:00"
    });

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/fed/inbox")
                .header("Content-Type", "application/activity+json")
                .header(
                    "Signature",
                    r#"keyId="k",algorithm="hs2019",headers="(request-target)",signature="AAAAAAAAAAAAAAAAAAAAAA==""#,
                )
                .body(Body::from(serde_json::to_vec(&activity).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_nodeinfo_returns_proper_json() {
    let app = ferro_server::make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fed/nodeinfo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["version"], "2.1");
    assert_eq!(json["software"]["name"], "ferro");
    assert!(json["protocols"].is_array());
    let protocols = json["protocols"].as_array().unwrap();
    assert!(protocols.contains(&serde_json::json!("activitypub")));
    assert!(protocols.contains(&serde_json::json!("webfinger")));
    assert!(json["services"]["inbound"].is_array());
    assert!(json["services"]["outbound"].is_array());
}

#[tokio::test]
async fn test_outbox_returns_activity_array() {
    let app = ferro_server::make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fed/outbox")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.is_array());
}

#[tokio::test]
async fn test_outbox_with_follow_returns_ordered_items() {
    let state = ferro_server::AppState::in_memory()
        .with_federation_secret("test-integration-secret".to_string());
    let app = ferro_server::build_router(state);

    let remote_actor = "https://remote.example.com/actor/bob";
    let activity = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/activities/test-outbox-1", remote_actor),
        "type": "Follow",
        "actor": remote_actor,
        "object": "https://local.example.com/fed/actor/admin",
        "to": ["https://local.example.com/fed/actor/admin"],
        "published": "2024-01-01T00:00:00+00:00"
    });

    let sig = create_hmac_signature(
        "test-integration-secret",
        "POST",
        "/fed/inbox",
        &format!("{}#main-key", remote_actor),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/fed/inbox")
                .header("Content-Type", "application/activity+json")
                .header("Signature", &sig)
                .body(Body::from(serde_json::to_vec(&activity).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fed/outbox")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.is_array());
    let outbox = json.as_array().unwrap();
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0]["type"], "Accept");
}

#[tokio::test]
async fn test_list_inbox_returns_stored_activities() {
    let state = ferro_server::AppState::in_memory()
        .with_federation_secret("test-integration-secret".to_string());
    let app = ferro_server::build_router(state);

    let remote_actor = "https://remote.example.com/actor/carol";
    let activity = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/activities/test-inbox-list", remote_actor),
        "type": "Follow",
        "actor": remote_actor,
        "object": "https://local.example.com/fed/actor/admin",
        "to": ["https://local.example.com/fed/actor/admin"],
        "published": "2024-01-01T00:00:00+00:00"
    });

    let sig = create_hmac_signature(
        "test-integration-secret",
        "POST",
        "/fed/inbox",
        &format!("{}#main-key", remote_actor),
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/fed/inbox")
                .header("Content-Type", "application/activity+json")
                .header("Signature", &sig)
                .body(Body::from(serde_json::to_vec(&activity).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/fed/inbox")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.is_array());
    let inbox = json.as_array().unwrap();
    assert!(!inbox.is_empty());
    assert_eq!(inbox[0]["type"], "Follow");
    assert_eq!(inbox[0]["actor"], remote_actor);
}
