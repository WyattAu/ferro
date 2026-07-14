use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventType {
    FileUploaded,
    FileDeleted,
    FileModified,
    ShareCreated,
    FileLocked,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileUploaded => "file.uploaded",
            Self::FileDeleted => "file.deleted",
            Self::FileModified => "file.modified",
            Self::ShareCreated => "share.created",
            Self::FileLocked => "file.locked",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "FileUploaded" | "file.uploaded" => Some(Self::FileUploaded),
            "FileDeleted" | "file.deleted" => Some(Self::FileDeleted),
            "FileModified" | "file.modified" => Some(Self::FileModified),
            "ShareCreated" | "share.created" => Some(Self::ShareCreated),
            "FileLocked" | "file.locked" => Some(Self::FileLocked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    pub id: String,
    pub event_type: EventType,
    pub worker_name: String,
    pub path_pattern: String,
    pub enabled: bool,
    pub created_at: String,
    pub success_count: u64,
    pub failure_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventTriggerRequest {
    pub event_type: EventType,
    pub worker_name: String,
    pub path_pattern: String,
}

#[allow(async_fn_in_trait)]
pub trait EventTriggerStore: Send + Sync {
    async fn add(&self, trigger: EventTrigger);
    async fn remove(&self, id: &str) -> bool;
    async fn list(&self) -> Vec<EventTrigger>;
    async fn toggle(&self, id: &str) -> bool;
    async fn find_matching(&self, event_type: EventType, path: &str) -> Vec<EventTrigger>;
    async fn record_success(&self, id: &str);
    async fn record_failure(&self, id: &str);
    fn load_from_db(&self, conn: &rusqlite::Connection);
}

fn trigger_store() -> &'static WasmEventTriggerStore {
    use std::sync::OnceLock;
    static STORE: OnceLock<WasmEventTriggerStore> = OnceLock::new();
    STORE.get_or_init(WasmEventTriggerStore::new)
}

pub struct WasmEventTriggerStore {
    triggers: Arc<RwLock<Vec<EventTrigger>>>,
    db: Option<crate::DbHandle>,
}

impl WasmEventTriggerStore {
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(RwLock::new(Vec::new())),
            db: None,
        }
    }

    pub fn with_db(mut self, db: crate::DbHandle) -> Self {
        self.db = Some(db);
        self
    }
}

impl Default for WasmEventTriggerStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventTriggerStore for WasmEventTriggerStore {
    async fn add(&self, trigger: EventTrigger) {
        let mut triggers = self.triggers.write().await;
        triggers.push(trigger.clone());
        while triggers.len() > 500 {
            triggers.remove(0);
        }
        self.persist_add(&trigger);
    }

    async fn remove(&self, id: &str) -> bool {
        let mut triggers = self.triggers.write().await;
        if let Some(pos) = triggers.iter().position(|t| t.id == id) {
            triggers.remove(pos);
            self.persist_delete(id);
            return true;
        }
        false
    }

    async fn list(&self) -> Vec<EventTrigger> {
        self.triggers.read().await.iter().cloned().collect()
    }

    async fn toggle(&self, id: &str) -> bool {
        let mut triggers = self.triggers.write().await;
        if let Some(trigger) = triggers.iter_mut().find(|t| t.id == id) {
            trigger.enabled = !trigger.enabled;
            let enabled = trigger.enabled;
            drop(triggers);
            if let Some(ref db) = self.db {
                let conn = db.lock().unwrap_or_else(|e| e.into_inner());
                if let Err(e) = conn.execute(
                    "UPDATE wasm_event_triggers SET enabled = ?1 WHERE id = ?2",
                    params![enabled as i32, id],
                ) {
                    tracing::warn!(error = %e, "failed to toggle wasm event trigger");
                }
            }
            return true;
        }
        false
    }

    async fn find_matching(&self, event_type: EventType, path: &str) -> Vec<EventTrigger> {
        let triggers = self.triggers.read().await;
        triggers
            .iter()
            .filter(|t| t.enabled && t.event_type == event_type && glob_match(&t.path_pattern, path))
            .cloned()
            .collect()
    }

