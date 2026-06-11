use bytes::Bytes;
use common::storage::StorageEngine;
use ferro_crdt::document::{CrdtDocument, DocumentId, ParticipantId};
use ferro_server::collab_ws::CollabMessage;
use ferro_server::storage::InMemoryStorageEngine;
use ferro_server::{AppState, build_router};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

async fn setup_server() -> (u16, Arc<InMemoryStorageEngine>) {
    let storage = Arc::new(InMemoryStorageEngine::new());
    let state = AppState::new(storage.clone() as Arc<dyn common::storage::StorageEngine>);
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    (addr.port(), storage)
}

async fn connect_client(
    port: u16,
    doc_id: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = format!("ws://127.0.0.1:{}/ws/collab/{}", port, doc_id);
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws
}

async fn recv_message(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> CollabMessage {
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(parsed) = serde_json::from_str::<CollabMessage>(&text) {
                        return parsed;
                    }
                }
                Ok(Message::Close(_)) => panic!("connection closed unexpectedly"),
                Err(e) => panic!("ws error: {:?}", e),
                _ => {}
            }
        }
        panic!("no message received");
    });
    timeout.await.unwrap()
}

async fn recv_message_skip_hello(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> CollabMessage {
    loop {
        let msg = recv_message(ws).await;
        match msg {
            CollabMessage::Hello { .. } => continue,
            other => return other,
        }
    }
}

async fn send_message(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    msg: &CollabMessage,
) {
    let json = serde_json::to_string(msg).unwrap();
    ws.send(Message::Text(json)).await.unwrap();
}

async fn join_client(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    doc_id: &str,
    pid: u32,
    name: &str,
) {
    send_message(
        ws,
        &CollabMessage::Join {
            document_id: doc_id.to_string(),
            participant_id: pid,
            name: name.to_string(),
        },
    )
    .await;
    let _state = recv_message_skip_hello(ws).await;
}

#[tokio::test]
async fn test_ws_route_exists() {
    use tower::ServiceExt;
    let state = AppState::in_memory();
    let app = build_router(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/ws/collab/test-doc")
                .header("upgrade", "websocket")
                .header("connection", "Upgrade")
                .header("sec-websocket-version", "13")
                .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == axum::http::StatusCode::SWITCHING_PROTOCOLS
            || resp.status() == axum::http::StatusCode::UPGRADE_REQUIRED,
        "route should exist, got: {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_two_clients_connect_same_document() {
    let (port, _) = setup_server().await;
    let mut ws1 = connect_client(port, "shared-doc").await;
    let mut ws2 = connect_client(port, "shared-doc").await;

    let hello1 = recv_message(&mut ws1).await;
    assert!(matches!(hello1, CollabMessage::Hello { .. }));
    let hello2 = recv_message(&mut ws2).await;
    assert!(matches!(hello2, CollabMessage::Hello { .. }));

    send_message(
        &mut ws1,
        &CollabMessage::Join {
            document_id: "shared-doc".into(),
            participant_id: 1,
            name: "Alice".into(),
        },
    )
    .await;

    send_message(
        &mut ws2,
        &CollabMessage::Join {
            document_id: "shared-doc".into(),
            participant_id: 2,
            name: "Bob".into(),
        },
    )
    .await;

    let mut got_document_state = false;
    let mut got_participants = false;
    for _ in 0..6 {
        let msg = recv_message(&mut ws1).await;
        match msg {
            CollabMessage::DocumentState { .. } => got_document_state = true,
            CollabMessage::Participants { .. } => got_participants = true,
            _ => {}
        }
        if got_document_state && got_participants {
            break;
        }
    }
    assert!(got_document_state, "client 1 should receive DocumentState");
    assert!(got_participants, "client 1 should receive Participants");
}

#[tokio::test]
async fn test_client_a_sends_client_b_receives() {
    let (port, _) = setup_server().await;
    let mut ws_a = connect_client(port, "relay-doc").await;
    let mut ws_b = connect_client(port, "relay-doc").await;

    let _hello = recv_message(&mut ws_a).await;
    let _hello = recv_message(&mut ws_b).await;

    join_client(&mut ws_a, "relay-doc", 10, "Alice").await;
    join_client(&mut ws_b, "relay-doc", 20, "Bob").await;

    let mut doc = CrdtDocument::new(DocumentId("relay-doc".into()));
    doc.join(ParticipantId(10), "Alice");
    let (ops, _) = doc.insert_text(ParticipantId(10), 0, "Hi from Alice");

    send_message(&mut ws_a, &CollabMessage::Operations { ops }).await;

    let received = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let msg = recv_message(&mut ws_b).await;
            if let CollabMessage::Operations { .. } = msg {
                return msg;
            }
        }
    })
    .await
    .unwrap();

    match received {
        CollabMessage::Operations { ops } => {
            assert!(!ops.is_empty());
        }
        _ => panic!("expected Operations"),
    }
}

