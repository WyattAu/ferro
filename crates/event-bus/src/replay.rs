use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub event_json: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub event_type: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub path_prefix: Option<String>,
    pub user_id: Option<String>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }

    pub fn path_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.path_prefix = Some(prefix.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn matches(&self, event: &StoredEvent) -> bool {
        if let Some(ref et) = self.event_type
            && &event.event_type != et
        {
            return false;
        }
        if let Some(since) = self.since
            && event.timestamp < since
        {
            return false;
        }
        if let Some(until) = self.until
            && event.timestamp > until
        {
            return false;
        }
        if let Some(ref prefix) = self.path_prefix {
            let json = &event.event_json;
            if !json.contains(&format!("\"path\":\"{prefix}")) {
                return false;
            }
        }
        if let Some(ref uid) = self.user_id {
            let json = &event.event_json;
            if !json.contains(&format!("\"user_id\":\"{uid}\"")) {
                return false;
            }
        }
        true
    }
}

pub struct EventStore {
    events: Mutex<Vec<StoredEvent>>,
}

impl EventStore {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn append(&self, mut event: StoredEvent) {
        if event.id.is_empty() {
            event.id = uuid::Uuid::new_v4().to_string();
        }
        self.events.lock().push(event);
    }

    pub fn query(&self, filter: &EventFilter) -> Vec<StoredEvent> {
        self.events
            .lock()
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    pub fn all(&self) -> Vec<StoredEvent> {
        self.events.lock().clone()
    }

    pub fn len(&self) -> usize {
        self.events.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.lock().is_empty()
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stored(event_type: &str, timestamp: DateTime<Utc>, path: &str, user_id: &str) -> StoredEvent {
        let event = serde_json::json!({
            "event_type": event_type,
            "path": path,
            "user_id": user_id,
        });
        StoredEvent {
            event_json: serde_json::to_string(&event).unwrap(),
            event_type: event_type.to_string(),
            timestamp,
            id: uuid::Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn append_and_query_all() {
        let store = EventStore::new();
        store.append(make_stored("file.created", Utc::now(), "/docs/a.txt", "u1"));
        store.append(make_stored("file.deleted", Utc::now(), "/docs/b.txt", "u2"));
        assert_eq!(store.len(), 2);
        let results = store.query(&EventFilter::new());
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn filter_by_event_type() {
        let store = EventStore::new();
        store.append(make_stored("file.created", Utc::now(), "/a", "u1"));
        store.append(make_stored("file.deleted", Utc::now(), "/b", "u2"));
        let results = store.query(&EventFilter::new().event_type("file.created"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type, "file.created");
    }

    #[test]
    fn filter_by_time_range() {
        let store = EventStore::new();
        let t1 = Utc::now() - chrono::Duration::days(2);
        let t2 = Utc::now();
        let t3 = Utc::now() + chrono::Duration::days(2);
        store.append(make_stored("test", t1, "/a", "u1"));
        store.append(make_stored("test", t2, "/b", "u2"));
        let results = store.query(&EventFilter::new().since(t1).until(t3));
        assert_eq!(results.len(), 2);
        let results = store.query(&EventFilter::new().since(Utc::now() - chrono::Duration::hours(1)));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_by_path_prefix() {
        let store = EventStore::new();
        store.append(make_stored("file.created", Utc::now(), "/docs/a.txt", "u1"));
        store.append(make_stored("file.created", Utc::now(), "/photos/b.jpg", "u2"));
        let results = store.query(&EventFilter::new().path_prefix("/docs"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn filter_by_user_id() {
        let store = EventStore::new();
        store.append(make_stored("file.created", Utc::now(), "/a", "alice"));
        store.append(make_stored("file.created", Utc::now(), "/b", "bob"));
        let results = store.query(&EventFilter::new().user_id("alice"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn empty_store_query() {
        let store = EventStore::new();
        assert!(store.is_empty());
        let results = store.query(&EventFilter::new());
        assert!(results.is_empty());
    }

    #[test]
    fn combined_filters() {
        let store = EventStore::new();
        store.append(make_stored("file.created", Utc::now(), "/docs/a.txt", "alice"));
        store.append(make_stored("file.deleted", Utc::now(), "/docs/b.txt", "alice"));
        let results = store.query(&EventFilter::new().event_type("file.created").user_id("alice"));
        assert_eq!(results.len(), 1);
    }
}
