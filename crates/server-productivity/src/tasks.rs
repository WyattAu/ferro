use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::DbHandle;
use crate::ProductivityState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub assignee: String,
    pub due_date: Option<String>,
    pub priority: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub priority: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveTaskRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct TasksQuery {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub tag: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

fn row_to_task(row: &rusqlite::Row) -> Result<Task, rusqlite::Error> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        assignee: row.get(4)?,
        due_date: row.get(5)?,
        priority: row.get(6)?,
        tags: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

#[derive(Clone)]
pub struct TaskStore {
    db: Option<DbHandle>,
}

impl Default for TaskStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    fn ensure_tasks_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'todo',
                assignee TEXT NOT NULL DEFAULT '',
                due_date TEXT,
                priority TEXT NOT NULL DEFAULT 'medium',
                tags TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
            CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee);
            CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date);",
        )
    }

    pub fn list(&self, query: &TasksQuery) -> Result<Vec<Task>, String> {
        let db = self.db.as_ref().ok_or("Database not configured")?;
        let conn = db.lock().map_err(|e| format!("Lock error: {}", e))?;
        Self::ensure_tasks_table(&conn).map_err(|e| format!("DB error: {}", e))?;

        let mut conditions = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref status) = query.status
            && !status.is_empty()
        {
            conditions.push(format!("status = ?{}", conditions.len() + 1));
            values.push(Box::new(status.clone()));
        }
        if let Some(ref assignee) = query.assignee
            && !assignee.is_empty()
        {
            conditions.push(format!("assignee = ?{}", conditions.len() + 1));
            values.push(Box::new(assignee.clone()));
        }
        if let Some(ref priority) = query.priority
            && !priority.is_empty()
        {
            conditions.push(format!("priority = ?{}", conditions.len() + 1));
            values.push(Box::new(priority.clone()));
        }
        if let Some(ref tag) = query.tag
            && !tag.is_empty()
        {
            conditions.push(format!("tags LIKE ?{}", conditions.len() + 1));
            values.push(Box::new(format!("%{}%", tag)));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sort_by = query.sort.as_deref().unwrap_or("created_at");
        let order = query.order.as_deref().unwrap_or("desc");
        let sort_column = match sort_by {
            "title" => "title",
            "priority" => "priority",
            "due_date" => "due_date",
            "status" => "status",
            _ => "created_at",
        };

        let sql = format!(
            "SELECT id, title, description, status, assignee, due_date, priority, tags, created_at, updated_at
             FROM tasks {} ORDER BY {} {}",
            where_clause, sort_column, order
        );

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();

        let mut tasks = Vec::new();
        match conn.prepare(&sql) {
            Ok(mut stmt) => {
                if let Ok(rows) = stmt.query_map(params_refs.as_slice(), row_to_task) {
                    for row in rows.flatten() {
                        tasks.push(row);
                    }
                }
            }
            Err(e) => {
                return Err(format!("Query error: {}", e));
            }
        }

        Ok(tasks)
    }

    pub fn get(&self, id: &str) -> Result<Option<Task>, String> {
        let db = self.db.as_ref().ok_or("Database not configured")?;
        let conn = db.lock().map_err(|e| format!("Lock error: {}", e))?;
        Self::ensure_tasks_table(&conn).map_err(|e| format!("DB error: {}", e))?;

        let task: Option<Task> = conn
            .query_row(
                "SELECT id, title, description, status, assignee, due_date, priority, tags, created_at, updated_at
                 FROM tasks WHERE id = ?1",
                rusqlite::params![id],
                row_to_task,
            )
            .ok();

        Ok(task)
    }

    pub fn create(&self, req: &CreateTaskRequest) -> Result<Task, String> {
        let db = self.db.as_ref().ok_or("Database not configured")?;
        let conn = db.lock().map_err(|e| format!("Lock error: {}", e))?;
        Self::ensure_tasks_table(&conn).map_err(|e| format!("DB error: {}", e))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let task = Task {
            id: id.clone(),
            title: req.title.clone().unwrap_or_else(|| "Untitled".to_string()),
            description: req.description.clone().unwrap_or_default(),
            status: req.status.clone().unwrap_or_else(|| "todo".to_string()),
            assignee: req.assignee.clone().unwrap_or_default(),
            due_date: req.due_date.clone(),
            priority: req.priority.clone().unwrap_or_else(|| "medium".to_string()),
            tags: req.tags.clone().unwrap_or_default(),
            created_at: now.clone(),
            updated_at: now,
        };

        conn.execute(
            "INSERT INTO tasks (id, title, description, status, assignee, due_date, priority, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                task.id,
                task.title,
                task.description,
                task.status,
                task.assignee,
                task.due_date,
                task.priority,
                task.tags,
                task.created_at,
                task.updated_at,
            ],
        )
        .map_err(|e| format!("Insert error: {}", e))?;

        Ok(task)
    }

    pub fn update(&self, id: &str, req: &UpdateTaskRequest) -> Result<Option<Task>, String> {
        let db = self.db.as_ref().ok_or("Database not configured")?;
        let conn = db.lock().map_err(|e| format!("Lock error: {}", e))?;
        Self::ensure_tasks_table(&conn).map_err(|e| format!("DB error: {}", e))?;

        let existing: Option<Task> = conn
            .query_row(
                "SELECT id, title, description, status, assignee, due_date, priority, tags, created_at, updated_at
                 FROM tasks WHERE id = ?1",
                rusqlite::params![id],
                row_to_task,
            )
            .ok();

        let existing = match existing {
            Some(t) => t,
            None => return Ok(None),
        };

        let now = chrono::Utc::now().to_rfc3339();
        let task = Task {
            id: existing.id,
            title: req.title.clone().unwrap_or(existing.title),
            description: req.description.clone().unwrap_or(existing.description),
            status: req.status.clone().unwrap_or(existing.status),
            assignee: req.assignee.clone().unwrap_or(existing.assignee),
            due_date: req.due_date.clone().or(existing.due_date),
            priority: req.priority.clone().unwrap_or(existing.priority),
            tags: req.tags.clone().unwrap_or(existing.tags),
            created_at: existing.created_at,
            updated_at: now,
        };

        conn.execute(
            "UPDATE tasks SET title=?1, description=?2, status=?3, assignee=?4, due_date=?5,
             priority=?6, tags=?7, updated_at=?8 WHERE id=?9",
            rusqlite::params![
                task.title,
                task.description,
                task.status,
                task.assignee,
                task.due_date,
                task.priority,
                task.tags,
                task.updated_at,
                task.id,
            ],
        )
        .map_err(|e| format!("Update error: {}", e))?;

        Ok(Some(task))
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let db = self.db.as_ref().ok_or("Database not configured")?;
        let conn = db.lock().map_err(|e| format!("Lock error: {}", e))?;
        Self::ensure_tasks_table(&conn).map_err(|e| format!("DB error: {}", e))?;

        let affected = conn
            .execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![id])
            .unwrap_or(0);

        Ok(affected > 0)
    }

    pub fn move_task(&self, id: &str, new_status: &str) -> Result<Option<Task>, String> {
        let db = self.db.as_ref().ok_or("Database not configured")?;
        let conn = db.lock().map_err(|e| format!("Lock error: {}", e))?;
        Self::ensure_tasks_table(&conn).map_err(|e| format!("DB error: {}", e))?;

        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![new_status, now, id],
            )
            .unwrap_or(0);

        if affected == 0 {
            return Ok(None);
        }

        let task: Option<Task> = conn
            .query_row(
                "SELECT id, title, description, status, assignee, due_date, priority, tags, created_at, updated_at
                 FROM tasks WHERE id = ?1",
                rusqlite::params![id],
                row_to_task,
            )
            .ok();

        Ok(task)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_tasks<S: ProductivityState>(
    State(state): State<S>,
    Query(params): Query<TasksQuery>,
) -> impl IntoResponse {
    match state.task_store().list(&params) {
        Ok(tasks) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "tasks": tasks,
                "total": tasks.len(),
            })),
        )
            .into_response(),
        Err(e) if e == "Database not configured" => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": e}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

