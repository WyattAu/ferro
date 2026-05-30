use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: i64,
    pub line: String,
    pub labels: std::collections::HashMap<String, String>,
    pub level: String,
    pub source: String,
}

pub struct LogBuffer {
    entries: RwLock<VecDeque<LogEntry>>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub fn push(&self, entry: LogEntry) {
        let mut entries = self.entries.write();
        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    pub fn query(&self, filter_level: Option<&str>, limit: usize) -> Vec<LogEntry> {
        let entries = self.entries.read();
        let result: Vec<_> = entries
            .iter()
            .rev()
            .filter(|e| filter_level.is_none_or(|l| e.level == l))
            .take(limit)
            .cloned()
            .collect();
        result
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }
}
