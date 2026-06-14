use axum::extract::{Path, State, WebSocketUpgrade, ws::Message, ws::WebSocket};
use axum::response::Response;
use bytes::Bytes;
use common::storage::StorageEngine;
use dashmap::DashMap;
use ferro_crdt::document::{CrdtDocument, DocumentId, ParticipantId};
use ferro_crdt::text::TextOperation;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

const BROADCAST_CAPACITY: usize = 256;
const IDLE_SAVE_INTERVAL_SECS: u64 = 30;

fn collab_storage_path(document_id: &str) -> String {
    format!(".ferro-collab/{}.crdt", document_id)
}

async fn load_crdt_bytes(storage: &dyn StorageEngine, document_id: &str) -> Option<Vec<u8>> {
    let path = collab_storage_path(document_id);
    storage.get(&path).await.ok().map(|b| b.to_vec())
}

#[allow(dead_code)]
async fn load_crdt_state(storage: &dyn StorageEngine, document_id: &str) -> Option<CrdtDocument> {
    let bytes = load_crdt_bytes(storage, document_id).await?;
    serde_json::from_slice(&bytes).ok()
}

async fn save_crdt_state(storage: &dyn StorageEngine, document_id: &str, data: &[u8]) {
    let path = collab_storage_path(document_id);
    let _ = storage
        .put(&path, Bytes::from(data.to_vec()), "system")
        .await;
}

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
    DocumentState {
        document_id: String,
        serialized_state: String,
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
    document: std::sync::RwLock<CrdtDocument>,
    dirty: AtomicBool,
    load_state: std::sync::Mutex<bool>,
}

