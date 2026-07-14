use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    pub id: String,
    pub name: String,
    pub event: String,
    pub path_prefix: Option<String>,
    pub path_pattern: Option<String>,
    pub action: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTriggerRequest {
    pub name: String,
    pub event: String,
    pub path_prefix: Option<String>,
    pub path_pattern: Option<String>,
    pub action: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

fn trigger_store() -> &'static TriggerStore {
    use std::sync::OnceLock;
    static STORE: OnceLock<TriggerStore> = OnceLock::new();
    STORE.get_or_init(TriggerStore::new)
}

pub struct TriggerStore {
    triggers: Arc<RwLock<Vec<EventTrigger>>>,
    db: Option<crate::DbHandle>,
}

impl TriggerStore {
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

    pub async fn add(&self, trigger: EventTrigger) {
        let mut triggers = self.triggers.write().await;
        triggers.push(trigger.clone());
        while triggers.len() > 100 {
            triggers.remove(0);
        }
        self.persist_add(&trigger);
    }

    pub async fn remove(&self, id: &str) -> bool {
        let mut triggers = self.triggers.write().await;
        if let Some(pos) = triggers.iter().position(|t| t.id == id) {
            triggers.remove(pos);
            self.persist_delete(id);
            return true;
        }
        false
    }

    pub async fn list(&self) -> Vec<EventTrigger> {
        self.triggers.read().await.iter().cloned().collect()
    }

    pub async fn find_matching(&self, event: &str, path: &str) -> Vec<EventTrigger> {
        let triggers = self.triggers.read().await;
        triggers
            .iter()
            .filter(|t| {
                if !t.enabled || t.event != event {
                    return false;
                }
                if let Some(ref prefix) = t.path_prefix
                    && !path.starts_with(prefix.as_str())
                {
                    return false;
                }
                if let Some(ref pattern) = t.path_pattern
                    && !simple_glob_match(pattern, path)
                {
                    return false;
                }
                true
            })
            .cloned()
            .collect()
    }

    fn persist_add(&self, trigger: &EventTrigger) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            let config_str = serde_json::to_string(&trigger.config).unwrap_or_default();
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO event_triggers (id, name, event, path_prefix, path_pattern, action, config, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![trigger.id, trigger.name, trigger.event, trigger.path_prefix, trigger.path_pattern, trigger.action, config_str, trigger.enabled as i32, trigger.created_at],
            ) {
                tracing::warn!(error = %e, "failed to persist event trigger");
            }
        }
    }

    fn persist_delete(&self, id: &str) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute("DELETE FROM event_triggers WHERE id = ?1", params![id]) {
                tracing::warn!(error = %e, "failed to delete event trigger");
            }
        }
    }

    pub fn load_from_db(conn: &rusqlite::Connection) -> Result<Vec<EventTrigger>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, name, event, path_prefix, path_pattern, action, config, enabled, created_at FROM event_triggers",
        )?;
        let rows = stmt.query_map([], |row| {
            let config_str: String = row.get(6)?;
            let config: serde_json::Value = serde_json::from_str(&config_str).unwrap_or_default();
            Ok(EventTrigger {
                id: row.get(0)?,
                name: row.get(1)?,
                event: row.get(2)?,
                path_prefix: row.get(3)?,
                path_pattern: row.get(4)?,
                action: row.get(5)?,
                config,
                enabled: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?;
        let mut triggers = Vec::new();
        for row in rows {
            triggers.push(row?);
        }
        Ok(triggers)
    }
}

impl Default for TriggerStore {
    fn default() -> Self {
        Self::new()
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

pub async fn create_trigger(axum::Json(req): axum::Json<CreateTriggerRequest>) -> Response {
    let trigger = EventTrigger {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        event: req.event,
        path_prefix: req.path_prefix,
        path_pattern: req.path_pattern,
        action: req.action,
        config: req.config,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    trigger_store().add(trigger.clone()).await;

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": trigger.id,
            "name": trigger.name,
            "event": trigger.event,
            "path_prefix": trigger.path_prefix,
            "path_pattern": trigger.path_pattern,
            "action": trigger.action,
            "enabled": true,
        })),
    )
        .into_response()
}

pub async fn list_triggers() -> Response {
    let triggers = trigger_store().list().await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "triggers": triggers }))).into_response()
}

