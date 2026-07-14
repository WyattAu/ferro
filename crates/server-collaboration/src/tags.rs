use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::warn;

use crate::DbHandle;
use crate::{ApiError, CollaborationState};

const MAX_TAGS_PER_FILE: usize = 50;
const MAX_TAGGED_FILES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTags {
    pub path: String,
    pub tags: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct TagStore {
    pub entries: Arc<DashMap<String, HashSet<String>>>,
    db: Option<DbHandle>,
}

impl TagStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn add_tag(&self, path: &str, tag: &str) -> Result<(), String> {
        if tag.is_empty() {
            return Err("Tag cannot be empty".to_string());
        }
        if tag.len() > 100 {
            return Err("Tag exceeds 100 character limit".to_string());
        }
        if !self.entries.contains_key(path) && self.entries.len() >= MAX_TAGGED_FILES {
            return Err(format!("Maximum tagged files limit ({}) reached", MAX_TAGGED_FILES));
        }
        let mut entry = self.entries.entry(path.to_string()).or_default();
        if entry.value().len() >= MAX_TAGS_PER_FILE {
            return Err(format!(
                "File already has {} tags (max {})",
                entry.value().len(),
                MAX_TAGS_PER_FILE
            ));
        }
        entry.value_mut().insert(tag.to_string());
        if let Some(ref db) = self.db
            && let Err(e) = db.lock().unwrap_or_else(|e| e.into_inner()).execute(
                "INSERT OR IGNORE INTO file_tags (file_path, tag) VALUES (?1, ?2)",
                params![path, tag],
            )
        {
            warn!("Failed to persist tag to SQLite: {}", e);
        }
        Ok(())
    }

    pub fn remove_tag(&self, path: &str, tag: &str) -> bool {
        if let Some(mut entry) = self.entries.get_mut(path) {
            let removed = entry.value_mut().remove(tag);
            if entry.value().is_empty() {
                drop(entry);
                self.entries.remove(path);
            }
            if removed
                && let Some(ref db) = self.db
                && let Err(e) = db.lock().unwrap_or_else(|e| e.into_inner()).execute(
                    "DELETE FROM file_tags WHERE file_path = ?1 AND tag = ?2",
                    params![path, tag],
                )
            {
                warn!("Failed to remove tag from SQLite: {}", e);
            }
            removed
        } else {
            false
        }
    }

    pub fn get_tags(&self, path: &str) -> HashSet<String> {
        self.entries.get(path).map(|e| e.value().clone()).unwrap_or_default()
    }

    pub fn list_all_tags(&self) -> Vec<(String, usize)> {
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for entry in self.entries.iter() {
            for tag in entry.value() {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut result: Vec<_> = tag_counts.into_iter().collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.value().contains(tag))
            .map(|e| e.key().clone())
            .collect()
    }

    pub fn remove_file(&self, path: &str) {
        self.entries.remove(path);
        if let Some(ref db) = self.db
            && let Err(e) = db
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .execute("DELETE FROM file_tags WHERE file_path = ?1", params![path])
        {
            warn!("Failed to remove file tags from SQLite: {}", e);
        }
    }

    pub fn load_all_from_db(&self, conn: &rusqlite::Connection) -> std::result::Result<(), rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT file_path, tag FROM file_tags")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        for row in rows {
            let (path, tag): (String, String) = row?;
            self.entries.entry(path).or_default().value_mut().insert(tag);
        }
        Ok(())
    }
}

impl Default for TagStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
pub struct AddTagsRequest {
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchTagQuery {
    pub tag: String,
}

pub async fn list_tags<S: CollaborationState>(State(state): State<S>) -> Response {
    let all_tags = state.tags().list_all_tags();
    let tags_json: Vec<serde_json::Value> = all_tags
        .into_iter()
        .map(|(tag, count)| serde_json::json!({ "tag": tag, "count": count }))
        .collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "tags": tags_json }))).into_response()
}

