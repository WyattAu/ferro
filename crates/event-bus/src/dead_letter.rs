use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    pub event_json: String,
    pub event_type: String,
    pub handler_name: String,
    pub error: String,
    pub timestamp: DateTime<Utc>,
    pub retry_count: u32,
}

pub struct DeadLetterQueue {
    entries: Mutex<Vec<DeadLetterEntry>>,
    max_size: usize,
}

impl DeadLetterQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(1000)
    }

    pub fn push(&self, entry: DeadLetterEntry) {
        let mut entries = self.entries.lock();
        if entries.len() >= self.max_size {
            entries.remove(0);
        }
        entries.push(entry);
    }

    pub fn drain(&self, limit: usize) -> Vec<DeadLetterEntry> {
        let mut entries = self.entries.lock();
        let drain_count = limit.min(entries.len());
        entries.drain(..drain_count).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.lock().is_empty()
    }

    pub fn all(&self) -> Vec<DeadLetterEntry> {
        self.entries.lock().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_drain() {
        let dlq = DeadLetterQueue::with_defaults();
        dlq.push(DeadLetterEntry {
            event_json: "{}".to_string(),
            event_type: "test".to_string(),
            handler_name: "h1".to_string(),
            error: "fail".to_string(),
            timestamp: Utc::now(),
            retry_count: 0,
        });
        assert_eq!(dlq.len(), 1);
        let drained = dlq.drain(10);
        assert_eq!(drained.len(), 1);
        assert!(dlq.is_empty());
    }

    #[test]
    fn max_size_evicts_oldest() {
        let dlq = DeadLetterQueue::new(3);
        for i in 0..5 {
            dlq.push(DeadLetterEntry {
                event_json: format!("{{\"i\":{i}}}"),
                event_type: "test".to_string(),
                handler_name: "h1".to_string(),
                error: "fail".to_string(),
                timestamp: Utc::now(),
                retry_count: 0,
            });
        }
        assert_eq!(dlq.len(), 3);
        let entries = dlq.all();
        assert_eq!(entries[0].event_json, r#"{"i":2}"#);
    }

    #[test]
    fn drain_respects_limit() {
        let dlq = DeadLetterQueue::new(10);
        for _ in 0..5 {
            dlq.push(DeadLetterEntry {
                event_json: "{}".to_string(),
                event_type: "test".to_string(),
                handler_name: "h1".to_string(),
                error: "fail".to_string(),
                timestamp: Utc::now(),
                retry_count: 0,
            });
        }
        let drained = dlq.drain(2);
        assert_eq!(drained.len(), 2);
        assert_eq!(dlq.len(), 3);
    }
}
