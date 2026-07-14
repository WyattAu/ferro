use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ProductivityState;

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

fn notes_dir(data_dir: Option<&str>) -> std::path::PathBuf {
    let base = data_dir.unwrap_or(".ferro");
    std::path::PathBuf::from(base).join("notes")
}

fn ensure_notes_dir(data_dir: Option<&str>) -> Result<std::path::PathBuf, (StatusCode, Json<serde_json::Value>)> {
    let dir = notes_dir(data_dir);
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
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();
    }
    if updated_at.is_empty() {
        updated_at = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
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

pub async fn list_notes<S: ProductivityState>(
    State(state): State<S>,
    Query(params): Query<NotesQuery>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(state.data_dir()) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let mut notes: Vec<NoteMeta> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md")
                && let Some(note) = read_note_from_file(&path)
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

    if let Some(ref folder) = params.folder
        && !folder.is_empty()
    {
        notes.retain(|n| n.folder == *folder);
    }

    if let Some(ref q) = params.q
        && !q.is_empty()
    {
        let q_lower = q.to_lowercase();
        notes.retain(|n| n.title.to_lowercase().contains(&q_lower) || n.tags.to_lowercase().contains(&q_lower));
    }

    let sort_by = params.sort.as_deref().unwrap_or("updated_at");
    let order = params.order.as_deref().unwrap_or("desc");
    notes.sort_by(|a, b| {
        let cmp = match sort_by {
            "title" => a.title.cmp(&b.title),
            "created_at" => a.created_at.cmp(&b.created_at),
            _ => a.updated_at.cmp(&b.updated_at),
        };
        if order == "asc" { cmp.reverse() } else { cmp }
    });

    Json(serde_json::json!({
        "notes": notes,
        "total": notes.len(),
    }))
    .into_response()
}

pub async fn get_note<S: ProductivityState>(State(state): State<S>, Path(id): Path<String>) -> impl IntoResponse {
    let dir = match ensure_notes_dir(state.data_dir()) {
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

pub async fn create_note<S: ProductivityState>(
    State(state): State<S>,
    Json(req): Json<CreateNoteRequest>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(state.data_dir()) {
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

pub async fn update_note<S: ProductivityState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNoteRequest>,
) -> impl IntoResponse {
    let dir = match ensure_notes_dir(state.data_dir()) {
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
                .into_response();
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

pub async fn delete_note<S: ProductivityState>(State(state): State<S>, Path(id): Path<String>) -> impl IntoResponse {
    let dir = match ensure_notes_dir(state.data_dir()) {
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

pub async fn search_notes<S: ProductivityState>(
    State(state): State<S>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").cloned().unwrap_or_default();
    if q.is_empty() {
        return list_notes(State(state), Query(NotesQuery::default()))
            .await
            .into_response();
    }

    let dir = match ensure_notes_dir(state.data_dir()) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };

    let q_lower = q.to_lowercase();
    let mut notes: Vec<NoteMeta> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md")
                && let Some(note) = read_note_from_file(&path)
                && (note.title.to_lowercase().contains(&q_lower)
                    || note.content.to_lowercase().contains(&q_lower)
                    || note.tags.to_lowercase().contains(&q_lower))
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

    Json(serde_json::json!({
        "notes": notes,
        "total": notes.len(),
        "query": q,
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    // ── Mock infrastructure ──────────────────────────────────────────────

    struct MockStorage;

    #[async_trait::async_trait]
    impl common::storage::StorageEngine for MockStorage {
        async fn head(&self, _: &str) -> common::error::Result<common::metadata::FileMetadata> {
            unimplemented!()
        }
        async fn get(&self, _: &str) -> common::error::Result<bytes::Bytes> {
            unimplemented!()
        }
        async fn put(
            &self,
            _: &str,
            _: bytes::Bytes,
            _: &str,
        ) -> common::error::Result<common::metadata::FileMetadata> {
            unimplemented!()
        }
        async fn delete(&self, _: &str) -> common::error::Result<()> {
            unimplemented!()
        }
        async fn list(&self, _: &str) -> common::error::Result<Vec<common::metadata::FileMetadata>> {
            unimplemented!()
        }
        async fn copy(&self, _: &str, _: &str) -> common::error::Result<()> {
            unimplemented!()
        }
        async fn move_path(&self, _: &str, _: &str) -> common::error::Result<()> {
            unimplemented!()
        }
        async fn exists(&self, _: &str) -> common::error::Result<bool> {
            unimplemented!()
        }
        async fn create_collection(&self, _: &str, _: &str) -> common::error::Result<common::metadata::FileMetadata> {
            unimplemented!()
        }
        async fn list_all(&self, _: &str, _: u32) -> common::error::Result<Vec<common::metadata::FileMetadata>> {
            unimplemented!()
        }
    }

    struct MockCalendarStore;
    #[async_trait::async_trait]
    impl ferro_dav::store::CalendarStore for MockCalendarStore {
        async fn list_calendars(&self, _: &str) -> Vec<ferro_dav::store::CalendarInfo> {
            vec![]
        }
        async fn get_calendar(&self, _: &str, _: &str) -> Option<ferro_dav::store::CalendarInfo> {
            None
        }
        async fn create_calendar(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> ferro_dav::store::StoreResult<ferro_dav::store::CalendarInfo> {
            unimplemented!()
        }
        async fn delete_calendar(&self, _: &str, _: &str) -> ferro_dav::store::StoreResult<()> {
            Ok(())
        }
        async fn list_events(&self, _: &str) -> Vec<ferro_dav::store::EventInfo> {
            vec![]
        }
        async fn get_event(&self, _: &str, _: &str) -> Option<ferro_dav::store::EventInfo> {
            None
        }
        async fn create_event(&self, _: &str, _: &str) -> ferro_dav::store::StoreResult<ferro_dav::store::EventInfo> {
            unimplemented!()
        }
        async fn update_event(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> ferro_dav::store::StoreResult<ferro_dav::store::EventInfo> {
            unimplemented!()
        }
        async fn delete_event(&self, _: &str, _: &str) -> ferro_dav::store::StoreResult<()> {
            Ok(())
        }
        async fn query_events(&self, _: &str, _: &ferro_dav::store::CalFilter) -> Vec<ferro_dav::store::EventInfo> {
            vec![]
        }
    }

    struct MockAddressBookStore;
    #[async_trait::async_trait]
    impl ferro_dav::store::AddressBookStore for MockAddressBookStore {
        async fn list_address_books(&self, _: &str) -> Vec<ferro_dav::store::AddressBookInfo> {
            vec![]
        }
        async fn get_address_book(&self, _: &str, _: &str) -> Option<ferro_dav::store::AddressBookInfo> {
            None
        }
        async fn create_address_book(
            &self,
            _: &str,
            _: &str,
        ) -> ferro_dav::store::StoreResult<ferro_dav::store::AddressBookInfo> {
            unimplemented!()
        }
        async fn delete_address_book(&self, _: &str, _: &str) -> ferro_dav::store::StoreResult<()> {
            Ok(())
        }
        async fn list_contacts(&self, _: &str) -> Vec<ferro_dav::store::ContactInfo> {
            vec![]
        }
        async fn get_contact(&self, _: &str, _: &str) -> Option<ferro_dav::store::ContactInfo> {
            None
        }
        async fn create_contact(
            &self,
            _: &str,
            _: &str,
        ) -> ferro_dav::store::StoreResult<ferro_dav::store::ContactInfo> {
            unimplemented!()
        }
        async fn update_contact(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> ferro_dav::store::StoreResult<ferro_dav::store::ContactInfo> {
            unimplemented!()
        }
        async fn delete_contact(&self, _: &str, _: &str) -> ferro_dav::store::StoreResult<()> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct MockState {
        data_dir: Option<String>,
        storage: Arc<dyn common::storage::StorageEngine>,
        calendar_store: Arc<dyn ferro_dav::store::CalendarStore>,
        address_book_store: Arc<dyn ferro_dav::store::AddressBookStore>,
        task_store: crate::tasks::TaskStore,
    }

    impl MockState {
        fn new(data_dir: &str) -> Self {
            Self {
                data_dir: Some(data_dir.to_string()),
                storage: Arc::new(MockStorage),
                calendar_store: Arc::new(MockCalendarStore),
                address_book_store: Arc::new(MockAddressBookStore),
                task_store: crate::tasks::TaskStore::new(),
            }
        }
    }

    impl common::server_context::HasStorage for MockState {
        fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
            &self.storage
        }
    }

    impl crate::ProductivityState for MockState {
        fn data_dir(&self) -> Option<&str> {
            self.data_dir.as_deref()
        }
        fn calendar_store(&self) -> &Arc<dyn ferro_dav::store::CalendarStore> {
            &self.calendar_store
        }
        fn address_book_store(&self) -> &Arc<dyn ferro_dav::store::AddressBookStore> {
            &self.address_book_store
        }
        fn task_store(&self) -> &crate::tasks::TaskStore {
            &self.task_store
        }
    }

    // ── Helper: write a raw .md file into the notes dir ──────────────────

    fn write_raw_note(dir: &std::path::Path, id: &str, content: &str) {
        std::fs::write(dir.join(format!("{}.md", id)), content).unwrap();
    }

    async fn response_body(resp: impl axum::response::IntoResponse) -> bytes::Bytes {
        let response = resp.into_response();
        axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap()
    }

    // ── notes_dir / ensure_notes_dir ─────────────────────────────────────

    #[test]
    fn notes_dir_default() {
        let dir = notes_dir(None);
        assert_eq!(dir, std::path::PathBuf::from(".ferro/notes"));
    }

    #[test]
    fn notes_dir_custom() {
        let dir = notes_dir(Some("/tmp/mydata"));
        assert_eq!(dir, std::path::PathBuf::from("/tmp/mydata/notes"));
    }

    #[test]
    fn ensure_notes_dir_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("sub");
        let dir = ensure_notes_dir(Some(base.to_str().unwrap())).unwrap();
        assert!(dir.exists());
        assert_eq!(dir, base.join("notes"));
    }

    #[test]
    fn ensure_notes_dir_idempotent() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("existing");
        std::fs::create_dir_all(&base).unwrap();
        let dir = ensure_notes_dir(Some(base.to_str().unwrap())).unwrap();
        assert!(dir.exists());
    }

    // ── read_note_from_file ──────────────────────────────────────────────

    #[test]
    fn read_note_full_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let content = "---\ntitle: My Note\nfolder: work\n\
            tags: rust,axum\ncreated_at: 2025-01-01T00:00:00Z\n\
            updated_at: 2025-06-01T00:00:00Z\n---\nHello world\n";
        write_raw_note(tmp.path(), "test1", content);
        let note = read_note_from_file(&tmp.path().join("test1.md")).unwrap();
        assert_eq!(note.id, "test1");
        assert_eq!(note.title, "My Note");
        assert_eq!(note.folder, "work");
        assert_eq!(note.tags, "rust,axum");
        assert_eq!(note.created_at, "2025-01-01T00:00:00Z");
        assert_eq!(note.updated_at, "2025-06-01T00:00:00Z");
        assert_eq!(note.content, "Hello world");
    }

    #[test]
    fn read_note_no_frontmatter() {
        let tmp = TempDir::new().unwrap();
        write_raw_note(tmp.path(), "plain", "Just content\n");
        let note = read_note_from_file(&tmp.path().join("plain.md")).unwrap();
        assert_eq!(note.id, "plain");
        assert_eq!(note.title, "plain");
        assert_eq!(note.content, "Just content");
    }

    #[test]
    fn read_note_partial_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let content = "---\ntitle: Partial\n---\nBody here\n";
        write_raw_note(tmp.path(), "partial", content);
        let note = read_note_from_file(&tmp.path().join("partial.md")).unwrap();
        assert_eq!(note.title, "Partial");
        assert!(note.folder.is_empty());
        assert!(note.tags.is_empty());
        assert_eq!(note.content, "Body here");
    }

    #[test]
    fn read_note_empty_file() {
        let tmp = TempDir::new().unwrap();
        write_raw_note(tmp.path(), "empty", "");
        // read_note_from_file returns None for files that can't be parsed
        // An empty file has no title in frontmatter, so title = filename
        let note = read_note_from_file(&tmp.path().join("empty.md")).unwrap();
        assert_eq!(note.title, "empty");
        assert!(note.content.is_empty());
    }

    #[test]
    fn read_note_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let result = read_note_from_file(&tmp.path().join("nope.md"));
        assert!(result.is_none());
    }

    #[test]
    fn read_note_multiline_body() {
        let tmp = TempDir::new().unwrap();
        let content = "---\ntitle: Multi\n---\nLine 1\nLine 2\nLine 3\n";
        write_raw_note(tmp.path(), "multi", content);
        let note = read_note_from_file(&tmp.path().join("multi.md")).unwrap();
        assert_eq!(note.content, "Line 1\nLine 2\nLine 3");
    }

    // ── write_note_to_file ───────────────────────────────────────────────

    #[test]
    fn write_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let note = Note {
            id: "rt".into(),
            title: "Roundtrip".into(),
            content: "Body".into(),
            folder: "f".into(),
            tags: "t1,t2".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            updated_at: "2025-06-01T00:00:00Z".into(),
        };
        let path = tmp.path().join("rt.md");
        write_note_to_file(&path, &note).unwrap();
        let read = read_note_from_file(&path).unwrap();
        assert_eq!(read.id, "rt");
        assert_eq!(read.title, "Roundtrip");
        assert_eq!(read.content, "Body");
        assert_eq!(read.folder, "f");
        assert_eq!(read.tags, "t1,t2");
    }

    #[test]
    fn write_note_special_characters() {
        let tmp = TempDir::new().unwrap();
        let note = Note {
            id: "sp".into(),
            title: "Title with \"quotes\" & <html>".into(),
            content: "Line1\nLine2".into(),
            folder: "".into(),
            tags: "".into(),
            created_at: "".into(),
            updated_at: "".into(),
        };
        let path = tmp.path().join("sp.md");
        write_note_to_file(&path, &note).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("Title with \"quotes\" & <html>"));
    }

    // ── Handler: list_notes ──────────────────────────────────────────────

    #[tokio::test]
    async fn list_notes_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = list_notes(State(state), Query(NotesQuery::default())).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 0);
        assert!(json["notes"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_notes_returns_all() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "a", "---\ntitle: A\n---\nBody A\n");
        write_raw_note(&notes_dir, "b", "---\ntitle: B\n---\nBody B\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = list_notes(State(state), Query(NotesQuery::default())).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 2);
    }

    #[tokio::test]
    async fn list_notes_filter_folder() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "w", "---\ntitle: W\nfolder: work\n---\n");
        write_raw_note(&notes_dir, "p", "---\ntitle: P\nfolder: personal\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = list_notes(
            State(state),
            Query(NotesQuery {
                folder: Some("work".into()),
                ..Default::default()
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["notes"][0]["title"], "W");
    }

    #[tokio::test]
    async fn list_notes_search_query() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "rust", "---\ntitle: Rust Guide\ntags: programming\n---\n");
        write_raw_note(&notes_dir, "cook", "---\ntitle: Cooking\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = list_notes(
            State(state),
            Query(NotesQuery {
                q: Some("rust".into()),
                ..Default::default()
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["notes"][0]["title"], "Rust Guide");
    }

    #[tokio::test]
    async fn list_notes_ignores_non_md_files() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "ok", "---\ntitle: OK\n---\n");
        std::fs::write(notes_dir.join("skip.txt"), "not a note").unwrap();

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = list_notes(State(state), Query(NotesQuery::default())).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
    }

    // ── Handler: get_note ────────────────────────────────────────────────

    #[tokio::test]
    async fn get_note_existing() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "myid", "---\ntitle: My\n---\nBody\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = get_note(State(state), Path("myid".into())).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["id"], "myid");
        assert_eq!(json["title"], "My");
    }

    #[tokio::test]
    async fn get_note_not_found() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = get_note(State(state), Path("nope".into())).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Note not found");
    }

    // ── Handler: create_note ─────────────────────────────────────────────

    #[tokio::test]
    async fn create_note_basic() {
        let tmp = TempDir::new().unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = create_note(
            State(state),
            Json(CreateNoteRequest {
                title: Some("New Note".into()),
                content: Some("Content".into()),
                folder: Some("f".into()),
                tags: Some("t".into()),
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["title"], "New Note");
        assert_eq!(json["content"], "Content");
        assert_eq!(json["folder"], "f");
        assert_eq!(json["tags"], "t");
        assert!(!json["id"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_note_defaults() {
        let tmp = TempDir::new().unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = create_note(
            State(state),
            Json(CreateNoteRequest {
                title: None,
                content: None,
                folder: None,
                tags: None,
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["title"], "Untitled");
        assert!(json["content"].as_str().unwrap().is_empty());
        assert!(json["folder"].as_str().unwrap().is_empty());
        assert!(json["tags"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_note_special_characters() {
        let tmp = TempDir::new().unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = create_note(
            State(state),
            Json(CreateNoteRequest {
                title: Some("Title with <html> & \"quotes\"".into()),
                content: Some("Line1\nLine2".into()),
                folder: None,
                tags: None,
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["title"], "Title with <html> & \"quotes\"");
        assert_eq!(json["content"], "Line1\nLine2");
    }

    // ── Handler: update_note ─────────────────────────────────────────────

    #[tokio::test]
    async fn update_note_existing() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "upd", "---\ntitle: Old\n---\nOld body\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = update_note(
            State(state),
            Path("upd".into()),
            Json(UpdateNoteRequest {
                title: Some("New".into()),
                content: Some("New body".into()),
                folder: None,
                tags: None,
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["title"], "New");
        assert_eq!(json["content"], "New body");
    }

    #[tokio::test]
    async fn update_note_not_found() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = update_note(
            State(state),
            Path("nope".into()),
            Json(UpdateNoteRequest {
                title: Some("X".into()),
                content: None,
                folder: None,
                tags: None,
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Note not found");
    }

    #[tokio::test]
    async fn update_note_partial() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "p", "---\ntitle: Title\nfolder: f\ntags: t\n---\nBody\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = update_note(
            State(state),
            Path("p".into()),
            Json(UpdateNoteRequest {
                title: Some("Updated".into()),
                content: None,
                folder: None,
                tags: None,
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["title"], "Updated");
        assert_eq!(json["folder"], "f");
        assert_eq!(json["tags"], "t");
        assert_eq!(json["content"], "Body");
    }

    // ── Handler: delete_note ─────────────────────────────────────────────

    #[tokio::test]
    async fn delete_note_existing() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "del", "---\ntitle: Delete\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = delete_note(State(state), Path("del".into())).await;
        let status = resp.into_response().status();
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(!notes_dir.join("del.md").exists());
    }

    #[tokio::test]
    async fn delete_note_not_found() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap();
        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = delete_note(State(state), Path("nope".into())).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Note not found");
    }

    // ── Handler: search_notes ────────────────────────────────────────────

    #[tokio::test]
    async fn search_notes_by_title() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "a", "---\ntitle: Rust Basics\n---\n");
        write_raw_note(&notes_dir, "b", "---\ntitle: Cooking 101\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let mut params = HashMap::new();
        params.insert("q".into(), "rust".into());
        let resp = search_notes(State(state), Query(params)).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["query"], "rust");
    }

    #[tokio::test]
    async fn search_notes_by_content() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "a", "---\ntitle: A\n---\nContains axum\n");
        write_raw_note(&notes_dir, "b", "---\ntitle: B\n---\nNothing here\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let mut params = HashMap::new();
        params.insert("q".into(), "axum".into());
        let resp = search_notes(State(state), Query(params)).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["notes"][0]["title"], "A");
    }

    #[tokio::test]
    async fn search_notes_empty_query_lists_all() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "x", "---\ntitle: X\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let params = HashMap::new();
        let resp = search_notes(State(state), Query(params)).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
    }

    #[tokio::test]
    async fn search_notes_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "c", "---\ntitle: Rust Guide\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let mut params = HashMap::new();
        params.insert("q".into(), "RUST".into());
        let resp = search_notes(State(state), Query(params)).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
    }

    #[tokio::test]
    async fn search_notes_no_results() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "n", "---\ntitle: Note\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let mut params = HashMap::new();
        params.insert("q".into(), "nonexistent".into());
        let resp = search_notes(State(state), Query(params)).await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 0);
    }

    // ── Handler: sort order ──────────────────────────────────────────────

    #[tokio::test]
    async fn list_notes_sort_by_title() {
        let tmp = TempDir::new().unwrap();
        let notes_dir = tmp.path().join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        write_raw_note(&notes_dir, "z", "---\ntitle: Zebra\n---\n");
        write_raw_note(&notes_dir, "a", "---\ntitle: Apple\n---\n");

        let state = MockState::new(tmp.path().to_str().unwrap());
        let resp = list_notes(
            State(state),
            Query(NotesQuery {
                sort: Some("title".into()),
                order: Some("asc".into()),
                ..Default::default()
            }),
        )
        .await;
        let body = response_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["notes"][0]["title"], "Zebra");
        assert_eq!(json["notes"][1]["title"], "Apple");
    }
}