    async fn record_success(&self, id: &str) {
        let mut triggers = self.triggers.write().await;
        if let Some(trigger) = triggers.iter_mut().find(|t| t.id == id) {
            trigger.success_count += 1;
        }
        drop(triggers);
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute(
                "UPDATE wasm_event_triggers SET success_count = success_count + 1 WHERE id = ?1",
                params![id],
            ) {
                tracing::warn!(error = %e, "failed to update trigger success count");
            }
        }
    }

    async fn record_failure(&self, id: &str) {
        let mut triggers = self.triggers.write().await;
        if let Some(trigger) = triggers.iter_mut().find(|t| t.id == id) {
            trigger.failure_count += 1;
        }
        drop(triggers);
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute(
                "UPDATE wasm_event_triggers SET failure_count = failure_count + 1 WHERE id = ?1",
                params![id],
            ) {
                tracing::warn!(error = %e, "failed to update trigger failure count");
            }
        }
    }

    fn load_from_db(&self, conn: &rusqlite::Connection) {
        let mut stmt = match conn.prepare(
            "SELECT id, event_type, worker_name, path_pattern, enabled, created_at, success_count, failure_count FROM wasm_event_triggers",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "failed to prepare wasm event triggers query");
                return;
            }
        };

        let rows = match stmt.query_map([], |row| {
            let event_type_str: String = row.get(1)?;
            let event_type = EventType::parse(&event_type_str).unwrap_or(EventType::FileUploaded);
            Ok(EventTrigger {
                id: row.get(0)?,
                event_type,
                worker_name: row.get(2)?,
                path_pattern: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
                success_count: row.get(6)?,
                failure_count: row.get(7)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "failed to load wasm event triggers from db");
                return;
            }
        };

        let loaded: Vec<EventTrigger> = rows.filter_map(|r| r.ok()).collect();
        let store = trigger_store();
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let mut triggers = store.triggers.write().await;
                for trigger in loaded {
                    triggers.push(trigger);
                }
            });
        });
    }
}

impl WasmEventTriggerStore {
    fn persist_add(&self, trigger: &EventTrigger) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO wasm_event_triggers (id, event_type, worker_name, path_pattern, enabled, created_at, success_count, failure_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    trigger.id,
                    trigger.event_type.as_str(),
                    trigger.worker_name,
                    trigger.path_pattern,
                    trigger.enabled as i32,
                    trigger.created_at,
                    trigger.success_count as i64,
                    trigger.failure_count as i64,
                ],
            ) {
                tracing::warn!(error = %e, "failed to persist wasm event trigger");
            }
        }
    }

    fn persist_delete(&self, id: &str) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute("DELETE FROM wasm_event_triggers WHERE id = ?1", params![id]) {
                tracing::warn!(error = %e, "failed to delete wasm event trigger");
            }
        }
    }
}

fn error_not_found(msg: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "NOT_FOUND",
        })),
    )
        .into_response()
}

pub async fn create_event_trigger(axum::Json(req): axum::Json<CreateEventTriggerRequest>) -> Response {
    let trigger = EventTrigger {
        id: uuid::Uuid::new_v4().to_string(),
        event_type: req.event_type,
        worker_name: req.worker_name,
        path_pattern: req.path_pattern,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        success_count: 0,
        failure_count: 0,
    };

    trigger_store().add(trigger.clone()).await;

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": trigger.id,
            "event_type": trigger.event_type.as_str(),
            "worker_name": trigger.worker_name,
            "path_pattern": trigger.path_pattern,
            "enabled": true,
            "created_at": trigger.created_at,
        })),
    )
        .into_response()
}

pub async fn list_event_triggers() -> Response {
    let triggers = trigger_store().list().await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "triggers": triggers }))).into_response()
}

pub async fn delete_event_trigger(Path(id): Path<String>) -> Response {
    if trigger_store().remove(&id).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        error_not_found("Trigger not found")
    }
}

pub async fn toggle_event_trigger(Path(id): Path<String>) -> Response {
    if trigger_store().toggle(&id).await {
        let triggers = trigger_store().list().await;
        let enabled = triggers.iter().find(|t| t.id == id).map(|t| t.enabled).unwrap_or(false);
        (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "id": id, "enabled": enabled })),
        )
            .into_response()
    } else {
        error_not_found("Trigger not found")
    }
}

