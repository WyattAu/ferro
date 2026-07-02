use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::Response;

pub use ferro_server_collaboration::collab_ws::CollabMessage;
pub use ferro_server_collaboration::collab_ws::CollabRoomManager;
pub use ferro_server_collaboration::collab_ws::ParticipantEntry;

pub async fn collab_ws_handler(
    ws: WebSocketUpgrade,
    Path(document_id): Path<String>,
    State(state): State<crate::AppState>,
) -> Response {
    ferro_server_collaboration::collab_ws::collab_ws_handler::<crate::AppState>(
        ws,
        Path(document_id),
        State(state),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_manager_create() {
        let manager = CollabRoomManager::new();
        assert_eq!(manager.room_count(), 0);
    }

    #[test]
    fn test_room_manager_participant_count() {
        let manager = CollabRoomManager::new();
        assert_eq!(manager.participant_count("doc1"), 0);
    }
}
