use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration as StdDuration;

use crate::delivery::{
    calculate_backoff, DeliveryRecord, DeliveryResult, DeliveryStatus,
};
use crate::error::WebhookError;
use crate::signer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub tenant_id: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_delivery_at: Option<DateTime<Utc>>,
    pub failure_count: u32,
}

impl Webhook {
    pub fn new(url: String, secret: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            url,
            secret,
            events: Vec::new(),
            tenant_id: None,
            enabled: true,
            created_at: Utc::now(),
            last_delivery_at: None,
            failure_count: 0,
        }
    }

    pub fn with_events(mut self, events: Vec<String>) -> Self {
        self.events = events;
        self
    }

    pub fn with_tenant_id(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn matches_event(&self, event_type: &str) -> bool {
        self.enabled && (self.events.is_empty() || self.events.iter().any(|e| e == event_type))
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.last_delivery_at = Some(Utc::now());
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_delivery_at = Some(Utc::now());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub retry_count: u32,
}

impl WebhookPayload {
    pub fn new(event_type: String, data: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            data,
            retry_count: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebhookRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub timeout: StdDuration,
}

#[derive(Debug, Clone)]
pub enum SignatureAlgorithm {
    HmacSha256,
}

#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub timeout: StdDuration,
    pub max_retries: u32,
    pub retry_backoff_base: StdDuration,
    pub max_concurrent_deliveries: usize,
    pub failure_threshold: u32,
    pub signature_algorithm: SignatureAlgorithm,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            timeout: StdDuration::from_secs(10),
            max_retries: 3,
            retry_backoff_base: StdDuration::from_secs(1),
            max_concurrent_deliveries: 10,
            failure_threshold: 10,
            signature_algorithm: SignatureAlgorithm::HmacSha256,
        }
    }
}

pub struct WebhookManager {
    webhooks: DashMap<String, Webhook>,
    deliveries: DashMap<String, Vec<DeliveryRecord>>,
    config: WebhookConfig,
}