pub async fn delete_trigger(Path(id): Path<String>) -> Response {
    if trigger_store().remove(&id).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        error_not_found("Trigger not found")
    }
}

pub async fn toggle_trigger(Path(id): Path<String>) -> Response {
    let store = trigger_store();
    let mut triggers = store.triggers.write().await;
    if let Some(trigger) = triggers.iter_mut().find(|t| t.id == id) {
        trigger.enabled = !trigger.enabled;
        let enabled = trigger.enabled;
        drop(triggers);

        if let Some(ref db) = store.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute(
                "UPDATE event_triggers SET enabled = ?1 WHERE id = ?2",
                params![enabled as i32, id],
            ) {
                tracing::warn!(error = %e, "failed to toggle trigger");
            }
        }

        (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "id": id, "enabled": enabled })),
        )
            .into_response()
    } else {
        error_not_found("Trigger not found")
    }
}

pub async fn evaluate_triggers(event_type: &str, path: &str, _size: Option<u64>) {
    let matching = trigger_store().find_matching(event_type, path).await;
    for trigger in matching {
        tracing::info!(
            trigger_id = %trigger.id,
            trigger_name = %trigger.name,
            event = %trigger.event,
            path = %path,
            "event trigger matched"
        );
        match trigger.action.as_str() {
            "tag" => {
                tracing::info!(path = %path, trigger = %trigger.name, "tag trigger evaluated");
            }
            "notification" => {
                tracing::info!(path = %path, trigger = %trigger.name, "notification trigger evaluated");
            }
            _ => {
                tracing::debug!(
                    action = %trigger.action,
                    "unknown trigger action, skipping"
                );
            }
        }
    }
}

fn simple_glob_match(pattern: &str, path: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == path;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        let (prefix, suffix) = (parts[0], parts[1]);
        path.starts_with(prefix) && path.ends_with(suffix)
    } else {
        parts.len() == 1 && parts[0].is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_glob_match() {
        assert!(simple_glob_match("*.pdf", "/docs/report.pdf"));
        assert!(simple_glob_match("*.pdf", "/docs/invoice.pdf"));
        assert!(!simple_glob_match("*.pdf", "/docs/report.docx"));
        assert!(simple_glob_match("/docs/*", "/docs/anything"));
        assert!(simple_glob_match("/docs/*", "/docs/sub/file.txt"));
        assert!(!simple_glob_match("/docs/*", "/other/file.txt"));
        assert!(simple_glob_match("*", "anything"));
        assert!(simple_glob_match("", ""));
        assert!(!simple_glob_match("", "something"));
        assert!(simple_glob_match("exact", "exact"));
        assert!(!simple_glob_match("exact", "other"));
    }

    #[test]
    fn test_trigger_store_basic() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = TriggerStore::new();
            let trigger = EventTrigger {
                id: "test-1".to_string(),
                name: "PDF Tag".to_string(),
                event: "file.upload".to_string(),
                path_prefix: Some("/documents".to_string()),
                path_pattern: Some("*.pdf".to_string()),
                action: "tag".to_string(),
                config: serde_json::json!({"tags": ["auto-tagged"]}),
                enabled: true,
                created_at: "2026-01-01T00:00:00Z".to_string(),
            };
            store.add(trigger).await;

            let matching = store.find_matching("file.upload", "/documents/report.pdf").await;
            assert_eq!(matching.len(), 1);

            let no_match = store.find_matching("file.upload", "/images/photo.jpg").await;
            assert!(no_match.is_empty());
        });
    }
}
