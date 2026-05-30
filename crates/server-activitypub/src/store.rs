use std::sync::Arc;

use dashmap::DashMap;
use rusqlite::params;
use tracing::warn;

use crate::activity::Activity;

/// # Safety
/// The wrapped `rusqlite::Connection` is only accessed via short-lived lock guards
/// that never cross an `.await` point. SQLite operations are synchronous
/// and complete in microseconds, well below the threshold for async poisoning.
pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

#[derive(Debug)]
pub enum StoreError {
    LockPoisoned,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockPoisoned => write!(f, "mutex lock poisoned, possible data corruption"),
        }
    }
}

impl std::error::Error for StoreError {}

fn acquire_lock<'a>(
    db: &'a DbHandle,
    context: &str,
) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, StoreError> {
    db.lock().map_err(|_e| {
        warn!("{context}: mutex poisoned, refusing to recover lock");
        StoreError::LockPoisoned
    })
}

pub struct ActivityStore {
    pub(crate) inbox: Arc<DashMap<String, Activity>>,
    pub(crate) outbox: Arc<DashMap<String, Activity>>,
    pub(crate) followers: Arc<DashMap<String, Vec<String>>>,
    pub(crate) following: Arc<DashMap<String, Vec<String>>>,
    max_entries: usize,
    db: Option<DbHandle>,
}

