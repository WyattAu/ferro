use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryStatus {
    Succeeded,
    Failed,
    Pending,
    Retrying,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct DeliveryRecord {
    pub webhook_id: String,
    pub payload_id: String,
    pub status: DeliveryStatus,
    pub status_code: Option<u16>,
    pub attempts: u32,
    pub last_attempt: Option<DateTime<Utc>>,
    pub next_retry: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub webhook_id: String,
    pub payload_id: String,
    pub status: DeliveryStatus,
    pub status_code: Option<u16>,
    pub response_body: Option<String>,
    pub duration: Duration,
    pub next_retry: Option<DateTime<Utc>>,
}

impl DeliveryResult {
    pub fn new(webhook_id: String, payload_id: String) -> Self {
        Self {
            webhook_id,
            payload_id,
            status: DeliveryStatus::Pending,
            status_code: None,
            response_body: None,
            duration: Duration::ZERO,
            next_retry: None,
        }
    }

    pub fn succeeded(webhook_id: String, payload_id: String, status_code: u16, duration: Duration) -> Self {
        Self {
            webhook_id,
            payload_id,
            status: DeliveryStatus::Succeeded,
            status_code: Some(status_code),
            response_body: None,
            duration,
            next_retry: None,
        }
    }

    pub fn failed(webhook_id: String, payload_id: String, status_code: Option<u16>, duration: Duration) -> Self {
        Self {
            webhook_id,
            payload_id,
            status: DeliveryStatus::Failed,
            status_code,
            response_body: None,
            duration,
            next_retry: None,
        }
    }

    pub fn retrying(webhook_id: String, payload_id: String, attempt: u32, backoff_base: Duration) -> Self {
        let delay = backoff_base.as_secs().saturating_pow(attempt.min(10));
        let next_retry = Utc::now() + Duration::from_secs(delay);
        Self {
            webhook_id,
            payload_id,
            status: DeliveryStatus::Retrying,
            status_code: None,
            response_body: None,
            duration: Duration::ZERO,
            next_retry: Some(next_retry),
        }
    }

    pub fn disabled(webhook_id: String, payload_id: String) -> Self {
        Self {
            webhook_id,
            payload_id,
            status: DeliveryStatus::Disabled,
            status_code: None,
            response_body: None,
            duration: Duration::ZERO,
            next_retry: None,
        }
    }
}

impl DeliveryRecord {
    pub fn from_result(result: &DeliveryResult) -> Self {
        Self {
            webhook_id: result.webhook_id.clone(),
            payload_id: result.payload_id.clone(),
            status: result.status.clone(),
            status_code: result.status_code,
            attempts: 1,
            last_attempt: Some(Utc::now()),
            next_retry: result.next_retry,
        }
    }

    pub fn increment_attempt(mut self) -> Self {
        self.attempts += 1;
        self.last_attempt = Some(Utc::now());
        self
    }
}

pub fn calculate_backoff(attempt: u32, backoff_base: Duration) -> Duration {
    let secs = backoff_base.as_secs().saturating_pow(attempt.min(10));
    Duration::from_secs(secs)
}

#[derive(Debug, Clone)]
pub struct DeliveryTracker {
    records: HashMap<String, Vec<DeliveryRecord>>,
}

impl DeliveryTracker {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn record(&mut self, webhook_id: &str, result: &DeliveryResult) {
        let entry = self.records.entry(webhook_id.to_string()).or_default();
        if let Some(last) = entry.last_mut()
            && last.payload_id == result.payload_id
        {
            *last = last.clone().increment_attempt();
            last.status = result.status.clone();
            last.status_code = result.status_code;
            last.next_retry = result.next_retry;
            return;
        }
        entry.push(DeliveryRecord::from_result(result));
    }

    pub fn get_history(&self, webhook_id: &str) -> Vec<DeliveryRecord> {
        self.records.get(webhook_id).cloned().unwrap_or_default()
    }

    pub fn get_pending(&self, webhook_id: &str) -> Vec<&DeliveryRecord> {
        self.records
            .get(webhook_id)
            .map(|v| v.iter().filter(|r| r.status == DeliveryStatus::Retrying).collect())
            .unwrap_or_default()
    }
}

impl Default for DeliveryTracker {
    fn default() -> Self {
        Self::new()
    }
}
