//! WebSocket manager — simplified for Phase 0.
//!
//! Full implementation in Phase 1 with auto-reconnect, message queue.

/// WebSocket connection state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WsState {
    Disconnected,
    Connected,
}

/// WebSocket event types from server.
#[derive(Clone, Debug)]
pub enum WsEvent {
    FileChanged { path: String, action: String },
    ChatMessage { room_id: String, message: String },
    Notification { title: String, body: String },
    Unknown(String),
}

/// WebSocket manager stub — full impl in Phase 1.
pub struct WsManager {
    state: std::rc::Rc<std::cell::RefCell<WsState>>,
}

impl WsManager {
    pub fn new(_url: &str) -> Self {
        Self {
            state: std::rc::Rc::new(std::cell::RefCell::new(WsState::Disconnected)),
        }
    }

    pub fn state(&self) -> WsState {
        self.state.borrow().clone()
    }

    pub fn connect(&self) {
        log::info!("WebSocket connect (stub)");
    }

    pub fn send(&self, _msg: &str) {
        log::debug!("WebSocket send (stub)");
    }
}

/// Parse a WebSocket message into a typed event.
pub fn parse_ws_event(text: &str) -> WsEvent {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        let action = json["action"].as_str().unwrap_or("");
        match action {
            "file.put" | "file.delete" | "file.move" => WsEvent::FileChanged {
                path: json["path"].as_str().unwrap_or("").to_string(),
                action: action.to_string(),
            },
            "chat.message" => WsEvent::ChatMessage {
                room_id: json["room_id"].as_str().unwrap_or("").to_string(),
                message: json["message"].as_str().unwrap_or("").to_string(),
            },
            "notification" => WsEvent::Notification {
                title: json["title"].as_str().unwrap_or("").to_string(),
                body: json["body"].as_str().unwrap_or("").to_string(),
            },
            _ => WsEvent::Unknown(text.to_string()),
        }
    } else {
        WsEvent::Unknown(text.to_string())
    }
}
