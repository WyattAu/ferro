use std::sync::Arc;
use tokio::sync::broadcast;

const MAX_WS_CONNECTIONS: usize = 1000;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    FileCreated {
        path: String,
        size: u64,
        owner: String,
    },
    FileUpdated {
        path: String,
        size: u64,
        owner: String,
    },
    FileDeleted {
        path: String,
        owner: String,
    },
    FileMoved {
        from: String,
        to: String,
        owner: String,
    },
    FileShared {
        path: String,
        token: String,
        owner: String,
    },
    SyncOp {
        clock: u64,
        op_type: String,
        path: String,
    },
    StorageHealth {
        healthy: bool,
        backend: String,
    },
}

#[derive(Debug, Clone)]
pub struct WsManager {
    tx: Arc<broadcast::Sender<String>>,
    connection_count: Arc<std::sync::atomic::AtomicU64>,
}

impl WsManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<String>(1024);
        Self {
            tx: Arc::new(tx),
            connection_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        let count = self
            .connection_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count as usize >= MAX_WS_CONNECTIONS {
            self.connection_count
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.tx.subscribe()
    }

    pub fn unsubscribe(&self) {
        self.connection_count
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn broadcast(&self, event: &WsEvent) {
        if let Ok(json) = serde_json::to_string(event) {
            let _ = self.tx.send(json);
        }
    }

    pub fn connection_count(&self) -> u64 {
        self.connection_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for WsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_manager_creation() {
        let manager = WsManager::new();
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn test_ws_event_serialization() {
        let event = WsEvent::FileCreated {
            path: "/test.txt".to_string(),
            size: 1024,
            owner: "admin".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("file_created"));
        assert!(json.contains("/test.txt"));
    }

    #[tokio::test]
    async fn test_ws_broadcast() {
        let manager = WsManager::new();
        let mut rx = manager.subscribe();

        manager.broadcast(&WsEvent::FileDeleted {
            path: "/old.txt".to_string(),
            owner: "admin".to_string(),
        });

        let msg = rx.recv().await.unwrap();
        assert!(msg.contains("file_deleted"));
    }

    #[test]
    fn test_ws_connection_count() {
        let manager = WsManager::new();
        assert_eq!(manager.connection_count(), 0);
        let _rx1 = manager.subscribe();
        assert_eq!(manager.connection_count(), 1);
        let _rx2 = manager.subscribe();
        assert_eq!(manager.connection_count(), 2);
    }

    #[test]
    fn test_ws_sync_op_event() {
        let event = WsEvent::SyncOp {
            clock: 42,
            op_type: "create".to_string(),
            path: "/file.txt".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("sync_op"));
        assert!(json.contains("\"clock\":42"));
    }
}