#[tokio::test]
async fn test_concurrent_operations_converge() {
    let mut doc1 = CrdtDocument::new(DocumentId("conv-doc".into()));
    doc1.join(ParticipantId(1), "Alice");
    doc1.join(ParticipantId(2), "Bob");

    let (init_ops, _) = doc1.insert_text(ParticipantId(1), 0, "AB");

    let mut doc2 = CrdtDocument::new(DocumentId("conv-doc".into()));
    doc2.join(ParticipantId(1), "Alice");
    doc2.join(ParticipantId(2), "Bob");
    doc2.apply_ops(&init_ops);

    let (ops_a, _) = doc1.insert_text(ParticipantId(1), 1, "x");
    let (ops_b, _) = doc2.insert_text(ParticipantId(2), 1, "y");

    doc1.apply_ops(&ops_b);
    doc2.apply_ops(&ops_a);

    assert_eq!(doc1.get_text(), doc2.get_text());
    assert!(doc1.get_text().contains("AxyB") || doc1.get_text().contains("Ayx"));
}

#[tokio::test]
async fn test_document_state_persists() {
    let storage = InMemoryStorageEngine::new();

    let mut doc = CrdtDocument::new(DocumentId("persist-integ".into()));
    doc.join(ParticipantId(1), "Alice");
    doc.insert_text(ParticipantId(1), 0, "Persisted content");

    let path = ".ferro-collab/persist-integ.crdt";
    let data = serde_json::to_vec(&doc).unwrap();
    storage
        .put(path, Bytes::from(data), "system")
        .await
        .unwrap();

    let loaded_bytes = storage.get(path).await.unwrap();
    let loaded: CrdtDocument = serde_json::from_slice(&loaded_bytes).unwrap();
    assert_eq!(loaded.get_text(), "Persisted content");
    assert_eq!(loaded.version, 1);
}

#[tokio::test]
async fn test_new_client_receives_document_state() {
    let (port, _) = setup_server().await;

    let mut ws_first = connect_client(port, "state-doc").await;
    let _hello = recv_message(&mut ws_first).await;

    join_client(&mut ws_first, "state-doc", 100, "First").await;

    let mut doc = CrdtDocument::new(DocumentId("state-doc".into()));
    doc.join(ParticipantId(100), "First");
    let (ops, _) = doc.insert_text(ParticipantId(100), 0, "Existing content");
    send_message(&mut ws_first, &CollabMessage::Operations { ops }).await;

    let mut ws_new = connect_client(port, "state-doc").await;
    let _hello = recv_message(&mut ws_new).await;

    send_message(
        &mut ws_new,
        &CollabMessage::Join {
            document_id: "state-doc".into(),
            participant_id: 200,
            name: "NewUser".into(),
        },
    )
    .await;

    let new_state = loop {
        let msg = recv_message(&mut ws_new).await;
        match msg {
            CollabMessage::DocumentState { .. } => break msg,
            CollabMessage::Hello { .. } | CollabMessage::Participants { .. } => continue,
            other => panic!("expected DocumentState, got {:?}", other),
        }
    };
    match new_state {
        CollabMessage::DocumentState {
            serialized_state, ..
        } => {
            let doc: CrdtDocument = serde_json::from_str(&serialized_state).unwrap();
            assert!(
                doc.get_text().contains("Existing content"),
                "new client should receive document with existing content, got: {:?}",
                doc.get_text()
            );
        }
        _ => panic!("expected DocumentState, got {:?}", new_state),
    }
}