impl ActivityStore {
    pub fn new() -> Self {
        Self {
            inbox: Arc::new(DashMap::new()),
            outbox: Arc::new(DashMap::new()),
            followers: Arc::new(DashMap::new()),
            following: Arc::new(DashMap::new()),
            max_entries: 10_000,
            db: None,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            max_entries,
            ..Self::new()
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn add_to_inbox(&self, activity: Activity) -> Result<(), StoreError> {
        let id = activity.id.clone();
        self.inbox.insert(id.clone(), activity.clone());
        if let Some(ref db) = self.db {
            let raw_json = serde_json::to_string(&activity).unwrap_or_else(|e| {
                warn!("inbox: failed to serialize activity: {e}");
                String::new()
            });
            let obj_json = serde_json::to_string(&activity.object).unwrap_or_else(|e| {
                warn!("inbox: failed to serialize activity object: {e}");
                String::new()
            });
            let target_json = activity.target.as_ref().map(|t| {
                serde_json::to_string(t).unwrap_or_else(|e| {
                    warn!("inbox: failed to serialize activity target: {e}");
                    String::new()
                })
            });
            let conn = acquire_lock(db, "inbox")?;
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO fed_activities (activity_id, actor, type, object, target, published, raw_json, box_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'inbox')",
                params![
                    id,
                    activity.actor,
                    format!("{:?}", activity.r#type),
                    obj_json,
                    target_json,
                    activity.published,
                    raw_json,
                ],
            ) {
                warn!("inbox: failed to persist activity to SQLite: {e}");
            }
        }
        let len = self.inbox.len();
        if len > self.max_entries {
            let to_remove = len - self.max_entries;
            let mut removed = 0;
            self.inbox.retain(|_, _| {
                removed += 1;
                removed <= to_remove
            });
        }
        Ok(())
    }

    pub fn get_inbox(&self, offset: usize, limit: usize) -> Vec<Activity> {
        let mut activities: Vec<_> = self.inbox.iter().map(|e| e.value().clone()).collect();
        activities.sort_by(|a, b| b.published.cmp(&a.published));
        activities.into_iter().skip(offset).take(limit).collect()
    }

    pub fn add_to_outbox(&self, activity: Activity) -> Result<(), StoreError> {
        let id = activity.id.clone();
        self.outbox.insert(id.clone(), activity.clone());
        if let Some(ref db) = self.db {
            let raw_json = serde_json::to_string(&activity).unwrap_or_else(|e| {
                warn!("outbox: failed to serialize activity: {e}");
                String::new()
            });
            let obj_json = serde_json::to_string(&activity.object).unwrap_or_else(|e| {
                warn!("outbox: failed to serialize activity object: {e}");
                String::new()
            });
            let target_json = activity.target.as_ref().map(|t| {
                serde_json::to_string(t).unwrap_or_else(|e| {
                    warn!("outbox: failed to serialize activity target: {e}");
                    String::new()
                })
            });
            let conn = acquire_lock(db, "outbox")?;
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO fed_activities (activity_id, actor, type, object, target, published, raw_json, box_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'outbox')",
                params![
                    id,
                    activity.actor,
                    format!("{:?}", activity.r#type),
                    obj_json,
                    target_json,
                    activity.published,
                    raw_json,
                ],
            ) {
                warn!("outbox: failed to persist activity to SQLite: {e}");
            }
        }
        let len = self.outbox.len();
        if len > self.max_entries {
            let to_remove = len - self.max_entries;
            let mut removed = 0;
            self.outbox.retain(|_, _| {
                removed += 1;
                removed <= to_remove
            });
        }
        Ok(())
    }

    pub fn get_outbox(&self, offset: usize, limit: usize) -> Vec<Activity> {
        let mut activities: Vec<_> = self.outbox.iter().map(|e| e.value().clone()).collect();
        activities.sort_by(|a, b| b.published.cmp(&a.published));
        activities.into_iter().skip(offset).take(limit).collect()
    }

    pub fn add_follower(&self, actor: &str, follower: &str) -> Result<(), StoreError> {
        self.followers
            .entry(actor.to_string())
            .or_default()
            .push(follower.to_string());
        if let Some(ref db) = self.db {
            let conn = acquire_lock(db, "add_follower")?;
            if let Err(e) = conn.execute(
                "INSERT OR IGNORE INTO fed_followers (actor, follower) VALUES (?1, ?2)",
                params![actor, follower],
            ) {
                warn!("Failed to persist follower to SQLite: {}", e);
            }
        }
        Ok(())
    }

    pub fn remove_follower(&self, actor: &str, follower: &str) -> Result<(), StoreError> {
        if let Some(mut followers) = self.followers.get_mut(actor) {
            followers.retain(|f| f != follower);
        }
        if let Some(ref db) = self.db {
            let conn = acquire_lock(db, "remove_follower")?;
            if let Err(e) = conn.execute(
                "DELETE FROM fed_followers WHERE actor = ?1 AND follower = ?2",
                params![actor, follower],
            ) {
                warn!("Failed to remove follower from SQLite: {}", e);
            }
        }
        Ok(())
    }

    pub fn get_followers(&self, actor: &str) -> Vec<String> {
        self.followers
            .get(actor)
            .map(|f| f.value().clone())
            .unwrap_or_default()
    }

    pub fn get_following(&self, actor: &str) -> Vec<String> {
        self.following
            .get(actor)
            .map(|f| f.value().clone())
            .unwrap_or_default()
    }

    pub fn add_following(&self, actor: &str, target: &str) -> Result<(), StoreError> {
        self.following
            .entry(actor.to_string())
            .or_default()
            .push(target.to_string());
        if let Some(ref db) = self.db {
            let conn = acquire_lock(db, "add_following")?;
            if let Err(e) = conn.execute(
                "INSERT OR IGNORE INTO fed_following (actor, target) VALUES (?1, ?2)",
                params![actor, target],
            ) {
                warn!("Failed to persist following to SQLite: {}", e);
            }
        }
        Ok(())
    }

    pub fn inbox_len(&self) -> usize {
        self.inbox.len()
    }

    pub fn outbox_len(&self) -> usize {
        self.outbox.len()
    }

    pub fn load_all_from_db(
        &self,
        conn: &rusqlite::Connection,
    ) -> std::result::Result<(), rusqlite::Error> {
        let mut stmt =
            conn.prepare("SELECT activity_id, raw_json, box_type FROM fed_activities")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (id, raw_json, box_type): (String, String, String) = row?;
            if let Ok(activity) = serde_json::from_str::<Activity>(&raw_json) {
                match box_type.as_str() {
                    "inbox" => {
                        self.inbox.insert(id, activity);
                    }
                    "outbox" => {
                        self.outbox.insert(id, activity);
                    }
                    _ => {}
                }
            }
        }

        let mut stmt = conn.prepare("SELECT actor, follower FROM fed_followers")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (actor, follower): (String, String) = row?;
            self.followers.entry(actor).or_default().push(follower);
        }

        let mut stmt = conn.prepare("SELECT actor, target FROM fed_following")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (actor, target): (String, String) = row?;
            self.following.entry(actor).or_default().push(target);
        }

