use crate::profile::SyncProfile;
use rusqlite::{Connection, params};
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("profile not found: {0}")]
    NotFound(String),
}

pub struct ProfileStore {
    conn: Mutex<Connection>,
}

unsafe impl Send for ProfileStore {}
unsafe impl Sync for ProfileStore {}

impl ProfileStore {
    pub fn new(conn: Connection) -> Result<Self, StoreError> {
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn new_in_memory() -> Result<Self, StoreError> {
        Self::new(Connection::open_in_memory()?)
    }

    fn ensure_schema(&self) -> Result<(), StoreError> {
        self.conn.lock().unwrap().execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_profiles (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                owner       TEXT NOT NULL,
                rules       TEXT NOT NULL DEFAULT '[]',
                path_prefix TEXT,
                enabled     INTEGER NOT NULL DEFAULT 1,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sync_profiles_owner ON sync_profiles(owner);",
        )?;
        Ok(())
    }

    pub fn list_profiles(&self, owner: &str) -> Result<Vec<SyncProfile>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, owner, rules, path_prefix, enabled, created_at, updated_at
             FROM sync_profiles WHERE owner = ?1 ORDER BY created_at",
        )?;

        let rows = stmt.query_map(params![owner], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let owner: String = row.get(2)?;
            let rules_json: String = row.get(3)?;
            let path_prefix: Option<String> = row.get(4)?;
            let enabled: bool = row.get::<_, i32>(5)? != 0;
            let created_at: String = row.get(6)?;
            let updated_at: String = row.get(7)?;

            Ok(SyncProfile {
                id,
                name,
                owner,
                rules: serde_json::from_str(&rules_json).unwrap_or_default(),
                path_prefix,
                enabled,
                created_at,
                updated_at,
            })
        })?;

        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row?);
        }
        Ok(profiles)
    }

    pub fn get_profile(&self, id: &str) -> Result<SyncProfile, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, owner, rules, path_prefix, enabled, created_at, updated_at
             FROM sync_profiles WHERE id = ?1",
        )?;

        stmt.query_row(params![id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let owner: String = row.get(2)?;
            let rules_json: String = row.get(3)?;
            let path_prefix: Option<String> = row.get(4)?;
            let enabled: bool = row.get::<_, i32>(5)? != 0;
            let created_at: String = row.get(6)?;
            let updated_at: String = row.get(7)?;

            Ok(SyncProfile {
                id,
                name,
                owner,
                rules: serde_json::from_str(&rules_json).unwrap_or_default(),
                path_prefix,
                enabled,
                created_at,
                updated_at,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => StoreError::NotFound(id.to_string()),
            other => StoreError::Sqlite(other),
        })
    }

    pub fn create_profile(&self, profile: &SyncProfile) -> Result<(), StoreError> {
        let rules_json = serde_json::to_string(&profile.rules)?;
        self.conn.lock().unwrap().execute(
            "INSERT INTO sync_profiles (id, name, owner, rules, path_prefix, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                profile.id,
                profile.name,
                profile.owner,
                rules_json,
                profile.path_prefix,
                profile.enabled as i32,
                profile.created_at,
                profile.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_profile(&self, profile: &SyncProfile) -> Result<(), StoreError> {
        let rules_json = serde_json::to_string(&profile.rules)?;
        let rows = self.conn.lock().unwrap().execute(
            "UPDATE sync_profiles SET name = ?1, rules = ?2, path_prefix = ?3, enabled = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                profile.name,
                rules_json,
                profile.path_prefix,
                profile.enabled as i32,
                profile.updated_at,
                profile.id,
            ],
        )?;
        if rows == 0 {
            return Err(StoreError::NotFound(profile.id.clone()));
        }
        Ok(())
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), StoreError> {
        let rows = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM sync_profiles WHERE id = ?1", params![id])?;
        if rows == 0 {
            return Err(StoreError::NotFound(id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{RuleDirection, SyncRule};

    fn make_profile(name: &str, owner: &str) -> SyncProfile {
        SyncProfile::new(
            name.to_string(),
            owner.to_string(),
            vec![SyncRule {
                pattern: "*.txt".to_string(),
                direction: RuleDirection::Include,
            }],
        )
    }

    #[test]
    fn test_create_and_get() {
        let store = ProfileStore::new_in_memory().unwrap();
        let profile = make_profile("docs", "alice");
        store.create_profile(&profile).unwrap();

        let fetched = store.get_profile(&profile.id).unwrap();
        assert_eq!(fetched.name, "docs");
        assert_eq!(fetched.owner, "alice");
        assert_eq!(fetched.rules.len(), 1);
    }

    #[test]
    fn test_list_by_owner() {
        let store = ProfileStore::new_in_memory().unwrap();
        store.create_profile(&make_profile("a", "alice")).unwrap();
        store.create_profile(&make_profile("b", "bob")).unwrap();
        store.create_profile(&make_profile("c", "alice")).unwrap();

        let alice = store.list_profiles("alice").unwrap();
        assert_eq!(alice.len(), 2);
        let bob = store.list_profiles("bob").unwrap();
        assert_eq!(bob.len(), 1);
    }

    #[test]
    fn test_update() {
        let store = ProfileStore::new_in_memory().unwrap();
        let mut profile = make_profile("old", "alice");
        store.create_profile(&profile).unwrap();

        profile.name = "new".to_string();
        store.update_profile(&profile).unwrap();

        let fetched = store.get_profile(&profile.id).unwrap();
        assert_eq!(fetched.name, "new");
    }

    #[test]
    fn test_delete() {
        let store = ProfileStore::new_in_memory().unwrap();
        let profile = make_profile("temp", "alice");
        store.create_profile(&profile).unwrap();
        store.delete_profile(&profile.id).unwrap();
        assert!(store.get_profile(&profile.id).is_err());
    }

    #[test]
    fn test_delete_nonexistent() {
        let store = ProfileStore::new_in_memory().unwrap();
        assert!(store.delete_profile("nope").is_err());
    }
}