pub async fn fire_event_triggers(state: &crate::AutomationState, event_type: EventType, path: &str, owner: &str) {
    let matching = trigger_store().find_matching(event_type, path).await;
    if matching.is_empty() {
        return;
    }

    let Some(runtime) = &state.wasm_runtime else {
        tracing::debug!("WASM runtime not configured, skipping event triggers");
        return;
    };

    for trigger in matching {
        let module_path = match find_module_path(&state.workers_dir, &trigger.worker_name) {
            Some(p) => p,
            None => {
                tracing::warn!(
                    trigger_id = %trigger.id,
                    worker_name = %trigger.worker_name,
                    "WASM module not found for event trigger"
                );
                trigger_store().record_failure(&trigger.id).await;
                continue;
            }
        };

        let module_path_str = module_path.to_string_lossy().to_string();
        let runtime = runtime.clone();
        let path_owned = path.to_string();
        let trigger_id = trigger.id.clone();
        let trigger_event_type = trigger.event_type.as_str().to_string();
        let owner_owned = owner.to_string();
        let dispatch_count = state.wasm_dispatch_count.clone();
        let error_count = state.wasm_error_count.clone();
        let fuel_total = state.wasm_fuel_total.clone();

        tokio::spawn(async move {
            let event_data = serde_json::json!({
                "event_type": trigger_event_type,
                "path": path_owned,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "owner": owner_owned,
            });
            let input = event_data.to_string().into_bytes();

            dispatch_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            match runtime.execute(&module_path_str, "handle_event", &input, None).await {
                Ok(result) => {
                    fuel_total.fetch_add(result.fuel_consumed, std::sync::atomic::Ordering::Relaxed);
                    if result.success {
                        tracing::info!(
                            trigger_id = %trigger_id,
                            module = %module_path_str,
                            path = %path_owned,
                            fuel = result.fuel_consumed,
                            "event trigger executed successfully"
                        );
                        trigger_store().record_success(&trigger_id).await;
                    } else {
                        error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        tracing::warn!(
                            trigger_id = %trigger_id,
                            module = %module_path_str,
                            error = ?result.error,
                            "event trigger execution returned failure"
                        );
                        trigger_store().record_failure(&trigger_id).await;
                    }
                }
                Err(e) => {
                    error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    tracing::warn!(
                        trigger_id = %trigger_id,
                        module = %module_path_str,
                        error = %e,
                        "event trigger execution error"
                    );
                    trigger_store().record_failure(&trigger_id).await;
                }
            }
        });
    }
}

fn find_module_path(workers_dir: &Option<std::path::PathBuf>, worker_name: &str) -> Option<std::path::PathBuf> {
    let dir = workers_dir.as_ref()?;
    let expected_suffix = format!("-{}", worker_name);
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if (name == worker_name || name.ends_with(&expected_suffix))
                && entry.path().extension().map(|e| e == "wasm").unwrap_or(false)
            {
                return Some(entry.path());
            }
        }
    }
    None
}

fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    if !pattern.contains('*') {
        return pattern == path || path.starts_with(pattern);
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        let (prefix, suffix) = (parts[0], parts[1]);
        path.starts_with(prefix) && (suffix.is_empty() || path.ends_with(suffix))
    } else {
        parts.len() == 1 && parts[0].is_empty()
    }
}

pub fn load_triggers_from_db(conn: &rusqlite::Connection) {
    trigger_store().load_from_db(conn);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_roundtrip() {
        for (s, expected) in [
            ("FileUploaded", EventType::FileUploaded),
            ("FileDeleted", EventType::FileDeleted),
            ("FileModified", EventType::FileModified),
            ("ShareCreated", EventType::ShareCreated),
            ("FileLocked", EventType::FileLocked),
            ("file.uploaded", EventType::FileUploaded),
            ("file.deleted", EventType::FileDeleted),
            ("file.modified", EventType::FileModified),
            ("share.created", EventType::ShareCreated),
            ("file.locked", EventType::FileLocked),
        ] {
            assert_eq!(EventType::parse(s), Some(expected));
        }
        assert_eq!(EventType::parse("unknown"), None);
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.pdf", "/docs/report.pdf"));
        assert!(glob_match("*.pdf", "/docs/invoice.pdf"));
        assert!(!glob_match("*.pdf", "/docs/report.docx"));
        assert!(glob_match("/docs/*", "/docs/anything"));
        assert!(glob_match("/docs/*", "/docs/sub/file.txt"));
        assert!(!glob_match("/docs/*", "/other/file.txt"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("", ""));
        assert!(!glob_match("", "something"));
        assert!(glob_match("/docs/reports", "/docs/reports"));
        assert!(glob_match("/docs/reports", "/docs/reports/sub"));
    }

    #[tokio::test]
    async fn test_store_add_remove_list() {
        let store = WasmEventTriggerStore::new();
        let trigger = EventTrigger {
            id: "test-1".to_string(),
            event_type: EventType::FileUploaded,
            worker_name: "worker.wasm".to_string(),
            path_pattern: "*.pdf".to_string(),
            enabled: true,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            success_count: 0,
            failure_count: 0,
        };
        store.add(trigger.clone()).await;

        let all = store.list().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "test-1");

        assert!(store.remove("test-1").await);
        assert!(!store.remove("test-1").await);
        assert!(store.list().await.is_empty());
    }

    #[test]
    fn test_find_module_path_no_dir() {
        assert_eq!(find_module_path(&None, "worker.wasm"), None);
    }
}