pub async fn get_tags<S: CollaborationState>(State(state): State<S>, Path(path): Path<String>) -> Response {
    let tags = state.tags().get_tags(&path);
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "path": path, "tags": tags })),
    )
        .into_response()
}

pub async fn add_tags<S: CollaborationState>(
    State(state): State<S>,
    Path(path): Path<String>,
    axum::Json(body): axum::Json<AddTagsRequest>,
) -> Response {
    let mut errors: Vec<String> = Vec::new();
    let mut added: Vec<String> = Vec::new();

    for tag in &body.tags {
        match state.tags().add_tag(&path, tag) {
            Ok(()) => added.push(tag.clone()),
            Err(e) => errors.push(format!("{}: {}", tag, e)),
        }
    }

    if added.is_empty() && !errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "added": added,
                "errors": errors,
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "added": added,
            "errors": errors,
        })),
    )
        .into_response()
}

pub async fn remove_tag<S: CollaborationState>(
    State(state): State<S>,
    axum::extract::Path((path, tag)): axum::extract::Path<(String, String)>,
) -> Response {
    let removed = state.tags().remove_tag(&path, &tag);
    if removed {
        (StatusCode::OK, axum::Json(serde_json::json!({ "status": "ok" }))).into_response()
    } else {
        ApiError::not_found(ApiError::NOT_FOUND, "Tag not found on file")
    }
}

pub async fn search_by_tag<S: CollaborationState>(
    State(state): State<S>,
    Query(params): Query<SearchTagQuery>,
) -> Response {
    let files = state.tags().find_by_tag(&params.tag);
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "tag": params.tag, "files": files })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_tag() {
        let store = TagStore::new();
        assert!(store.add_tag("/file.txt", "important").is_ok());
        let tags = store.get_tags("/file.txt");
        assert!(tags.contains("important"));
    }

    #[test]
    fn test_add_duplicate_tag() {
        let store = TagStore::new();
        assert!(store.add_tag("/file.txt", "work").is_ok());
        assert!(store.add_tag("/file.txt", "work").is_ok());
        let tags = store.get_tags("/file.txt");
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_add_empty_tag_rejected() {
        let store = TagStore::new();
        assert!(store.add_tag("/file.txt", "").is_err());
    }

    #[test]
    fn test_tag_limit_per_file() {
        let store = TagStore::new();
        for i in 0..MAX_TAGS_PER_FILE {
            assert!(store.add_tag("/file.txt", &format!("tag-{}", i)).is_ok());
        }
        assert!(store.add_tag("/file.txt", "overflow").is_err());
    }

    #[test]
    fn test_remove_tag() {
        let store = TagStore::new();
        store.add_tag("/file.txt", "keep").unwrap();
        store.add_tag("/file.txt", "remove-me").unwrap();
        assert!(store.remove_tag("/file.txt", "remove-me"));
        let tags = store.get_tags("/file.txt");
        assert!(!tags.contains("remove-me"));
        assert!(tags.contains("keep"));
    }

    #[test]
    fn test_find_by_tag() {
        let store = TagStore::new();
        store.add_tag("/a.txt", "shared").unwrap();
        store.add_tag("/b.txt", "shared").unwrap();
        store.add_tag("/c.txt", "private").unwrap();

        let files = store.find_by_tag("shared");
        assert_eq!(files.len(), 2);

        let files = store.find_by_tag("private");
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_list_all_tags() {
        let store = TagStore::new();
        store.add_tag("/a.txt", "work").unwrap();
        store.add_tag("/b.txt", "work").unwrap();
        store.add_tag("/a.txt", "personal").unwrap();

        let all = store.list_all_tags();
        assert_eq!(all.len(), 2);
        let work_entry = all.iter().find(|(t, _)| t == "work").unwrap();
        assert_eq!(work_entry.1, 2);
    }

    #[test]
    fn test_remove_file() {
        let store = TagStore::new();
        store.add_tag("/file.txt", "tag1").unwrap();
        store.add_tag("/file.txt", "tag2").unwrap();
        store.remove_file("/file.txt");
        assert!(store.get_tags("/file.txt").is_empty());
    }
}
