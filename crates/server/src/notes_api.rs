use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub folder: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub folder: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub folder: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub folder: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct NotesQuery {
    pub q: Option<String>,
    pub folder: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

fn notes_dir(state: &AppState) -> std::path::PathBuf {
    let base = state
        .data_dir
        .as_deref()
        .unwrap_or(".ferro");
    std::path::PathBuf::from(base).join("notes")
}

fn ensure_notes_dir(state: &AppState) -> Result<std::path::PathBuf, (StatusCode, Json<serde_json::Value>)> {
    let dir = notes_dir(state);
    std::fs::create_dir_all(&dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create notes directory: {}", e)})),
        )
    })?;
    Ok(dir)
}

fn read_note_from_file(path: &std::path::Path) -> Option<Note> {
    let content = std::fs::read_to_string(path).ok()?;
    let filename = path.file_stem()?.to_str()?;
    let id = filename.to_string();

    let mut title = String::new();
    let mut folder = String::new();
    let mut tags = String::new();
    let mut created_at = String::new();
    let mut updated_at = String::new();
    let mut body = String::new();
    let mut in_frontmatter = false;

    for line in content.lines() {
        if line == "---" {
            if in_frontmatter {
                in_frontmatter = false;
                continue;
            } else if title.is_empty() && body.is_empty() {
                in_frontmatter = true;
                continue;
            }
        }
        if in_frontmatter {
            if let Some(v) = line.strip_prefix("title: ") {
                title = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("folder: ") {
                folder = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("tags: ") {
                tags = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("created_at: ") {
                created_at = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("updated_at: ") {
                updated_at = v.trim().to_string();
            }
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }

    if title.is_empty() {
        title = id.replace('_', " ");
    }

    let meta = path.metadata().ok();
    if created_at.is_empty() {
        created_at = meta
            .as_ref()
            .and_then(|m| m.created().ok())
            .and_then(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                Some(dt.to_rfc3339())
            })
            .unwrap_or_default();
    }
    if updated_at.is_empty() {
        updated_at = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                Some(dt.to_rfc3339())
            })
            .unwrap_or_default();
    }

    Some(Note {
        id,
        title,
        content: body.trim().to_string(),
        folder,
        tags,
        created_at,
        updated_at,
    })
}

fn write_note_to_file(path: &std::path::Path, note: &Note) -> Result<(), std::io::Error> {
    let content = format!(
        "---\ntitle: {}\nfolder: {}\ntags: {}\ncreated_at: {}\nupdated_at: {}\n---\n{}\n",
        note.title, note.folder, note.tags, note.created_at, note.updated_at, note.content
    );
    std::fs::write(path, content)
}

pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<NotesQuery>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(&state) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let mut notes: Vec<NoteMeta> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(note) = read_note_from_file(&path) {
                    notes.push(NoteMeta {
                        id: note.id,
                        title: note.title,
                        folder: note.folder,
                        tags: note.tags,
                        created_at: note.created_at,
                        updated_at: note.updated_at,
                    });
                }
            }
        }
    }

    // Filter by folder
    if let Some(ref folder) = params.folder {
        if !folder.is_empty() {
            notes.retain(|n| n.folder == *folder);
        }
    }

    // Search filter
    if let Some(ref q) = params.q {
        if !q.is_empty() {
            let q_lower = q.to_lowercase();
            notes.retain(|n| {
                n.title.to_lowercase().contains(&q_lower)
                    || n.tags.to_lowercase().contains(&q_lower)
            });
        }
    }

    // Sort
    let sort_by = params.sort.as_deref().unwrap_or("updated_at");
    let order = params.order.as_deref().unwrap_or("desc");
    notes.sort_by(|a, b| {
        let cmp = match sort_by {
            "title" => a.title.cmp(&b.title),
            "created_at" => a.created_at.cmp(&b.created_at),
            _ => a.updated_at.cmp(&b.updated_at),
        };
        if order == "asc" {
            cmp.reverse()
        } else {
            cmp
        }
    });

    Json(serde_json::json!({
        "notes": notes,
        "total": notes.len(),
    }))
    .into_response()
}

pub async fn get_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(&state) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let path = dir.join(format!("{}.md", id));
    match read_note_from_file(&path) {
        Some(note) => Json(serde_json::json!(note)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Note not found"})),
        )
            .into_response(),
    }
}

pub async fn create_note(
    State(state): State<AppState>,
    Json(req): Json<CreateNoteRequest>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(&state) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let note = Note {
        id: id.clone(),
        title: req.title.unwrap_or_else(|| "Untitled".to_string()),
        content: req.content.unwrap_or_default(),
        folder: req.folder.unwrap_or_default(),
        tags: req.tags.unwrap_or_default(),
        created_at: now.clone(),
        updated_at: now,
    };

    let path = dir.join(format!("{}.md", id));
    if let Err(e) = write_note_to_file(&path, &note) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to write note: {}", e)})),
        )
            .into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!(note))).into_response()
}

pub async fn update_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNoteRequest>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(&state) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let path = dir.join(format!("{}.md", id));
    let existing = match read_note_from_file(&path) {
        Some(n) => n,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Note not found"})),
            )
                .into_response()
        }
    };

    let note = Note {
        id: existing.id,
        title: req.title.unwrap_or(existing.title),
        content: req.content.unwrap_or(existing.content),
        folder: req.folder.unwrap_or(existing.folder),
        tags: req.tags.unwrap_or(existing.tags),
        created_at: existing.created_at,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    if let Err(e) = write_note_to_file(&path, &note) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to write note: {}", e)})),
        )
            .into_response();
    }

    Json(serde_json::json!(note)).into_response()
}

pub async fn delete_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(&state) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let path = dir.join(format!("{}.md", id));
    if !path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Note not found"})),
        )
            .into_response();
    }

    match std::fs::remove_file(&path) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to delete note: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn search_notes(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").cloned().unwrap_or_default();
    if q.is_empty() {
        return list_notes(State(state), Query(NotesQuery::default())).await.into_response();
    }

    let dir = match ensure_notes_dir(&state) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let q_lower = q.to_lowercase();
    let mut notes: Vec<NoteMeta> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(note) = read_note_from_file(&path) {
                    if note.title.to_lowercase().contains(&q_lower)
                        || note.content.to_lowercase().contains(&q_lower)
                        || note.tags.to_lowercase().contains(&q_lower)
                    {
                        notes.push(NoteMeta {
                            id: note.id,
                            title: note.title,
                            folder: note.folder,
                            tags: note.tags,
                            created_at: note.created_at,
                            updated_at: note.updated_at,
                        });
                    }
                }
            }
        }
    }

    Json(serde_json::json!({
        "notes": notes,
        "total": notes.len(),
        "query": q,
    }))
    .into_response()
}
