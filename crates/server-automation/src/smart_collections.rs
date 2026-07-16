use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DbHandle;

#[allow(dead_code)]
const MAX_SMART_COLLECTIONS: usize = 50;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectionRule {
    #[serde(rename = "file_type")]
    FileType { mime_pattern: String },
    #[serde(rename = "tag")]
    Tag { tag: String },
    #[serde(rename = "date_range")]
    DateRange {
        after: Option<String>,
        before: Option<String>,
    },
    #[serde(rename = "size_range")]
    SizeRange {
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    },
    #[serde(rename = "path_pattern")]
    PathPattern { pattern: String },
    #[serde(rename = "last_modified")]
    LastModified { days: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCollection {
    pub id: String,
    pub name: String,
    pub rules: Vec<CollectionRule>,
    pub auto_update: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSmartCollectionRequest {
    pub name: String,
    pub rules: Vec<CollectionRule>,
    #[serde(default = "default_true")]
    pub auto_update: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSmartCollectionRequest {
    pub name: Option<String>,
    pub rules: Option<Vec<CollectionRule>>,
    pub auto_update: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SmartCollectionFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub modified_at: String,
    pub is_collection: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// SmartCollectionStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SmartCollectionStore {
    db: Option<DbHandle>,
}

impl Default for SmartCollectionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartCollectionStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn init_tables(&self) -> Result<(), String> {
        let Some(db) = &self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS smart_collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                rules_data TEXT NOT NULL,
                auto_update INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to init smart_collections table: {}", e))
    }

    pub fn list(&self) -> Result<Vec<SmartCollection>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, name, rules_data, auto_update, created_at, updated_at FROM smart_collections ORDER BY name",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let rules_str: String = row.get(2)?;
                let rules: Vec<CollectionRule> =
                    serde_json::from_str(&rules_str).unwrap_or_default();
                let auto_update: i32 = row.get(3)?;
                Ok(SmartCollection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    rules,
                    auto_update: auto_update != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query collections: {}", e))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get(&self, id: &str) -> Result<Option<SmartCollection>, String> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, name, rules_data, auto_update, created_at, updated_at FROM smart_collections WHERE id = ?1",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                let rules_str: String = row.get(2)?;
                let rules: Vec<CollectionRule> =
                    serde_json::from_str(&rules_str).unwrap_or_default();
                let auto_update: i32 = row.get(3)?;
                Ok(SmartCollection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    rules,
                    auto_update: auto_update != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| format!("Failed to query collection: {}", e))?;
        rows.next()
            .transpose()
            .map_err(|e| format!("Failed to read collection: {}", e))
    }

    pub fn create(&self, req: &CreateSmartCollectionRequest) -> Result<SmartCollection, String> {
        if req.name.trim().is_empty() {
            return Err("Collection name must not be empty".to_string());
        }
        if req.rules.is_empty() {
            return Err("At least one rule is required".to_string());
        }
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let rules_str =
            serde_json::to_string(&req.rules).map_err(|e| format!("Invalid rules: {}", e))?;

        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO smart_collections (id, name, rules_data, auto_update, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                req.name,
                rules_str,
                req.auto_update as i32,
                now,
                now,
            ],
        )
        .map_err(|e| format!("Failed to create collection: {}", e))?;

        Ok(SmartCollection {
            id,
            name: req.name.clone(),
            rules: req.rules.clone(),
            auto_update: req.auto_update,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn update(
        &self,
        id: &str,
        req: &UpdateSmartCollectionRequest,
    ) -> Result<Option<SmartCollection>, String> {
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM smart_collections WHERE id = ?1",
                params![id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !exists {
            return Ok(None);
        }

        let now = chrono::Utc::now().to_rfc3339();

        if let Some(name) = &req.name {
            let _ = conn.execute(
                "UPDATE smart_collections SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, id],
            );
        }
        if let Some(rules) = &req.rules
            && let Ok(rules_str) = serde_json::to_string(rules)
        {
            let _ = conn.execute(
                "UPDATE smart_collections SET rules_data = ?1, updated_at = ?2 WHERE id = ?3",
                params![rules_str, now, id],
            );
        }
        if let Some(auto_update) = req.auto_update {
            let _ = conn.execute(
                "UPDATE smart_collections SET auto_update = ?1, updated_at = ?2 WHERE id = ?3",
                params![auto_update as i32, now, id],
            );
        }

        self.get(id)
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let Some(db) = &self.db else {
            return Err("Database not available".to_string());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn
            .execute("DELETE FROM smart_collections WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete collection: {}", e))?;
        Ok(affected > 0)
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/smart-collections — Create a smart collection.
pub async fn create_smart_collection(
    Json(req): Json<CreateSmartCollectionRequest>,
) -> Response {
    let store = smart_collection_store();
    match store.create(&req) {
        Ok(collection) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "message": "Smart collection created",
                "collection": collection,
            })),
        )
            .into_response(),
        Err(e) => {
            if e.contains("must not be empty") || e.contains("At least one") {
                (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e })),
                )
                    .into_response()
            }
        }
    }
}

/// GET /api/v1/smart-collections — List all smart collections.
pub async fn list_smart_collections() -> Response {
    let collections = smart_collection_store().list().unwrap_or_default();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "collections": collections,
            "total": collections.len(),
        })),
    )
        .into_response()
}

