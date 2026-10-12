//! Mesh registry leak regression tests (YP-MESH-TRANSPORT-001).
//!
//! TV-MESH-001 / TV-MESH-005 / THM-MESH-NO-EXISTENCE-ORACLE: the registry
//! must never persist or reveal per-hash holdings. The old design had clients
//! POST their full block inventory every 600 s and answer "who holds hash H"
//! queries — an active existence oracle across tenants.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::make_app;
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    use http_body_util::BodyExt;
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

/// Old-client compatibility: registering with a legacy `block_hashes`
/// payload still succeeds — but the inventory must never be persisted or
/// reflected back.
#[tokio::test]
async fn test_register_with_legacy_inventory_ignored() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/mesh/peers")
                .header("content-type", "application/json")
                .header("x-ferro-user", "alice")
                .body(Body::from(
                    serde_json::json!({
                        "device_name": "laptop",
                        "addresses": ["192.168.1.10"],
                        "port": 7878,
                        "block_hashes": ["aaaa", "bbbb", "cccc"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "old clients must still register");

    // Peer listing must not expose inventory-derived data (no block_count).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/mesh/peers")
                .header("x-ferro-user", "bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let peers = body["peers"].as_array().expect("peers array");
    for peer in peers {
        assert!(
            peer.get("block_count").is_none(),
            "block_count leaked inventory size: {peer}"
        );
        assert!(peer.get("block_hashes").is_none(), "block_hashes echoed back: {peer}");
    }
}

/// TV-MESH-005 (DC-MESH-004): holder queries are constant-shape — the
/// response for any hash (registered-adjacent or random) is the same empty
/// list. The registry cannot leak what it does not know.
#[tokio::test]
async fn test_holder_query_constant_shape() {
    let app = make_app();

    // Register a peer (legacy payload, as if it holds "sensitivehash").
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/mesh/peers")
                .header("content-type", "application/json")
                .header("x-ferro-user", "alice")
                .body(Body::from(
                    serde_json::json!({
                        "device_name": "laptop",
                        "addresses": ["192.168.1.10"],
                        "port": 7878,
                        "block_hashes": ["sensitivehash"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Probe the hash Alice "holds" and one nobody holds: identical responses.
    for hash in ["sensitivehash", "randomunknownhash"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/mesh/peers/blocks?hash={hash}"))
                    .header("x-ferro-user", "mallory")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let holders = body["holders"].as_array().expect("holders array");
        assert!(
            holders.is_empty(),
            "holder query leaked holders for {hash}: {holders:?}"
        );
    }
}

/// Unauthenticated registration is rejected (no anonymous inventory posts).
#[tokio::test]
async fn test_register_requires_identity() {
    let app = make_app();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/mesh/peers")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "device_name": "ghost",
                        "addresses": ["10.0.0.1"],
                        "port": 7878,
                        "block_hashes": ["x"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