impl Room {
    fn new(document_id: &str) -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Room {
            tx,
            participants: DashMap::new(),
            document: std::sync::RwLock::new(CrdtDocument::new(DocumentId(
                document_id.to_string(),
            ))),
            dirty: AtomicBool::new(false),
            load_state: std::sync::Mutex::new(false),
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
            .or_insert_with(|| Arc::new(Room::new(document_id)))
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
    let storage = state.storage.clone();
    ws.on_upgrade(move |socket| {
        handle_collab_socket(socket, state.collab_rooms.clone(), storage, document_id)
    })
}

async fn idle_save_loop(room: Arc<Room>, storage: Arc<dyn StorageEngine>, document_id: String) {
    let mut interval =
        tokio::time::interval(std::time::Duration::from_secs(IDLE_SAVE_INTERVAL_SECS));
    loop {
        interval.tick().await;
        if room.participants.is_empty() {
            break;
        }
        if room.dirty.swap(false, Ordering::SeqCst) {
            let data = serde_json::to_vec(&*room.document.read().unwrap_or_else(|e| e.into_inner())).ok();
            if let Some(data) = data {
                save_crdt_state(&*storage, &document_id, &data).await;
            }
        }
    }
}

async fn handle_collab_socket(
    socket: WebSocket,
    manager: CollabRoomManager,
    storage: Arc<dyn StorageEngine>,
    document_id: String,
) {
    let room = manager.get_or_create_room(&document_id);

    let should_load = {
        let mut guard = room.load_state.lock().unwrap_or_else(|e| e.into_inner());
        if *guard {
            false
        } else {
            *guard = true;
            true
        }
    };
    if should_load {
        if let Some(bytes) = load_crdt_bytes(&*storage, &document_id).await
            && let Ok(doc) = serde_json::from_slice(&bytes)
        {
            *room.document.write().unwrap_or_else(|e| e.into_inner()) = doc;
        }
        let _ = storage.create_collection(".ferro-collab", "system").await;
        let idle_room = room.clone();
        let idle_storage = storage.clone();
        let idle_doc_id = document_id.clone();
        tokio::spawn(async move {
            idle_save_loop(idle_room, idle_storage, idle_doc_id).await;
        });
    }

    let mut rx = room.tx.subscribe();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let my_participant_id = Arc::new(std::sync::Mutex::new(None::<u32>));

    let hello_msg = CollabMessage::Hello { participant_id: 0 };
    if let Ok(json) = serde_json::to_string(&hello_msg) {
        let _ = ws_sender.send(Message::Text(json)).await;
    }

    let (direct_tx, mut direct_rx) = tokio::sync::mpsc::channel::<String>(32);

    let room_for_recv = room.clone();
    let pid_for_cleanup = my_participant_id.clone();
    let doc_id_for_recv = document_id.clone();
    let direct_tx_for_recv = direct_tx.clone();

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
                                room_for_recv
                                    .participants
                                    .insert(participant_id, name.clone());
                                {
                                    let mut doc = room_for_recv.document.write().unwrap_or_else(|e| e.into_inner());
                                    doc.join(ParticipantId(participant_id), &name);
                                }
                                *pid_for_cleanup.lock().unwrap_or_else(|e| e.into_inner()) = Some(participant_id);

                                let serialized = {
                                    let doc = room_for_recv.document.read().unwrap_or_else(|e| e.into_inner());
                                    serde_json::to_string(&*doc).unwrap_or_default()
                                };
                                let state_msg = CollabMessage::DocumentState {
                                    document_id: doc_id_for_recv.clone(),
                                    serialized_state: serialized,
                                };
                                if let Ok(json) = serde_json::to_string(&state_msg) {
                                    let _ = direct_tx_for_recv.send(json).await;
                                }

                                room_for_recv.broadcast_participants();
                                let _ = room_for_recv.tx.send(text);
                            }
                            CollabMessage::Operations { ops } => {
                                {
                                    let mut doc = room_for_recv.document.write().unwrap_or_else(|e| e.into_inner());
                                    doc.apply_ops(&ops);
                                }
                                room_for_recv.dirty.store(true, Ordering::SeqCst);
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
        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            if ws_sender.send(Message::Text(msg)).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                msg = direct_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if ws_sender.send(Message::Text(msg)).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    };

    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }

    if let Some(pid) = *my_participant_id.lock().unwrap_or_else(|e| e.into_inner()) {
        room.participants.remove(&pid);
        room.broadcast_participants();
    }

    if room.dirty.swap(false, Ordering::SeqCst) {
        let data = serde_json::to_vec(&*room.document.read().unwrap_or_else(|e| e.into_inner())).ok();
        if let Some(data) = data {
            save_crdt_state(&*storage, &document_id, &data).await;
        }
    }

    drop(direct_tx);
    manager.cleanup_room(&document_id);
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
        let room = Room::new("test-doc");
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

    #[test]
    fn test_room_crdt_document() {
        let room = Room::new("crdt-doc");
        {
            let mut doc = room.document.write().unwrap();
            doc.join(ParticipantId(1), "Alice");
            doc.insert_text(ParticipantId(1), 0, "Hello");
        }
        let doc = room.document.read().unwrap();
        assert_eq!(doc.get_text(), "Hello");
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_room_dirty_flag() {
        let room = Room::new("dirty-doc");
        assert!(!room.dirty.load(Ordering::SeqCst));
        room.dirty.store(true, Ordering::SeqCst);
        assert!(room.dirty.load(Ordering::SeqCst));
    }

    #[test]
    fn test_document_state_serialization() {
        let msg = CollabMessage::DocumentState {
            document_id: "doc-1".to_string(),
            serialized_state: r#"{"id":{"0":"doc-1"}}"#.to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("document_state"));

        let parsed: CollabMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, CollabMessage::DocumentState { .. }));
    }

    #[tokio::test]
    async fn test_crdt_operations_in_room() {
        let room = Room::new("ops-doc");
        {
            let mut doc = room.document.write().unwrap();
            doc.join(ParticipantId(1), "Alice");
            doc.join(ParticipantId(2), "Bob");
            let (ops, v1) = doc.insert_text(ParticipantId(1), 0, "Hello");
            assert_eq!(v1, 1);
            assert!(!ops.is_empty());
        }
        let doc = room.document.read().unwrap();
        assert_eq!(doc.get_text(), "Hello");
    }

    #[tokio::test]
    async fn test_persistence_roundtrip() {
        let storage = crate::storage::InMemoryStorageEngine::new();
        let mut doc = CrdtDocument::new(DocumentId("persist-doc".to_string()));
        doc.join(ParticipantId(1), "Alice");
        doc.insert_text(ParticipantId(1), 0, "Persist me");

        let data = serde_json::to_vec(&doc).unwrap();
        save_crdt_state(&storage, "persist-doc", &data).await;

        let loaded = load_crdt_state(&storage, "persist-doc").await;
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.get_text(), "Persist me");
        assert_eq!(loaded.version, 1);
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let storage = crate::storage::InMemoryStorageEngine::new();
        let loaded = load_crdt_state(&storage, "no-such-doc").await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_concurrent_crdt_convergence() {
        let mut doc1 = CrdtDocument::new(DocumentId("conv".to_string()));
        doc1.join(ParticipantId(1), "Alice");
        doc1.join(ParticipantId(2), "Bob");
        let (ops1, _) = doc1.insert_text(ParticipantId(1), 0, "AB");

        let mut doc2 = CrdtDocument::new(DocumentId("conv".to_string()));
        doc2.join(ParticipantId(1), "Alice");
        doc2.join(ParticipantId(2), "Bob");
        doc2.apply_ops(&ops1);

        let (ops_a, _) = doc1.insert_text(ParticipantId(1), 1, "x");
        let (ops_b, _) = doc2.insert_text(ParticipantId(2), 1, "y");

        doc1.apply_ops(&ops_b);
        doc2.apply_ops(&ops_a);

        assert_eq!(doc1.get_text(), doc2.get_text());
    }
}
