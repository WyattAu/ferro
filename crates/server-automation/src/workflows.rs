use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::AutomationState;
use crate::DbHandle;

const MAX_WORKFLOWS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub trigger: Trigger,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    OnUpload,
    OnDelete,
    OnShare,
    OnRename,
    Schedule { cron: String },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    FileType { mime_pattern: String },
    PathMatch { pattern: String },
    SizeRange { min: Option<u64>, max: Option<u64> },
    UserMatch { username: String },
    TagContains { tag: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Move { destination: String },
    Copy { destination: String },
    Rename { pattern: String },
    Notify { message: String, channel: String },
    Tag { tags: Vec<String> },
    Webhook { url: String, method: String, headers: Option<serde_json::Value> },
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub trigger: Trigger,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub trigger: Option<Trigger>,
    pub conditions: Option<Vec<Condition>>,
    pub actions: Option<Vec<Action>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub trigger_event: String,
    pub file_path: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct WorkflowStore {
    workflows: Arc<RwLock<Vec<Workflow>>>,
    executions: Arc<RwLock<Vec<WorkflowExecution>>>,
    db: Option<DbHandle>,
}

impl Default for WorkflowStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowStore {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(Vec::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub async fn add(&self, workflow: Workflow) -> Result<(), String> {
        let mut workflows = self.workflows.write().await;
        if workflows.len() >= MAX_WORKFLOWS {
            return Err("Workflow limit reached".to_string());
        }
        workflows.push(workflow.clone());
        self.persist_add(&workflow);
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Option<Workflow> {
        self.workflows.read().await.iter().find(|w| w.id == id).cloned()
    }

    pub async fn update(&self, id: &str, req: UpdateWorkflowRequest) -> Result<Workflow, String> {
        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.iter_mut().find(|w| w.id == id) {
            if let Some(name) = req.name {
                workflow.name = name;
            }
            if let Some(description) = req.description {
                workflow.description = description;
            }
            if let Some(trigger) = req.trigger {
                workflow.trigger = trigger;
            }
            if let Some(conditions) = req.conditions {
                workflow.conditions = conditions;
            }
            if let Some(actions) = req.actions {
                workflow.actions = actions;
            }
            if let Some(enabled) = req.enabled {
                workflow.enabled = enabled;
            }
            workflow.updated_at = chrono::Utc::now().to_rfc3339();
            let w = workflow.clone();
            self.persist_update(&w);
            Ok(w)
        } else {
            Err("Workflow not found".to_string())
        }
    }

    pub async fn delete(&self, id: &str) -> bool {
        let mut workflows = self.workflows.write().await;
        if let Some(pos) = workflows.iter().position(|w| w.id == id) {
            workflows.remove(pos);
            self.persist_delete(id);
            return true;
        }
        false
    }

    pub async fn list(&self) -> Vec<Workflow> {
        self.workflows.read().await.iter().cloned().collect()
    }

    pub async fn find_matching_triggers(&self, trigger: &Trigger) -> Vec<Workflow> {
        self.workflows
            .read()
            .await
            .iter()
            .filter(|w| w.enabled && std::mem::discriminant(&w.trigger) == std::mem::discriminant(trigger))
            .cloned()
            .collect()
    }

    pub async fn record_execution(&self, execution: WorkflowExecution) {
        let mut executions = self.executions.write().await;
        executions.push(execution);
        while executions.len() > 1000 {
            executions.remove(0);
        }
    }

    pub async fn list_executions(&self, workflow_id: Option<&str>) -> Vec<WorkflowExecution> {
        let executions = self.executions.read().await;
        match workflow_id {
            Some(id) => executions
                .iter()
                .filter(|e| e.workflow_id == id)
                .cloned()
                .collect(),
            None => executions.iter().cloned().collect(),
        }
    }

    fn persist_add(&self, workflow: &Workflow) {
        if let Some(ref db) = self.db {
            let db = db.clone();
            let w = workflow.clone();
            tokio::task::spawn_blocking(move || {
                let conn = db.lock().unwrap();
                let _ = conn.execute(
                    "INSERT INTO workflows (id, name, description, trigger_data, conditions_data, actions_data, enabled, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        w.id,
                        w.name,
                        w.description,
                        serde_json::to_string(&w.trigger).unwrap_or_default(),
                        serde_json::to_string(&w.conditions).unwrap_or_default(),
                        serde_json::to_string(&w.actions).unwrap_or_default(),
                        w.enabled,
                        w.created_at,
                        w.updated_at,
                    ],
                );
            });
        }
    }

    fn persist_update(&self, workflow: &Workflow) {
        if let Some(ref db) = self.db {
            let db = db.clone();
            let w = workflow.clone();
            tokio::task::spawn_blocking(move || {
                let conn = db.lock().unwrap();
                let _ = conn.execute(
                    "UPDATE workflows SET name = ?2, description = ?3, trigger_data = ?4, conditions_data = ?5, actions_data = ?6, enabled = ?7, updated_at = ?8 WHERE id = ?1",
                    params![
                        w.id,
                        w.name,
                        w.description,
                        serde_json::to_string(&w.trigger).unwrap_or_default(),
                        serde_json::to_string(&w.conditions).unwrap_or_default(),
                        serde_json::to_string(&w.actions).unwrap_or_default(),
                        w.enabled,
                        w.updated_at,
                    ],
                );
            });
        }
    }

    fn persist_delete(&self, id: &str) {
        if let Some(ref db) = self.db {
            let db = db.clone();
            let id = id.to_string();
            tokio::task::spawn_blocking(move || {
                let conn = db.lock().unwrap();
                let _ = conn.execute("DELETE FROM workflows WHERE id = ?1", params![id]);
            });
        }
    }
}

pub async fn create_workflow(
    axum::extract::State(_state): axum::extract::State<AutomationState>,
    axum::extract::Json(req): axum::extract::Json<CreateWorkflowRequest>,
) -> Response {
    let workflow = Workflow {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        description: req.description,
        trigger: req.trigger,
        conditions: req.conditions,
        actions: req.actions,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    match workflow_store().add(workflow.clone()).await {
        Ok(()) => (StatusCode::CREATED, axum::Json(workflow)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

pub async fn get_workflow(
    Path(id): Path<String>,
) -> Response {
    match workflow_store().get(&id).await {
        Some(workflow) => axum::Json(workflow).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn update_workflow(
    Path(id): Path<String>,
    axum::extract::Json(req): axum::extract::Json<UpdateWorkflowRequest>,
) -> Response {
    match workflow_store().update(&id, req).await {
        Ok(workflow) => axum::Json(workflow).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e).into_response(),
    }
}

pub async fn delete_workflow(
    Path(id): Path<String>,
) -> Response {
    if workflow_store().delete(&id).await {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn list_workflows() -> Response {
    let workflows = workflow_store().list().await;
    axum::Json(workflows).into_response()
}

pub async fn trigger_workflow(
    Path(id): Path<String>,
) -> Response {
    match workflow_store().get(&id).await {
        Some(workflow) => {
            let execution = WorkflowExecution {
                id: Uuid::new_v4().to_string(),
                workflow_id: workflow.id,
                status: "running".to_string(),
                trigger_event: "manual".to_string(),
                file_path: None,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: None,
                error: None,
            };
            workflow_store().record_execution(execution.clone()).await;
            axum::Json(execution).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn list_executions(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let workflow_id = params.get("workflow_id").map(|s| s.as_str());
    let executions = workflow_store().list_executions(workflow_id).await;
    axum::Json(executions).into_response()
}

fn workflow_store() -> &'static WorkflowStore {
    use std::sync::OnceLock;
    static STORE: OnceLock<WorkflowStore> = OnceLock::new();
    STORE.get_or_init(WorkflowStore::new)
}

pub fn init_workflow_store(db: DbHandle) {
    use std::sync::OnceLock;
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let conn = db.lock().unwrap();
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                trigger_data TEXT NOT NULL,
                conditions_data TEXT NOT NULL DEFAULT '[]',
                actions_data TEXT NOT NULL DEFAULT '[]',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workflow_executions (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                status TEXT NOT NULL,
                trigger_event TEXT NOT NULL,
                file_path TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                error TEXT,
                FOREIGN KEY (workflow_id) REFERENCES workflows(id)
            );",
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_serde() {
        let workflow = Workflow {
            id: "test-id".to_string(),
            name: "Test Workflow".to_string(),
            description: Some("A test workflow".to_string()),
            trigger: Trigger::OnUpload,
            conditions: vec![Condition::FileType {
                mime_pattern: "text/*".to_string(),
            }],
            actions: vec![Action::Notify {
                message: "File uploaded".to_string(),
                channel: "default".to_string(),
            }],
            enabled: true,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&workflow).unwrap();
        let parsed: Workflow = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test Workflow");
        assert!(parsed.enabled);
    }

    #[test]
    fn test_trigger_serde() {
        let trigger = Trigger::Schedule {
            cron: "0 12 * * *".to_string(),
        };
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("Schedule"));
    }
}