        Ok(())
    }
}

impl Default for ActivityStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{Activity, ActivityType};

    fn make_activity(id: &str, published: &str) -> Activity {
        Activity {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: id.to_string(),
            r#type: ActivityType::Create,
            actor: "https://example.com/actor/alice".to_string(),
            object: serde_json::json!({"type": "Note"}),
            to: Some(vec![
                "https://www.w3.org/ns/activitystreams#Public".to_string(),
            ]),
            cc: None,
            published: published.to_string(),
            target: None,
        }
    }

    #[test]
    fn test_inbox_outbox_ordering() {
        let store = ActivityStore::new();
        store
            .add_to_inbox(make_activity("a", "2024-01-01T00:00:00+00:00"))
            .unwrap();
        store
            .add_to_inbox(make_activity("b", "2024-01-03T00:00:00+00:00"))
            .unwrap();
        store
            .add_to_inbox(make_activity("c", "2024-01-02T00:00:00+00:00"))
            .unwrap();

        let items = store.get_inbox(0, 10);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "b");
        assert_eq!(items[1].id, "c");
        assert_eq!(items[2].id, "a");
    }

    #[test]
    fn test_outbox_ordering() {
        let store = ActivityStore::new();
        store
            .add_to_outbox(make_activity("x", "2024-06-01T00:00:00+00:00"))
            .unwrap();
        store
            .add_to_outbox(make_activity("y", "2024-06-03T00:00:00+00:00"))
            .unwrap();
        store
            .add_to_outbox(make_activity("z", "2024-06-02T00:00:00+00:00"))
            .unwrap();

        let items = store.get_outbox(0, 10);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "y");
        assert_eq!(items[1].id, "z");
        assert_eq!(items[2].id, "x");
    }

    #[test]
    fn test_followers_following() {
        let store = ActivityStore::new();
        store.add_follower("alice", "bob").unwrap();
        store.add_follower("alice", "carol").unwrap();
        store.add_follower("bob", "alice").unwrap();

        let followers = store.get_followers("alice");
        assert_eq!(followers.len(), 2);
        assert!(followers.contains(&"bob".to_string()));
        assert!(followers.contains(&"carol".to_string()));

        store.add_following("alice", "dave").unwrap();
        let following = store.get_following("alice");
        assert_eq!(following, vec!["dave".to_string()]);

        assert!(store.get_followers("nonexistent").is_empty());
        assert!(store.get_following("nonexistent").is_empty());
    }

    #[test]
    fn test_remove_follower() {
        let store = ActivityStore::new();
        store.add_follower("alice", "bob").unwrap();
        store.add_follower("alice", "carol").unwrap();
        store.remove_follower("alice", "bob").unwrap();
        let followers = store.get_followers("alice");
        assert_eq!(followers, vec!["carol".to_string()]);
    }

    #[test]
    fn test_store_bounded_max_entries() {
        let store = ActivityStore::with_max_entries(3);
        for i in 0..5 {
            store
                .add_to_inbox(make_activity(
                    &format!("msg-{}", i),
                    &format!("2024-01-{:02}T00:00:00+00:00", i + 1),
                ))
                .unwrap();
        }
        assert!(store.inbox_len() <= 3);
    }

    #[test]
    fn test_inbox_pagination() {
        let store = ActivityStore::new();
        for i in 0..10 {
            store
                .add_to_inbox(make_activity(
                    &format!("msg-{:02}", i),
                    &format!("2024-01-{:02}T00:00:00+00:00", 10 - i),
                ))
                .unwrap();
        }
        let page1 = store.get_inbox(0, 3);
        assert_eq!(page1.len(), 3);
        let page2 = store.get_inbox(3, 3);
        assert_eq!(page2.len(), 3);
        assert_ne!(page1[0].id, page2[0].id);
    }
}