/// GET /api/v1/smart-collections/:id — Get a smart collection.
pub async fn get_smart_collection(Path(id): Path<String>) -> Response {
    match smart_collection_store().get(&id) {
        Ok(Some(collection)) => axum::Json(collection).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Smart collection not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// PUT /api/v1/smart-collections/:id — Update a smart collection.
pub async fn update_smart_collection(
    Path(id): Path<String>,
    Json(req): Json<UpdateSmartCollectionRequest>,
) -> Response {
    match smart_collection_store().update(&id, &req) {
        Ok(Some(collection)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Smart collection updated",
                "collection": collection,
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Smart collection not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/smart-collections/:id — Delete a smart collection.
pub async fn delete_smart_collection(Path(id): Path<String>) -> Response {
    match smart_collection_store().delete(&id) {
        Ok(true) => (
            StatusCode::OK,
            Json(serde_json::json!({ "message": "Smart collection deleted" })),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Smart collection not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

/// GET /api/v1/smart-collections/:id/files — Get files matching a smart collection's rules.
pub async fn get_smart_collection_files(Path(id): Path<String>) -> Response {
    match smart_collection_store().get(&id) {
        Ok(Some(_collection)) => {
            // In a full implementation, this would evaluate the rules against
            // the storage engine and return matching files. For now, return empty.
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "files": Vec::<SmartCollectionFile>::new(),
                    "total": 0,
                })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Smart collection not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

fn smart_collection_store() -> &'static SmartCollectionStore {
    use std::sync::OnceLock;
    static STORE: OnceLock<SmartCollectionStore> = OnceLock::new();
    STORE.get_or_init(SmartCollectionStore::new)
}

pub fn init_smart_collection_store(db: DbHandle) {
    use std::sync::OnceLock;
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let store = SmartCollectionStore::new().with_db(db);
        let _ = store.init_tables();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn init_db() -> DbHandle {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));
        let store = SmartCollectionStore::new().with_db(db.clone());
        store.init_tables().unwrap();
        db
    }

    fn make_request(name: &str, rules: Vec<CollectionRule>) -> CreateSmartCollectionRequest {
        CreateSmartCollectionRequest {
            name: name.to_string(),
            rules,
            auto_update: true,
        }
    }

    #[test]
    fn test_smart_collection_store_new_no_db() {
        let store = SmartCollectionStore::new();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn test_smart_collection_store_create_and_list() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        let req = make_request(
            "Images",
            vec![CollectionRule::FileType {
                mime_pattern: "image/*".to_string(),
            }],
        );
        let collection = store.create(&req).unwrap();
        assert_eq!(collection.name, "Images");
        assert!(collection.auto_update);
        assert!(!collection.id.is_empty());

        let list = store.list().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_smart_collection_store_create_empty_name() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        let req = make_request("", vec![CollectionRule::FileType { mime_pattern: "*".to_string() }]);
        assert!(store.create(&req).is_err());
    }

    #[test]
    fn test_smart_collection_store_create_no_rules() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        let req = CreateSmartCollectionRequest {
            name: "Empty".to_string(),
            rules: vec![],
            auto_update: true,
        };
        assert!(store.create(&req).is_err());
    }

    #[test]
    fn test_smart_collection_store_get() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        let req = make_request(
            "Videos",
            vec![CollectionRule::FileType {
                mime_pattern: "video/*".to_string(),
            }],
        );
        let created = store.create(&req).unwrap();
        let found = store.get(&created.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Videos");
    }

    #[test]
    fn test_smart_collection_store_get_not_found() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        assert!(store.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_smart_collection_store_update() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        let req = make_request(
            "Original",
            vec![CollectionRule::FileType {
                mime_pattern: "*".to_string(),
            }],
        );
        let created = store.create(&req).unwrap();

        let update = UpdateSmartCollectionRequest {
            name: Some("Updated".to_string()),
            rules: None,
            auto_update: Some(false),
        };
        let updated = store.update(&created.id, &update).unwrap();
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name, "Updated");
        assert!(!updated.auto_update);
    }

    #[test]
    fn test_smart_collection_store_delete() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        let req = make_request(
            "To Delete",
            vec![CollectionRule::FileType {
                mime_pattern: "*".to_string(),
            }],
        );
        let created = store.create(&req).unwrap();
        assert!(store.delete(&created.id).unwrap());
        assert!(store.get(&created.id).unwrap().is_none());
    }

    #[test]
    fn test_smart_collection_store_delete_not_found() {
        let db = init_db();
        let store = SmartCollectionStore::new().with_db(db);
        assert!(!store.delete("nonexistent").unwrap());
    }

    #[test]
    fn test_collection_rule_serde() {
        let rules = vec![
            CollectionRule::FileType {
                mime_pattern: "image/*".to_string(),
            },
            CollectionRule::Tag {
                tag: "important".to_string(),
            },
            CollectionRule::DateRange {
                after: Some("2024-01-01".to_string()),
                before: None,
            },
        ];
        let json = serde_json::to_string(&rules).unwrap();
        let parsed: Vec<CollectionRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 3);
    }

    #[test]
    fn test_smart_collection_serde() {
        let collection = SmartCollection {
            id: "test-id".to_string(),
            name: "Test".to_string(),
            rules: vec![CollectionRule::FileType {
                mime_pattern: "text/*".to_string(),
            }],
            auto_update: true,
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&collection).unwrap();
        let parsed: SmartCollection = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test");
    }
}
