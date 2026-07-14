use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GDPR consent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsentType {
    /// Essential services (no consent required)
    Essential,
    /// Marketing communications
    Marketing,
    /// Analytics and tracking
    Analytics,
    /// Third-party data sharing
    ThirdPartySharing,
}

/// Consent record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub user_id: String,
    pub consent_type: ConsentType,
    pub granted: bool,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub withdrawal_timestamp: Option<DateTime<Utc>>,
}

/// Data processing purpose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingPurpose {
    /// Contract performance
    ContractPerformance,
    /// Legitimate interest
    LegitimateInterest,
    /// Legal obligation
    LegalObligation,
    /// Consent
    Consent,
}

/// Data processing record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingRecord {
    pub purpose: ProcessingPurpose,
    pub data_categories: Vec<String>,
    pub retention_period: String,
    pub security_measures: Vec<String>,
    pub legal_basis: String,
}

/// GDPR compliance manager
pub struct GdprManager {
    consent_records: HashMap<String, Vec<ConsentRecord>>,
    _processing_records: Vec<ProcessingRecord>,
}

impl Default for GdprManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GdprManager {
    pub fn new() -> Self {
        Self {
            consent_records: HashMap::new(),
            _processing_records: Vec::new(),
        }
    }

    /// Record consent
    pub fn record_consent(
        &mut self,
        user_id: &str,
        consent_type: ConsentType,
        granted: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> ConsentRecord {
        let record = ConsentRecord {
            user_id: user_id.to_string(),
            consent_type,
            granted,
            timestamp: Utc::now(),
            ip_address,
            user_agent,
            withdrawal_timestamp: None,
        };

        self.consent_records
            .entry(user_id.to_string())
            .or_default()
            .push(record.clone());

        record
    }

    /// Withdraw consent
    pub fn withdraw_consent(&mut self, user_id: &str, consent_type: &ConsentType) -> Option<ConsentRecord> {
        if let Some(records) = self.consent_records.get_mut(user_id) {
            for record in records.iter_mut() {
                if record.consent_type == *consent_type && record.granted {
                    record.granted = false;
                    record.withdrawal_timestamp = Some(Utc::now());
                    return Some(record.clone());
                }
            }
        }
        None
    }

    /// Check if consent is granted
    pub fn has_consent(&self, user_id: &str, consent_type: &ConsentType) -> bool {
        self.consent_records
            .get(user_id)
            .map(|records| {
                records
                    .iter()
                    .rev()
                    .find(|r| r.consent_type == *consent_type)
                    .map(|r| r.granted)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Export user data (GDPR Article 20)
    pub fn export_user_data(&self, user_id: &str) -> HashMap<String, serde_json::Value> {
        let mut data = HashMap::new();

        if let Some(records) = self.consent_records.get(user_id) {
            data.insert("consent_records".to_string(), serde_json::to_value(records).unwrap());
        }

        data
    }

    /// Delete user data (GDPR Article 17)
    pub fn delete_user_data(&mut self, user_id: &str) {
        self.consent_records.remove(user_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consent_recording() {
        let mut manager = GdprManager::new();

        let record = manager.record_consent(
            "user123",
            ConsentType::Marketing,
            true,
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
        );

        assert_eq!(record.user_id, "user123");
        assert_eq!(record.consent_type, ConsentType::Marketing);
        assert!(record.granted);
    }

    #[test]
    fn test_consent_withdrawal() {
        let mut manager = GdprManager::new();

        manager.record_consent("user123", ConsentType::Marketing, true, None, None);

        let withdrawn = manager.withdraw_consent("user123", &ConsentType::Marketing);
        assert!(withdrawn.is_some());
        assert!(!manager.has_consent("user123", &ConsentType::Marketing));
    }

    #[test]
    fn test_data_export() {
        let mut manager = GdprManager::new();

        manager.record_consent("user123", ConsentType::Marketing, true, None, None);

        let data = manager.export_user_data("user123");
        assert!(data.contains_key("consent_records"));
    }

    #[test]
    fn test_data_deletion() {
        let mut manager = GdprManager::new();

        manager.record_consent("user123", ConsentType::Marketing, true, None, None);

        manager.delete_user_data("user123");
        assert!(!manager.has_consent("user123", &ConsentType::Marketing));
    }
}
