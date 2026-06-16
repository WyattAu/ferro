use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRoom {
    pub id: String,
    pub name: String,
    pub room_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub room_id: String,
    pub user_id: String,
    pub content: String,
    pub timestamp: String,
    pub reply_to: Option<String>,
    pub attachment_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub room_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub reply_to: Option<String>,
    pub attachment_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct MessagesQuery {
    pub limit: Option<usize>,
    pub before: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TypingIndicator {
    pub user_id: String,
    pub is_typing: bool,
}

fn chat_dir(state: &AppState) -> std::path::PathBuf {
    let base = state.data_dir.as_deref().unwrap_or(".ferro");
    std::path::PathBuf::from(base).join("chat")
}

fn rooms_file(state: &AppState) -> std::path::PathBuf {
    chat_dir(state).join("rooms.json")
}

fn messages_file(state: &AppState, room_id: &str) -> std::path::PathBuf {
    chat_dir(state).join(format!("messages_{}.json", room_id))
}

fn ensure_chat_dir(state: &AppState) -> Result<std::path::PathBuf, (StatusCode, Json<serde_json::Value>)> {
    let dir = chat_dir(state);
    std::fs::create_dir_all(&dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create chat directory: {}", e)})),
        )
    })?;
    Ok(dir)
}

fn load_rooms(state: &AppState) -> Vec<ChatRoom> {
    let path = rooms_file(state);
    if !path.exists() {
        let mut rooms = Vec::new();
        let global_id = uuid::Uuid::new_v4().to_string();
        rooms.push(ChatRoom {
            id: global_id,
            name: "Global".to_string(),
            room_type: "global".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        let _ = save_rooms(state, &rooms);
        return rooms;
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_rooms(state: &AppState, rooms: &[ChatRoom]) -> Result<(), std::io::Error> {
    let path = rooms_file(state);
    std::fs::write(path, serde_json::to_string_pretty(rooms).unwrap_or_default())
}

fn load_messages(state: &AppState, room_id: &str) -> Vec<ChatMessage> {
    let path = messages_file(state, room_id);
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_messages(state: &AppState, room_id: &str, messages: &[ChatMessage]) -> Result<(), std::io::Error> {
    let _ = ensure_chat_dir(state);
    let path = messages_file(state, room_id);
    std::fs::write(path, serde_json::to_string_pretty(messages).unwrap_or_default())
}

pub async fn list_rooms(State(state): State<AppState>) -> impl IntoResponse {
    let rooms = load_rooms(&state);
    Json(serde_json::json!({
        "rooms": rooms,
        "total": rooms.len(),
    }))
    .into_response()
}

pub async fn create_room(
    State(state): State<AppState>,
    Json(req): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    if let Err(e) = ensure_chat_dir(&state) {
        return e.into_response();
    }

    let mut rooms = load_rooms(&state);
    let room = ChatRoom {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        room_type: req.room_type.unwrap_or_else(|| "global".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    rooms.push(room.clone());
    if let Err(e) = save_rooms(&state, &rooms) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save rooms: {}", e)})),
        )
            .into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!(room))).into_response()
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Query(params): Query<MessagesQuery>,
) -> impl IntoResponse {
    let all_messages = load_messages(&state, &room_id);
    let limit = params.limit.unwrap_or(50).min(200);

    let filtered: Vec<&ChatMessage> = if let Some(ref before) = params.before {
        all_messages
            .iter()
            .filter(|m| m.timestamp.as_str() < before.as_str())
            .collect()
    } else {
        all_messages.iter().collect()
    };

    let start = if filtered.len() > limit {
        filtered.len() - limit
    } else {
        0
    };
    let messages: Vec<&ChatMessage> = filtered[start..].to_vec();

    Json(serde_json::json!({
        "messages": messages,
        "total": all_messages.len(),
        "has_more": start > 0,
    }))
    .into_response()
}

pub async fn send_message(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    if let Err(e) = ensure_chat_dir(&state) {
        return e.into_response();
    }

    let mut messages = load_messages(&state, &room_id);

    let message = ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        room_id: room_id.clone(),
        user_id: "current-user".to_string(),
        content: req.content,
        timestamp: chrono::Utc::now().to_rfc3339(),
        reply_to: req.reply_to,
        attachment_path: req.attachment_path,
    };

    messages.push(message.clone());

    if let Err(e) = save_messages(&state, &room_id, &messages) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save message: {}", e)})),
        )
            .into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!(message))).into_response()
}

pub async fn ws_chat_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    Path(room_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_chat_ws(socket, room_id, state))
}

async fn handle_chat_ws(
    mut socket: axum::extract::ws::WebSocket,
    room_id: String,
    state: AppState,
) {
    use axum::extract::ws::Message;

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                    let msg_type = payload.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    match msg_type {
                        "message" => {
                            let content = payload.get("content").and_then(|c| c.as_str()).unwrap_or("");
                            let user_id = payload.get("user_id").and_then(|u| u.as_str()).unwrap_or("anonymous");
                            let reply_to = payload.get("reply_to").and_then(|r| r.as_str()).map(|s| s.to_string());
                            let attachment_path = payload.get("attachment_path").and_then(|a| a.as_str()).map(|s| s.to_string());

                            let mut messages = load_messages(&state, &room_id);
                            let message = ChatMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                room_id: room_id.clone(),
                                user_id: user_id.to_string(),
                                content: content.to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                reply_to,
                                attachment_path,
                            };

                            messages.push(message.clone());
                            let _ = save_messages(&state, &room_id, &messages);

                            if let Ok(response) = serde_json::to_string(&serde_json::json!({
                                "type": "message",
                                "message": message,
                            })) {
                                let _ = socket.send(Message::Text(response.into())).await;
                            }
                        }
                        "typing" => {
                            let user_id = payload.get("user_id").and_then(|u| u.as_str()).unwrap_or("anonymous");
                            let is_typing = payload.get("is_typing").and_then(|t| t.as_bool()).unwrap_or(false);

                            if let Ok(response) = serde_json::to_string(&serde_json::json!({
                                "type": "typing",
                                "user_id": user_id,
                                "is_typing": is_typing,
                            })) {
                                let _ = socket.send(Message::Text(response.into())).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
