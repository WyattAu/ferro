use axum::extract::{Path, State, WebSocketUpgrade, ws::Message, ws::WebSocket};
use axum::response::Response;
use dashmap::DashMap;
use ferro_crdt::text::TextOperation;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

const BROADCAST_CAPACITY: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollabMessage {
    Join {
        document_id: String,
        participant_id: u32,
        name: String,
    },
    Operations {
        ops: Vec<TextOperation>,
    },
    State {
        document_id: String,
        version: u64,
    },
    Participants {
        participants: Vec<ParticipantEntry>,
    },
    Hello {
        participant_id: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantEntry {
    pub participant_id: u32,
    pub name: String,
}

#[derive(Debug)]
struct Room {
    tx: broadcast::Sender<String>,
    participants: DashMap<u32, String>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Room {
            tx,
            participants: DashMap::new(),
        }
    }

    fn participant_list(&self) -> Vec<ParticipantEntry> {
        self.participants
            .iter()
            .map(|r| ParticipantEntry {
                participant_id: *r.key(),
                name: r.value().clone(),
            })
            .collect()
    }

    fn broadcast_participants(&self) {
        let msg = CollabMessage::Participants {
            participants: self.participant_list(),
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.tx.send(json);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CollabRoomManager {
    rooms: Arc<DashMap<String, Arc<Room>>>,
}

impl CollabRoomManager {
    pub fn new() -> Self {
        CollabRoomManager {
            rooms: Arc::new(DashMap::new()),
        }
    }

    fn get_or_create_room(&self, document_id: &str) -> Arc<Room> {
        self.rooms
            .entry(document_id.to_string())
            .or_insert_with(|| Arc::new(Room::new()))
            .clone()
    }

    fn cleanup_room(&self, document_id: &str) {
        if let Some(entry) = self.rooms.get(document_id)
            && entry.participants.is_empty()
        {
            drop(entry);
            self.rooms
                .remove_if(document_id, |_, room| room.participants.is_empty());
        }
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    pub fn participant_count(&self, document_id: &str) -> usize {
        self.rooms
            .get(document_id)
            .map(|r| r.participants.len())
            .unwrap_or(0)
    }
}

impl Default for CollabRoomManager {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn collab_ws_handler(
    ws: WebSocketUpgrade,
    Path(document_id): Path<String>,
    State(state): State<crate::AppState>,
) -> Response {
    ws.on_upgrade(move |socket| {
        handle_collab_socket(socket, state.collab_rooms.clone(), document_id)
    })
}

async fn handle_collab_socket(socket: WebSocket, manager: CollabRoomManager, document_id: String) {
    let room = manager.get_or_create_room(&document_id);
    let mut rx = room.tx.subscribe();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let my_participant_id = Arc::new(std::sync::Mutex::new(None::<u32>));

    let hello_msg = CollabMessage::Hello { participant_id: 0 };
    if let Ok(json) = serde_json::to_string(&hello_msg) {
        let _ = ws_sender.send(Message::Text(json)).await;
    }

    let room_for_recv = room.clone();
    let pid_for_cleanup = my_participant_id.clone();
    let doc_id_for_cleanup = document_id.clone();
    let manager_for_cleanup = manager.clone();

    let recv_task = async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(collab_msg) = serde_json::from_str::<CollabMessage>(&text) {
                        match collab_msg {
                            CollabMessage::Join {
                                participant_id,
                                name,
                                ..
                            } => {
                                room_for_recv.participants.insert(participant_id, name);
                                *pid_for_cleanup.lock().unwrap() = Some(participant_id);
                                room_for_recv.broadcast_participants();
                                let _ = room_for_recv.tx.send(text);
                            }
                            CollabMessage::Operations { .. } => {
                                let _ = room_for_recv.tx.send(text);
                            }
                            _ => {}
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    };

    let send_task = async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }

    if let Some(pid) = *my_participant_id.lock().unwrap() {
        room.participants.remove(&pid);
        room.broadcast_participants();
    }

    manager_for_cleanup.cleanup_room(&doc_id_for_cleanup);
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
    fn test_get_or_create_room() {
        let manager = CollabRoomManager::new();
        let room = manager.get_or_create_room("doc-1");
        assert_eq!(manager.room_count(), 1);

        let _room2 = manager.get_or_create_room("doc-1");
        assert_eq!(manager.room_count(), 1);

        let _room3 = manager.get_or_create_room("doc-2");
        assert_eq!(manager.room_count(), 2);
    }

    #[test]
    fn test_participant_tracking() {
        let manager = CollabRoomManager::new();
        let room = manager.get_or_create_room("doc-1");

        room.participants.insert(1, "Alice".to_string());
        room.participants.insert(2, "Bob".to_string());
        assert_eq!(manager.participant_count("doc-1"), 2);

        room.participants.remove(&1);
        assert_eq!(manager.participant_count("doc-1"), 1);
    }

    #[test]
    fn test_cleanup_empty_room() {
        let manager = CollabRoomManager::new();
        let _room = manager.get_or_create_room("doc-1");
        assert_eq!(manager.room_count(), 1);

        manager.cleanup_room("doc-1");
        assert_eq!(manager.room_count(), 0);
    }

    #[test]
    fn test_cleanup_skips_nonempty_room() {
        let manager = CollabRoomManager::new();
        let room = manager.get_or_create_room("doc-1");
        room.participants.insert(1, "Alice".to_string());

        drop(room);
        manager.cleanup_room("doc-1");
        assert_eq!(manager.room_count(), 1);
    }

    #[tokio::test]
    async fn test_room_broadcast() {
        let manager = CollabRoomManager::new();
        let room = manager.get_or_create_room("doc-1");
        let mut rx = room.tx.subscribe();

        room.participants.insert(1, "Alice".to_string());
        room.broadcast_participants();

        let msg = rx.recv().await.unwrap();
        let parsed: CollabMessage = serde_json::from_str(&msg).unwrap();
        match parsed {
            CollabMessage::Participants { participants } => {
                assert_eq!(participants.len(), 1);
                assert_eq!(participants[0].participant_id, 1);
                assert_eq!(participants[0].name, "Alice");
            }
            _ => panic!("expected Participants message"),
        }
    }

    #[tokio::test]
    async fn test_operations_broadcast() {
        let manager = CollabRoomManager::new();
        let room = manager.get_or_create_room("doc-1");
        let mut rx = room.tx.subscribe();

        let ops_msg = CollabMessage::Operations { ops: vec![] };
        let json = serde_json::to_string(&ops_msg).unwrap();
        let _ = room.tx.send(json);

        let msg = rx.recv().await.unwrap();
        let parsed: CollabMessage = serde_json::from_str(&msg).unwrap();
        assert!(matches!(parsed, CollabMessage::Operations { .. }));
    }

    #[test]
    fn test_participant_list() {
        let room = Room::new();
        room.participants.insert(1, "Alice".to_string());
        room.participants.insert(2, "Bob".to_string());

        let list = room.participant_list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_message_serialization() {
        let msg = CollabMessage::Join {
            document_id: "doc-1".to_string(),
            participant_id: 42,
            name: "Alice".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("join"));
        assert!(json.contains("doc-1"));

        let parsed: CollabMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, CollabMessage::Join { .. }));
    }

    #[test]
    fn test_nonexistent_room_participant_count() {
        let manager = CollabRoomManager::new();
        assert_eq!(manager.participant_count("no-such-doc"), 0);
    }
}