pub async fn create_task<S: ProductivityState>(
    State(state): State<S>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    match state.task_store().create(&req) {
        Ok(task) => (StatusCode::CREATED, Json(serde_json::json!(task))).into_response(),
        Err(e) if e == "Database not configured" => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": e}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

pub async fn update_task<S: ProductivityState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    match state.task_store().update(&id, &req) {
        Ok(Some(task)) => Json(serde_json::json!(task)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) if e == "Database not configured" => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": e}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

pub async fn delete_task<S: ProductivityState>(State(state): State<S>, Path(id): Path<String>) -> impl IntoResponse {
    match state.task_store().delete(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) if e == "Database not configured" => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": e}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

pub async fn move_task<S: ProductivityState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<MoveTaskRequest>,
) -> impl IntoResponse {
    match state.task_store().move_task(&id, &req.status) {
        Ok(Some(task)) => Json(serde_json::json!(task)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) if e == "Database not configured" => {
            (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": e}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn setup_db() -> TaskStore {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        TaskStore::new().with_db(Arc::new(std::sync::Mutex::new(conn)))
    }

    fn full_req(title: &str) -> CreateTaskRequest {
        CreateTaskRequest {
            title: Some(title.to_string()),
            description: None,
            status: None,
            assignee: None,
            due_date: None,
            priority: None,
            tags: None,
        }
    }

    // ── create ───────────────────────────────────────────────────────────

    #[test]
    fn create_task_sets_defaults() {
        let store = setup_db();
        let task = store
            .create(&CreateTaskRequest {
                title: None,
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        assert_eq!(task.title, "Untitled");
        assert_eq!(task.status, "todo");
        assert_eq!(task.priority, "medium");
        assert!(task.assignee.is_empty());
        assert!(task.tags.is_empty());
        assert!(task.description.is_empty());
        assert!(task.due_date.is_none());
        assert!(!task.id.is_empty());
        assert!(!task.created_at.is_empty());
        assert_eq!(task.created_at, task.updated_at);
    }

    #[test]
    fn create_task_with_all_fields() {
        let store = setup_db();
        let task = store
            .create(&CreateTaskRequest {
                title: Some("Full".into()),
                description: Some("Desc".into()),
                status: Some("in_progress".into()),
                assignee: Some("alice".into()),
                due_date: Some("2025-06-01".into()),
                priority: Some("high".into()),
                tags: Some("a,b".into()),
            })
            .unwrap();
        assert_eq!(task.title, "Full");
        assert_eq!(task.description, "Desc");
        assert_eq!(task.status, "in_progress");
        assert_eq!(task.assignee, "alice");
        assert_eq!(task.due_date.as_deref(), Some("2025-06-01"));
        assert_eq!(task.priority, "high");
        assert_eq!(task.tags, "a,b");
    }

    #[test]
    fn create_task_special_characters() {
        let store = setup_db();
        let task = store
            .create(&CreateTaskRequest {
                title: Some("Task <with> & \"special\" 'chars'".into()),
                description: Some("Line1\nLine2\tTab".into()),
                status: None,
                assignee: Some("user@domain.com".into()),
                due_date: None,
                priority: None,
                tags: Some("tag1, tag with spaces, café".into()),
            })
            .unwrap();
        let found = store.get(&task.id).unwrap().unwrap();
        assert_eq!(found.title, "Task <with> & \"special\" 'chars'");
        assert_eq!(found.description, "Line1\nLine2\tTab");
        assert_eq!(found.assignee, "user@domain.com");
        assert_eq!(found.tags, "tag1, tag with spaces, café");
    }

    // ── get ──────────────────────────────────────────────────────────────

    #[test]
    fn get_existing_task() {
        let store = setup_db();
        let created = store.create(&full_req("Find me")).unwrap();
        let found = store.get(&created.id).unwrap().unwrap();
        assert_eq!(found.id, created.id);
        assert_eq!(found.title, "Find me");
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let store = setup_db();
        assert!(store.get("no-such-id").unwrap().is_none());
    }

    #[test]
    fn get_empty_string_id() {
        let store = setup_db();
        assert!(store.get("").unwrap().is_none());
    }

    // ── list ─────────────────────────────────────────────────────────────

    #[test]
    fn list_empty_table() {
        let store = setup_db();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn list_returns_all() {
        let store = setup_db();
        store.create(&full_req("A")).unwrap();
        store.create(&full_req("B")).unwrap();
        store.create(&full_req("C")).unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn list_filter_by_status() {
        let store = setup_db();
        store
            .create(&CreateTaskRequest {
                title: Some("todo".into()),
                description: None,
                status: Some("todo".into()),
                assignee: None,
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        store
            .create(&CreateTaskRequest {
                title: Some("done".into()),
                description: None,
                status: Some("done".into()),
                assignee: None,
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: Some("todo".into()),
                assignee: None,
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "todo");
    }

    #[test]
    fn list_filter_by_assignee() {
        let store = setup_db();
        store
            .create(&CreateTaskRequest {
                title: Some("a".into()),
                description: None,
                status: None,
                assignee: Some("alice".into()),
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        store
            .create(&CreateTaskRequest {
                title: Some("b".into()),
                description: None,
                status: None,
                assignee: Some("bob".into()),
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: Some("alice".into()),
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].assignee, "alice");
    }

    #[test]
    fn list_filter_by_priority() {
        let store = setup_db();
        store
            .create(&CreateTaskRequest {
                title: Some("h".into()),
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: Some("high".into()),
                tags: None,
            })
            .unwrap();
        store
            .create(&CreateTaskRequest {
                title: Some("l".into()),
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: Some("low".into()),
                tags: None,
            })
            .unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: Some("high".into()),
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].priority, "high");
    }

    #[test]
    fn list_filter_by_tag() {
        let store = setup_db();
        store
            .create(&CreateTaskRequest {
                title: Some("tagged".into()),
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: None,
                tags: Some("rust,backend".into()),
            })
            .unwrap();
        store
            .create(&CreateTaskRequest {
                title: Some("untagged".into()),
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: Some("rust".into()),
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "tagged");
    }

    #[test]
    fn list_sort_by_title_asc() {
        let store = setup_db();
        store.create(&full_req("Banana")).unwrap();
        store.create(&full_req("Apple")).unwrap();
        store.create(&full_req("Cherry")).unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: None,
                sort: Some("title".into()),
                order: Some("asc".into()),
            })
            .unwrap();
        let titles: Vec<_> = tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["Apple", "Banana", "Cherry"]);
    }

    #[test]
    fn list_sort_by_priority_desc() {
        let store = setup_db();
        store
            .create(&CreateTaskRequest {
                title: Some("low".into()),
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: Some("low".into()),
                tags: None,
            })
            .unwrap();
        store
            .create(&CreateTaskRequest {
                title: Some("high".into()),
                description: None,
                status: None,
                assignee: None,
                due_date: None,
                priority: Some("high".into()),
                tags: None,
            })
            .unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: None,
                sort: Some("priority".into()),
                order: Some("desc".into()),
            })
            .unwrap();
        assert_eq!(tasks[0].title, "low");
        assert_eq!(tasks[1].title, "high");
    }

    #[test]
    fn list_empty_filter_matches_nothing() {
        let store = setup_db();
        store.create(&full_req("Task")).unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: Some("nonexistent".into()),
                assignee: None,
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn list_filter_ignores_empty_strings() {
        let store = setup_db();
        store.create(&full_req("Task")).unwrap();
        let tasks = store
            .list(&TasksQuery {
                status: Some("".into()),
                assignee: Some("".into()),
                priority: Some("".into()),
                tag: Some("".into()),
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(tasks.len(), 1);
    }

    // ── update ───────────────────────────────────────────────────────────

    #[test]
    fn update_task_all_fields() {
        let store = setup_db();
        let created = store.create(&full_req("Original")).unwrap();
        let updated = store
            .update(
                &created.id,
                &UpdateTaskRequest {
                    title: Some("New".into()),
                    description: Some("Desc".into()),
                    status: Some("done".into()),
                    assignee: Some("bob".into()),
                    due_date: Some("2025-12-25".into()),
                    priority: Some("critical".into()),
                    tags: Some("x".into()),
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.title, "New");
        assert_eq!(updated.description, "Desc");
        assert_eq!(updated.status, "done");
        assert_eq!(updated.assignee, "bob");
        assert_eq!(updated.due_date.as_deref(), Some("2025-12-25"));
        assert_eq!(updated.priority, "critical");
        assert_eq!(updated.tags, "x");
        assert_eq!(updated.created_at, created.created_at);
        assert_ne!(updated.updated_at, created.updated_at);
    }

    #[test]
    fn update_task_partial_fields() {
        let store = setup_db();
        let created = store
            .create(&CreateTaskRequest {
                title: Some("Title".into()),
                description: Some("Desc".into()),
                status: Some("todo".into()),
                assignee: Some("alice".into()),
                due_date: None,
                priority: Some("low".into()),
                tags: Some("old".into()),
            })
            .unwrap();
        let updated = store
            .update(
                &created.id,
                &UpdateTaskRequest {
                    title: Some("New Title".into()),
                    description: None,
                    status: None,
                    assignee: None,
                    due_date: None,
                    priority: None,
                    tags: None,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.description, "Desc");
        assert_eq!(updated.status, "todo");
        assert_eq!(updated.assignee, "alice");
        assert_eq!(updated.priority, "low");
        assert_eq!(updated.tags, "old");
    }

    #[test]
    fn update_nonexistent_returns_none() {
        let store = setup_db();
        let result = store
            .update(
                "ghost",
                &UpdateTaskRequest {
                    title: Some("X".into()),
                    description: None,
                    status: None,
                    assignee: None,
                    due_date: None,
                    priority: None,
                    tags: None,
                },
            )
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn update_preserves_created_at() {
        let store = setup_db();
        let created = store.create(&full_req("T")).unwrap();
        let original_created = created.created_at.clone();
        let updated = store
            .update(
                &created.id,
                &UpdateTaskRequest {
                    title: Some("T2".into()),
                    description: None,
                    status: None,
                    assignee: None,
                    due_date: None,
                    priority: None,
                    tags: None,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.created_at, original_created);
    }

    // ── delete ───────────────────────────────────────────────────────────

    #[test]
    fn delete_existing_task() {
        let store = setup_db();
        let created = store.create(&full_req("Gone")).unwrap();
        assert!(store.delete(&created.id).unwrap());
        assert!(store.get(&created.id).unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let store = setup_db();
        assert!(!store.delete("nope").unwrap());
    }

    #[test]
    fn delete_empty_id() {
        let store = setup_db();
        assert!(!store.delete("").unwrap());
    }

    #[test]
    fn delete_does_not_affect_others() {
        let store = setup_db();
        let a = store.create(&full_req("A")).unwrap();
        let _b = store.create(&full_req("B")).unwrap();
        store.delete(&a.id).unwrap();
        let remaining = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "B");
    }

    // ── move_task ────────────────────────────────────────────────────────

    #[test]
    fn move_task_changes_status() {
        let store = setup_db();
        let created = store
            .create(&CreateTaskRequest {
                title: Some("Movable".into()),
                description: None,
                status: Some("todo".into()),
                assignee: None,
                due_date: None,
                priority: None,
                tags: None,
            })
            .unwrap();
        let moved = store.move_task(&created.id, "done").unwrap().unwrap();
        assert_eq!(moved.status, "done");
        assert_ne!(moved.updated_at, created.updated_at);
    }

    #[test]
    fn move_task_nonexistent_returns_none() {
        let store = setup_db();
        assert!(store.move_task("ghost", "done").unwrap().is_none());
    }

    #[test]
    fn move_task_updates_updated_at() {
        let store = setup_db();
        let created = store.create(&full_req("T")).unwrap();
        let moved = store.move_task(&created.id, "review").unwrap().unwrap();
        assert_ne!(moved.updated_at, created.updated_at);
    }

    // ── no-database errors ───────────────────────────────────────────────

    #[test]
    fn no_db_list_error() {
        let store = TaskStore::new();
        let err = store
            .list(&TasksQuery {
                status: None,
                assignee: None,
                priority: None,
                tag: None,
                sort: None,
                order: None,
            })
            .unwrap_err();
        assert_eq!(err, "Database not configured");
    }

    #[test]
    fn no_db_get_error() {
        let store = TaskStore::new();
        assert!(store.get("x").unwrap_err().contains("not configured"));
    }

    #[test]
    fn no_db_create_error() {
        let store = TaskStore::new();
        assert!(store.create(&full_req("x")).unwrap_err().contains("not configured"));
    }

    #[test]
    fn no_db_update_error() {
        let store = TaskStore::new();
        assert!(
            store
                .update(
                    "x",
                    &UpdateTaskRequest {
                        title: None,
                        description: None,
                        status: None,
                        assignee: None,
                        due_date: None,
                        priority: None,
                        tags: None,
                    }
                )
                .unwrap_err()
                .contains("not configured")
        );
    }

    #[test]
    fn no_db_delete_error() {
        let store = TaskStore::new();
        assert!(store.delete("x").unwrap_err().contains("not configured"));
    }

    #[test]
    fn no_db_move_error() {
        let store = TaskStore::new();
        assert!(store.move_task("x", "y").unwrap_err().contains("not configured"));
    }

    // ── default impl ─────────────────────────────────────────────────────

    #[test]
    fn default_task_store_has_no_db() {
        let store = TaskStore::default();
        assert!(
            store
                .list(&TasksQuery {
                    status: None,
                    assignee: None,
                    priority: None,
                    tag: None,
                    sort: None,
                    order: None,
                })
                .is_err()
        );
    }

    // ── with_db chaining ─────────────────────────────────────────────────

    #[test]
    fn with_db_returns_self() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let store = TaskStore::new().with_db(Arc::new(std::sync::Mutex::new(conn)));
        assert!(
            store
                .list(&TasksQuery {
                    status: None,
                    assignee: None,
                    priority: None,
                    tag: None,
                    sort: None,
                    order: None,
                })
                .is_ok()
        );
    }
}
