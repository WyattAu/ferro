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

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            values.iter().map(|v| v.as_ref()).collect();

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
        Err(e) if e == "Database not configured" => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

pub async fn create_task<S: ProductivityState>(
    State(state): State<S>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    match state.task_store().create(&req) {
        Ok(task) => (StatusCode::CREATED, Json(serde_json::json!(task))).into_response(),
        Err(e) if e == "Database not configured" => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
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
        Err(e) if e == "Database not configured" => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

pub async fn delete_task<S: ProductivityState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.task_store().delete(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response(),
        Err(e) if e == "Database not configured" => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
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
        Err(e) if e == "Database not configured" => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}
