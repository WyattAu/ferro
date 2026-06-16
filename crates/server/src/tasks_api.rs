use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::AppState;

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

pub async fn list_tasks(
    State(state): State<AppState>,
    Query(params): Query<TasksQuery>,
) -> impl IntoResponse {
    let Some(ref db) = state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Database not configured"})),
        )
            .into_response();
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
            )
                .into_response()
        }
    };

    if let Err(e) = ensure_tasks_table(&conn) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("DB error: {}", e)})),
        )
            .into_response();
    }

    let mut conditions = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref status) = params.status {
        if !status.is_empty() {
            conditions.push(format!("status = ?{}", conditions.len() + 1));
            values.push(Box::new(status.clone()));
        }
    }
    if let Some(ref assignee) = params.assignee {
        if !assignee.is_empty() {
            conditions.push(format!("assignee = ?{}", conditions.len() + 1));
            values.push(Box::new(assignee.clone()));
        }
    }
    if let Some(ref priority) = params.priority {
        if !priority.is_empty() {
            conditions.push(format!("priority = ?{}", conditions.len() + 1));
            values.push(Box::new(priority.clone()));
        }
    }
    if let Some(ref tag) = params.tag {
        if !tag.is_empty() {
            conditions.push(format!("tags LIKE ?{}", conditions.len() + 1));
            values.push(Box::new(format!("%{}%", tag)));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sort_by = params.sort.as_deref().unwrap_or("created_at");
    let order = params.order.as_deref().unwrap_or("desc");
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Query error: {}", e)})),
            )
                .into_response()
        }
    }

    Json(serde_json::json!({
        "tasks": tasks,
        "total": tasks.len(),
    }))
    .into_response()
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    let Some(ref db) = state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Database not configured"})),
        )
            .into_response();
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
            )
                .into_response()
        }
    };

    if let Err(e) = ensure_tasks_table(&conn) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("DB error: {}", e)})),
        )
            .into_response();
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let task = Task {
        id: id.clone(),
        title: req.title.unwrap_or_else(|| "Untitled".to_string()),
        description: req.description.unwrap_or_default(),
        status: req.status.unwrap_or_else(|| "todo".to_string()),
        assignee: req.assignee.unwrap_or_default(),
        due_date: req.due_date,
        priority: req.priority.unwrap_or_else(|| "medium".to_string()),
        tags: req.tags.unwrap_or_default(),
        created_at: now.clone(),
        updated_at: now,
    };

    if let Err(e) = conn.execute(
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
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Insert error: {}", e)})),
        )
            .into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!(task))).into_response()
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    let Some(ref db) = state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Database not configured"})),
        )
            .into_response();
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
            )
                .into_response()
        }
    };

    if let Err(e) = ensure_tasks_table(&conn) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("DB error: {}", e)})),
        )
            .into_response();
    }

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
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Task not found"})),
            )
                .into_response()
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    let task = Task {
        id: existing.id,
        title: req.title.unwrap_or(existing.title),
        description: req.description.unwrap_or(existing.description),
        status: req.status.unwrap_or(existing.status),
        assignee: req.assignee.unwrap_or(existing.assignee),
        due_date: req.due_date.or(existing.due_date),
        priority: req.priority.unwrap_or(existing.priority),
        tags: req.tags.unwrap_or(existing.tags),
        created_at: existing.created_at,
        updated_at: now,
    };

    if let Err(e) = conn.execute(
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
    ) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Update error: {}", e)})),
        )
            .into_response();
    }

    Json(serde_json::json!(task)).into_response()
}

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let Some(ref db) = state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Database not configured"})),
        )
            .into_response();
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
            )
                .into_response()
        }
    };

    if let Err(e) = ensure_tasks_table(&conn) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("DB error: {}", e)})),
        )
            .into_response();
    }

    let affected = conn
        .execute("DELETE FROM tasks WHERE id = ?1", rusqlite::params![id])
        .unwrap_or(0);

    if affected == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

pub async fn move_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<MoveTaskRequest>,
) -> impl IntoResponse {
    let Some(ref db) = state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Database not configured"})),
        )
            .into_response();
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Lock error: {}", e)})),
            )
                .into_response()
        }
    };

    if let Err(e) = ensure_tasks_table(&conn) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("DB error: {}", e)})),
        )
            .into_response();
    }

    let now = chrono::Utc::now().to_rfc3339();
    let affected = conn
        .execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![req.status, now, id],
        )
        .unwrap_or(0);

    if affected == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Task not found"})),
        )
            .into_response();
    }

    let task: Option<Task> = conn
        .query_row(
            "SELECT id, title, description, status, assignee, due_date, priority, tags, created_at, updated_at
             FROM tasks WHERE id = ?1",
            rusqlite::params![id],
            row_to_task,
        )
        .ok();

    match task {
        Some(t) => Json(serde_json::json!(t)).into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to read updated task"})),
        )
            .into_response(),
    }
}
