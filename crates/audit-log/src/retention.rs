use std::time::Duration;

use chrono::Utc;

use crate::{AuditAction, AuditEntry};

pub struct RetentionPolicy {
    pub max_entries: Option<usize>,
    pub max_age: Option<Duration>,
    pub action_filter: Option<Vec<AuditAction>>,
}

impl RetentionPolicy {
    pub fn new() -> Self {
        Self {
            max_entries: None,
            max_age: None,
            action_filter: None,
        }
    }

    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = Some(n);
        self
    }

    pub fn max_age(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration);
        self
    }

    pub fn action_filter(mut self, actions: Vec<AuditAction>) -> Self {
        self.action_filter = Some(actions);
        self
    }

    pub fn apply(&self, entries: &mut Vec<AuditEntry>) -> usize {
        let before = entries.len();

        entries.retain(|entry| {
            if let Some(ref filter) = self.action_filter
                && !filter.contains(&entry.action)
            {
                return true;
            }

            if let Some(max_age) = self.max_age {
                let cutoff = Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
                if entry.timestamp < cutoff {
                    return false;
                }
            }

            true
        });

        if let Some(max) = self.max_entries
            && entries.len() > max
        {
            let excess = entries.len() - max;
            entries.drain(0..excess);
        }

        before - entries.len()
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResourceType;
    use chrono::{Duration, Utc};

    fn make_entry(id: &str, age_days: i64, action: AuditAction) -> AuditEntry {
        use std::collections::HashMap;
        AuditEntry {
            id: id.to_string(),
            timestamp: Utc::now() - Duration::days(age_days),
            action,
            actor_id: "user-1".to_string(),
            resource_type: ResourceType::File,
            resource_id: format!("res-{id}"),
            details: HashMap::new(),
            ip_address: None,
            user_agent: None,
            previous_hash: String::new(),
            hash: String::new(),
        }
    }

    #[test]
    fn test_retention_max_entries() {
        let policy = RetentionPolicy::new().max_entries(2);
        let mut entries = vec![
            make_entry("1", 0, AuditAction::FileCreate),
            make_entry("2", 0, AuditAction::FileCreate),
            make_entry("3", 0, AuditAction::FileCreate),
        ];
        let pruned = policy.apply(&mut entries);
        assert_eq!(pruned, 1);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "2");
    }

    #[test]
    fn test_retention_max_age() {
        let policy = RetentionPolicy::new().max_age(std::time::Duration::from_secs(3 * 86400));
        let mut entries = vec![
            make_entry("1", 5, AuditAction::FileCreate),
            make_entry("2", 2, AuditAction::FileCreate),
            make_entry("3", 10, AuditAction::FileCreate),
        ];
        let pruned = policy.apply(&mut entries);
        assert_eq!(pruned, 2);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "2");
    }

    #[test]
    fn test_retention_action_filter() {
        let policy = RetentionPolicy::new()
            .max_age(std::time::Duration::from_secs(3 * 86400))
            .action_filter(vec![AuditAction::FileCreate]);
        let mut entries = vec![
            make_entry("1", 5, AuditAction::FileCreate),
            make_entry("2", 5, AuditAction::FileDelete),
            make_entry("3", 1, AuditAction::FileCreate),
        ];
        let pruned = policy.apply(&mut entries);
        assert_eq!(pruned, 1);
        assert_eq!(entries[0].id, "2");
        assert_eq!(entries[1].id, "3");
    }

    #[test]
    fn test_retention_no_policy() {
        let policy = RetentionPolicy::new();
        let mut entries = vec![
            make_entry("1", 100, AuditAction::FileCreate),
            make_entry("2", 200, AuditAction::FileCreate),
        ];
        let pruned = policy.apply(&mut entries);
        assert_eq!(pruned, 0);
        assert_eq!(entries.len(), 2);
    }
}