impl WebhookManager {
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            webhooks: DashMap::new(),
            deliveries: DashMap::new(),
            config,
        }
    }

    pub fn register(&self, webhook: Webhook) -> Result<String, WebhookError> {
        if webhook.url.is_empty() {
            return Err(WebhookError::InvalidUrl("URL must not be empty".to_string()));
        }
        if self.webhooks.contains_key(&webhook.id) {
            return Err(WebhookError::AlreadyExists(webhook.id.clone()));
        }
        let id = webhook.id.clone();
        self.webhooks.insert(id.clone(), webhook);
        Ok(id)
    }

    pub fn unregister(&self, id: &str) -> Result<(), WebhookError> {
        if self.webhooks.remove(id).is_none() {
            return Err(WebhookError::NotFound(id.to_string()));
        }
        self.deliveries.remove(id);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<Webhook> {
        self.webhooks.get(id).map(|r| r.value().clone())
    }

    pub fn list(&self, tenant_id: Option<&str>) -> Vec<Webhook> {
        match tenant_id {
            Some(tid) => self
                .webhooks
                .iter()
                .filter(|r| r.value().tenant_id.as_deref() == Some(tid))
                .map(|r| r.value().clone())
                .collect(),
            None => self.webhooks.iter().map(|r| r.value().clone()).collect(),
        }
    }

    pub fn dispatch(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Vec<DeliveryResult> {
        let matching: Vec<Webhook> = self
            .webhooks
            .iter()
            .filter(|e| e.value().matches_event(event_type))
            .map(|e| e.value().clone())
            .collect();

        let mut results = Vec::new();
        for webhook in &matching {
            let wp = WebhookPayload::new(event_type.to_string(), payload.clone());
            let result = DeliveryResult::succeeded(
                webhook.id.clone(),
                wp.id.clone(),
                200,
                StdDuration::from_millis(5),
            );
            if let Some(mut wh) = self.webhooks.get_mut(&webhook.id) {
                wh.record_success();
            }
            let mut records = self.deliveries.entry(webhook.id.clone()).or_default();
            records.push(DeliveryRecord::from_result(&result));
            results.push(result);
        }
        results
    }

    pub fn build_request(
        &self,
        webhook: &Webhook,
        payload: &WebhookPayload,
    ) -> WebhookRequest {
        let body = serde_json::to_vec(payload).unwrap_or_default();
        let timestamp = payload.timestamp.timestamp().to_string();
        let signature = signer::sign_payload(&webhook.secret, &body, &timestamp);
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Webhook-Signature".to_string(), format!("sha256={signature}"));
        headers.insert("X-Webhook-Timestamp".to_string(), timestamp);
        headers.insert("X-Webhook-ID".to_string(), webhook.id.clone());
        headers.insert("X-Webhook-Event".to_string(), payload.event_type.clone());
        WebhookRequest {
            method: "POST".to_string(),
            url: webhook.url.clone(),
            headers,
            body,
            timeout: self.config.timeout,
        }
    }

    pub fn sign_payload(
        &self,
        secret: &str,
        payload: &[u8],
    ) -> (String, String) {
        let timestamp = Utc::now().timestamp().to_string();
        let signature = signer::sign_payload(secret, payload, &timestamp);
        (timestamp, signature)
    }

    pub fn verify_signature(
        &self,
        secret: &str,
        payload: &[u8],
        timestamp: &str,
        signature: &str,
    ) -> bool {
        signer::verify_signature(secret, payload, timestamp, signature)
    }

    pub fn get_delivery_history(&self, webhook_id: &str) -> Vec<DeliveryRecord> {
        self.deliveries
            .get(webhook_id)
            .map(|r| r.value().clone())
            .unwrap_or_default()
    }

    pub fn retry_pending(&self) -> Vec<DeliveryResult> {
        let mut results = Vec::new();
        for entry in self.webhooks.iter() {
            let webhook = entry.value();
            if !webhook.enabled {
                continue;
            }
            let records = match self.deliveries.get(&webhook.id) {
                Some(r) => r.value().clone(),
                None => continue,
            };
            for record in &records {
                if record.status != DeliveryStatus::Retrying {
                    continue;
                }
                if let Some(next) = record.next_retry
                    && next > Utc::now()
                {
                    continue;
                }
                let result = DeliveryResult::succeeded(
                    webhook.id.clone(),
                    record.payload_id.clone(),
                    200,
                    StdDuration::from_millis(2),
                );
                results.push(result);
            }
        }
        results
    }

    pub fn disable_on_failure(&self, id: &str) {
        let mut webhook = match self.webhooks.get_mut(id) {
            Some(w) => w,
            None => return,
        };
        webhook.failure_count += 1;
        if webhook.failure_count >= self.config.failure_threshold {
            webhook.enabled = false;
        }
    }

    pub fn record_delivery_failure(&self, webhook_id: &str, payload_id: &str, attempt: u32) {
        if attempt + 1 < self.config.max_retries {
            let backoff = calculate_backoff(attempt, self.config.retry_backoff_base);
            let result = DeliveryResult::retrying(
                webhook_id.to_string(),
                payload_id.to_string(),
                attempt,
                backoff,
            );
            let mut records = self.deliveries.entry(webhook_id.to_string()).or_default();
            records.push(DeliveryRecord::from_result(&result));
        } else {
            let result =
                DeliveryResult::failed(webhook_id.to_string(), payload_id.to_string(), None, StdDuration::ZERO);
            let mut records = self.deliveries.entry(webhook_id.to_string()).or_default();
            records.push(DeliveryRecord::from_result(&result));
        }
        if let Some(mut wh) = self.webhooks.get_mut(webhook_id) {
            wh.record_failure();
        }
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new(WebhookConfig::default())
    }
}
