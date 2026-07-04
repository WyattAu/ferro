pub use ferro_server_api_core::ws::WsManager as WsManagerReexport;
pub use ferro_server_api_core::ws::{WsEvent, WsManager};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use futures::{SinkExt, StreamExt};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<crate::AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.ws_manager.clone()))
}

async fn handle_socket(socket: WebSocket, manager: std::sync::Arc<WsManager>) {
    let mut rx = manager.subscribe();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let send_task = async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
        manager.unsubscribe();
    };

    let recv_task = async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_) => {}
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    };
}
