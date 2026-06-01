use sha2::{Digest, Sha256};

use crate::AuditEntry;

#[derive(Debug, Clone, PartialEq)]
pub struct ChainVerificationResult {
    pub valid: bool,
    pub total: usize,
    pub first_break: Option<usize>,
    pub reason: Option<String>,
}

impl ChainVerificationResult {
    pub fn valid(total: usize) -> Self {
        Self {
            valid: true,
            total,
            first_break: None,
            reason: None,
        }
    }
}

pub fn compute_hash(entry: &AuditEntry) -> String {
    let mut hasher = Sha256::new();

    hasher.update(entry.id.as_bytes());
    hasher.update(entry.timestamp.to_rfc3339().as_bytes());
    hasher.update(format!("{:?}", entry.action).as_bytes());
    hasher.update(entry.actor_id.as_bytes());
    hasher.update(format!("{:?}", entry.resource_type).as_bytes());
    hasher.update(entry.resource_id.as_bytes());

    let mut detail_keys: Vec<&String> = entry.details.keys().collect();
    detail_keys.sort();
    for key in detail_keys {
        if let Some(val) = entry.details.get(key) {
            hasher.update(key.as_bytes());
            hasher.update(serde_json::to_string(val).unwrap_or_default().as_bytes());
        }
    }

    if let Some(ref ip) = entry.ip_address {
        hasher.update(ip.as_bytes());
    }
    if let Some(ref ua) = entry.user_agent {
        hasher.update(ua.as_bytes());
    }
    hasher.update(entry.previous_hash.as_bytes());

    hex::encode(hasher.finalize())
}

pub fn verify_chain(entries: &[AuditEntry]) -> ChainVerificationResult {
    if entries.is_empty() {
        return ChainVerificationResult::valid(0);
    }

    if entries.len() == 1 {
        let expected = compute_hash(&entries[0].hash_from_fields());
        if entries[0].hash == expected {
            return ChainVerificationResult::valid(1);
        }
        return ChainVerificationResult {
            valid: false,
            total: 1,
            first_break: Some(0),
            reason: Some("hash mismatch on single entry".into()),
        };
    }

    for window in entries.windows(2) {
        let prev = &window[0];
        let curr = &window[1];

        let expected_prev_hash = compute_hash(&prev.hash_from_fields());
        if prev.hash != expected_prev_hash {
            return ChainVerificationResult {
                valid: false,
                total: entries.len(),
                first_break: Some(entries.iter().position(|e| e.id == prev.id).unwrap()),
                reason: Some(format!("hash mismatch for entry {}", prev.id)),
            };
        }

        if curr.previous_hash != prev.hash {
            let idx = entries.iter().position(|e| e.id == curr.id).unwrap();
            return ChainVerificationResult {
                valid: false,
                total: entries.len(),
                first_break: Some(idx),
                reason: Some(format!(
                    "previous_hash mismatch for entry {}: expected {}, got {}",
                    curr.id, prev.hash, curr.previous_hash
                )),
            };
        }
    }

    let last = entries.last().unwrap();
    let expected_last_hash = compute_hash(&last.hash_from_fields());
    if last.hash != expected_last_hash {
        return ChainVerificationResult {
            valid: false,
            total: entries.len(),
            first_break: Some(entries.len() - 1),
            reason: Some(format!("hash mismatch for last entry {}", last.id)),
        };
    }

    ChainVerificationResult::valid(entries.len())
}

trait HashFields {
    fn hash_from_fields(&self) -> AuditEntry;
}

impl HashFields for AuditEntry {
    fn hash_from_fields(&self) -> AuditEntry {
        AuditEntry {
            id: self.id.clone(),
            timestamp: self.timestamp,
            action: self.action.clone(),
            actor_id: self.actor_id.clone(),
            resource_type: self.resource_type.clone(),
            resource_id: self.resource_id.clone(),
            details: self.details.clone(),
            ip_address: self.ip_address.clone(),
            user_agent: self.user_agent.clone(),
            previous_hash: self.previous_hash.clone(),
            hash: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditAction, AuditEntry, ResourceType};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_entry(id: &str, prev_hash: &str) -> AuditEntry {
        let mut entry = AuditEntry {
            id: id.to_string(),
            timestamp: Utc::now(),
            action: AuditAction::FileCreate,
            actor_id: "user-1".to_string(),
            resource_type: ResourceType::File,
            resource_id: "file-1".to_string(),
            details: HashMap::new(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: None,
            previous_hash: prev_hash.to_string(),
            hash: String::new(),
        };
        entry.hash = compute_hash(&entry);
        entry
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let entry = make_entry("a", "");
        let hash1 = compute_hash(&entry.hash_from_fields());
        let hash2 = compute_hash(&entry.hash_from_fields());
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_compute_hash_differs_on_field_change() {
        let e1 = make_entry("a", "");
        let mut e2 = e1.clone();
        e2.action = AuditAction::FileDelete;
        let h1 = compute_hash(&e1.hash_from_fields());
        let h2 = compute_hash(&e2.hash_from_fields());
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_chain_empty() {
        let result = verify_chain(&[]);
        assert!(result.valid);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_verify_chain_single_entry() {
        let entry = make_entry("1", "");
        let result = verify_chain(&[entry]);
        assert!(result.valid);
        assert_eq!(result.total, 1);
    }

    #[test]
    fn test_verify_chain_single_entry_tampered() {
        let mut entry = make_entry("1", "");
        entry.hash = "deadbeef".to_string();
        let result = verify_chain(&[entry]);
        assert!(!result.valid);
        assert_eq!(result.first_break, Some(0));
    }

    #[test]
    fn test_verify_chain_valid_multiple() {
        let e1 = make_entry("1", "");
        let e2 = make_entry("2", &e1.hash);
        let e3 = make_entry("3", &e2.hash);
        let result = verify_chain(&[e1, e2, e3]);
        assert!(result.valid);
        assert_eq!(result.total, 3);
    }

    #[test]
    fn test_verify_chain_tampered_middle() {
        let e1 = make_entry("1", "");
        let e2 = make_entry("2", &e1.hash);
        let e3 = make_entry("3", &e2.hash);
        let mut tampered = e2.clone();
        tampered.details.insert("evil".to_string(), serde_json::Value::String("yes".to_string()));
        let result = verify_chain(&[e1, tampered, e3]);
        assert!(!result.valid);
    }

    #[test]
    fn test_verify_chain_missing_entry() {
        let e1 = make_entry("1", "");
        let e2 = make_entry("2", &e1.hash);
        let mut e3 = make_entry("3", "wrong_hash");
        e3.hash = compute_hash(&e3.hash_from_fields());
        let result = verify_chain(&[e1, e2, e3]);
        assert!(!result.valid);
        assert!(result.first_break.is_some());
    }
}
