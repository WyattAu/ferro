use std::sync::Arc;

use dashmap::DashMap;

use super::activity::Activity;

pub struct ActivityStore {
    inbox: Arc<DashMap<String, Activity>>,
    outbox: Arc<DashMap<String, Activity>>,
    followers: Arc<DashMap<String, Vec<String>>>,
    following: Arc<DashMap<String, Vec<String>>>,
    max_entries: usize,
}

impl ActivityStore {
    pub fn new() -> Self {
        Self {
            inbox: Arc::new(DashMap::new()),
            outbox: Arc::new(DashMap::new()),
            followers: Arc::new(DashMap::new()),
            following: Arc::new(DashMap::new()),
            max_entries: 10_000,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            max_entries,
            ..Self::new()
        }
    }

    pub fn add_to_inbox(&self, activity: Activity) {
        let id = activity.id.clone();
        self.inbox.insert(id, activity);
        // Evict if over capacity
        let len = self.inbox.len();
        if len > self.max_entries {
            let to_remove = len - self.max_entries;
            let mut removed = 0;
            self.inbox.retain(|_, _| {
                removed += 1;
                removed <= to_remove
            });
        }
    }

    pub fn get_inbox(&self, offset: usize, limit: usize) -> Vec<Activity> {
        let mut activities: Vec<_> = self.inbox.iter().map(|e| e.value().clone()).collect();
        activities.sort_by(|a, b| b.published.cmp(&a.published));
        activities.into_iter().skip(offset).take(limit).collect()
    }

    pub fn add_to_outbox(&self, activity: Activity) {
        let id = activity.id.clone();
        self.outbox.insert(id, activity);
        let len = self.outbox.len();
        if len > self.max_entries {
            let to_remove = len - self.max_entries;
            let mut removed = 0;
            self.outbox.retain(|_, _| {
                removed += 1;
                removed <= to_remove
            });
        }
    }

    pub fn get_outbox(&self, offset: usize, limit: usize) -> Vec<Activity> {
        let mut activities: Vec<_> = self.outbox.iter().map(|e| e.value().clone()).collect();
        activities.sort_by(|a, b| b.published.cmp(&a.published));
        activities.into_iter().skip(offset).take(limit).collect()
    }

    pub fn add_follower(&self, actor: &str, follower: &str) {
        self.followers
            .entry(actor.to_string())
            .or_default()
            .push(follower.to_string());
    }

    pub fn remove_follower(&self, actor: &str, follower: &str) {
        if let Some(mut followers) = self.followers.get_mut(actor) {
            followers.retain(|f| f != follower);
        }
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

    pub fn add_following(&self, actor: &str, target: &str) {
        self.following
            .entry(actor.to_string())
            .or_default()
            .push(target.to_string());
    }

    pub fn inbox_len(&self) -> usize {
        self.inbox.len()
    }

    pub fn outbox_len(&self) -> usize {
        self.outbox.len()
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
    use crate::federation::activity::{Activity, ActivityType};

    fn make_activity(id: &str, published: &str) -> Activity {
        Activity {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: id.to_string(),
            r#type: ActivityType::Create,
            actor: "https://example.com/actor/alice".to_string(),
            object: serde_json::json!({"type": "Note"}),
            to: Some(vec!["https://www.w3.org/ns/activitystreams#Public".to_string()]),
            cc: None,
            published: published.to_string(),
            target: None,
        }
    }

    #[test]
    fn test_inbox_outbox_ordering() {
        let store = ActivityStore::new();
        store.add_to_inbox(make_activity("a", "2024-01-01T00:00:00+00:00"));
        store.add_to_inbox(make_activity("b", "2024-01-03T00:00:00+00:00"));
        store.add_to_inbox(make_activity("c", "2024-01-02T00:00:00+00:00"));

        let items = store.get_inbox(0, 10);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "b");
        assert_eq!(items[1].id, "c");
        assert_eq!(items[2].id, "a");
    }

    #[test]
    fn test_outbox_ordering() {
        let store = ActivityStore::new();
        store.add_to_outbox(make_activity("x", "2024-06-01T00:00:00+00:00"));
        store.add_to_outbox(make_activity("y", "2024-06-03T00:00:00+00:00"));
        store.add_to_outbox(make_activity("z", "2024-06-02T00:00:00+00:00"));

        let items = store.get_outbox(0, 10);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "y");
        assert_eq!(items[1].id, "z");
        assert_eq!(items[2].id, "x");
    }

    #[test]
    fn test_followers_following() {
        let store = ActivityStore::new();
        store.add_follower("alice", "bob");
        store.add_follower("alice", "carol");
        store.add_follower("bob", "alice");

        let followers = store.get_followers("alice");
        assert_eq!(followers.len(), 2);
        assert!(followers.contains(&"bob".to_string()));
        assert!(followers.contains(&"carol".to_string()));

        store.add_following("alice", "dave");
        let following = store.get_following("alice");
        assert_eq!(following, vec!["dave".to_string()]);

        assert!(store.get_followers("nonexistent").is_empty());
        assert!(store.get_following("nonexistent").is_empty());
    }

    #[test]
    fn test_remove_follower() {
        let store = ActivityStore::new();
        store.add_follower("alice", "bob");
        store.add_follower("alice", "carol");
        store.remove_follower("alice", "bob");
        let followers = store.get_followers("alice");
        assert_eq!(followers, vec!["carol".to_string()]);
    }

    #[test]
    fn test_store_bounded_max_entries() {
        let store = ActivityStore::with_max_entries(3);
        for i in 0..5 {
            store.add_to_inbox(make_activity(&format!("msg-{}", i), &format!("2024-01-{:02}T00:00:00+00:00", i + 1)));
        }
        assert!(store.inbox_len() <= 3);
    }

    #[test]
    fn test_inbox_pagination() {
        let store = ActivityStore::new();
        for i in 0..10 {
            store.add_to_inbox(make_activity(
                &format!("msg-{:02}", i),
                &format!("2024-01-{:02}T00:00:00+00:00", 10 - i),
            ));
        }
        let page1 = store.get_inbox(0, 3);
        assert_eq!(page1.len(), 3);
        let page2 = store.get_inbox(3, 3);
        assert_eq!(page2.len(), 3);
        assert_ne!(page1[0].id, page2[0].id);
    }
}
