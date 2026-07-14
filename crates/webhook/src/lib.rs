pub mod delivery;
pub mod error;
pub mod signer;
pub mod webhook;

pub use delivery::{DeliveryRecord, DeliveryResult, DeliveryStatus};
pub use error::WebhookError;
pub use webhook::{Webhook, WebhookConfig, WebhookManager, WebhookPayload, WebhookRequest};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    use crate::delivery::calculate_backoff;

    fn make_manager() -> WebhookManager {
        WebhookManager::new(WebhookConfig::default())
    }

    fn make_webhook(url: &str, events: Vec<&str>) -> Webhook {
        Webhook::new(url.to_string(), "test-secret".to_string())
            .with_events(events.into_iter().map(String::from).collect())
    }

    #[test]
    fn register_webhook() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec!["file.created"]);
        let id = mgr.register(wh).unwrap();
        assert!(mgr.get(&id).is_some());
    }

    #[test]
    fn register_returns_id() {
        let mgr = make_manager();
        let mut wh = make_webhook("https://example.com/hook", vec![]);
        let expected = "fixed-id".to_string();
        wh.id = expected.clone();
        let id = mgr.register(wh).unwrap();
        assert_eq!(id, expected);
    }

    #[test]
    fn register_duplicate_fails() {
        let mgr = make_manager();
        let mut wh1 = make_webhook("https://example.com/hook", vec!["file.created"]);
        wh1.id = "same-id".to_string();
        let mut wh2 = make_webhook("https://example.com/hook2", vec!["file.deleted"]);
        wh2.id = "same-id".to_string();
        mgr.register(wh1).unwrap();
        let err = mgr.register(wh2).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn register_empty_url_fails() {
        let mgr = make_manager();
        let wh = make_webhook("", vec!["file.created"]);
        let err = mgr.register(wh).unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn unregister_webhook() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let id = mgr.register(wh).unwrap();
        mgr.unregister(&id).unwrap();
        assert!(mgr.get(&id).is_none());
    }

    #[test]
    fn unregister_nonexistent_fails() {
        let mgr = make_manager();
        let err = mgr.unregister("nope").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn unregister_clears_delivery_history() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec!["file.created"]);
        let id = mgr.register(wh).unwrap();
        mgr.dispatch("file.created", serde_json::json!({"path": "/a.txt"}));
        assert!(!mgr.get_delivery_history(&id).is_empty());
        mgr.unregister(&id).unwrap();
        assert!(mgr.get_delivery_history(&id).is_empty());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let mgr = make_manager();
        assert!(mgr.get("nope").is_none());
    }

    #[test]
    fn list_all_webhooks() {
        let mgr = make_manager();
        mgr.register(make_webhook("https://a.com/h", vec![])).unwrap();
        mgr.register(make_webhook("https://b.com/h", vec![])).unwrap();
        assert_eq!(mgr.list(None).len(), 2);
    }

    #[test]
    fn list_filtered_by_tenant() {
        let mgr = make_manager();
        let mut wh1 = make_webhook("https://a.com/h", vec![]);
        wh1 = wh1.with_tenant_id("t1".to_string());
        let mut wh2 = make_webhook("https://b.com/h", vec![]);
        wh2 = wh2.with_tenant_id("t2".to_string());
        let mut wh3 = make_webhook("https://c.com/h", vec![]);
        wh3 = wh3.with_tenant_id("t1".to_string());
        mgr.register(wh1).unwrap();
        mgr.register(wh2).unwrap();
        mgr.register(wh3).unwrap();
        assert_eq!(mgr.list(Some("t1")).len(), 2);
        assert_eq!(mgr.list(Some("t2")).len(), 1);
    }

    #[test]
    fn dispatch_to_matching_webhook() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec!["file.created"]);
        let id = mgr.register(wh).unwrap();
        let results = mgr.dispatch("file.created", serde_json::json!({"path": "/a.txt"}));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].webhook_id, id);
        assert_eq!(results[0].status, DeliveryStatus::Succeeded);
    }

    #[test]
    fn dispatch_skips_non_matching_event() {
        let mgr = make_manager();
        mgr.register(make_webhook("https://example.com/hook", vec!["file.deleted"]))
            .unwrap();
        let results = mgr.dispatch("file.created", serde_json::json!({"path": "/a.txt"}));
        assert!(results.is_empty());
    }

    #[test]
    fn dispatch_empty_events_matches_all() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let id = mgr.register(wh).unwrap();
        let r1 = mgr.dispatch("file.created", serde_json::json!({}));
        let r2 = mgr.dispatch("file.deleted", serde_json::json!({}));
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].webhook_id, id);
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].webhook_id, id);
    }

    #[test]
    fn dispatch_skips_disabled_webhook() {
        let mgr = make_manager();
        let mut wh = make_webhook("https://example.com/hook", vec!["file.created"]);
        wh.enabled = false;
        mgr.register(wh).unwrap();
        let results = mgr.dispatch("file.created", serde_json::json!({}));
        assert!(results.is_empty());
    }

    #[test]
    fn dispatch_no_matching_webhooks_empty() {
        let mgr = make_manager();
        let results = mgr.dispatch("file.created", serde_json::json!({}));
        assert!(results.is_empty());
    }

    #[test]
    fn dispatch_multiple_webhooks_same_event() {
        let mgr = make_manager();
        mgr.register(make_webhook("https://a.com/h", vec!["file.created"]))
            .unwrap();
        mgr.register(make_webhook("https://b.com/h", vec!["file.created"]))
            .unwrap();
        mgr.register(make_webhook("https://c.com/h", vec!["file.deleted"]))
            .unwrap();
        let results = mgr.dispatch("file.created", serde_json::json!({}));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn hmac_sha256_sign_and_verify() {
        let mgr = make_manager();
        let payload = b"{\"event\":\"test\"}";
        let (timestamp, sig) = mgr.sign_payload("my-secret", payload);
        assert!(mgr.verify_signature("my-secret", payload, &timestamp, &sig));
    }

    #[test]
    fn hmac_wrong_secret_fails() {
        let mgr = make_manager();
        let payload = b"{\"event\":\"test\"}";
        let (timestamp, sig) = mgr.sign_payload("secret-a", payload);
        assert!(!mgr.verify_signature("secret-b", payload, &timestamp, &sig));
    }

    #[test]
    fn hmac_wrong_payload_fails() {
        let mgr = make_manager();
        let (timestamp, sig) = mgr.sign_payload("secret", b"original");
        assert!(!mgr.verify_signature("secret", b"tampered", &timestamp, &sig));
    }

    #[test]
    fn build_request_has_headers() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let payload = WebhookPayload::new("file.created".to_string(), serde_json::json!({}));
        let req = mgr.build_request(&wh, &payload);
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://example.com/hook");
        assert!(req.headers.contains_key("Content-Type"));
        assert!(req.headers.contains_key("X-Webhook-Signature"));
        assert!(req.headers.contains_key("X-Webhook-Timestamp"));
        assert!(req.headers["X-Webhook-Signature"].starts_with("sha256="));
        assert_eq!(req.timeout, Duration::from_secs(10));
    }

    #[test]
    fn build_request_signature_verifiable() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let payload = WebhookPayload::new("file.created".to_string(), serde_json::json!({"key": "val"}));
        let req = mgr.build_request(&wh, &payload);
        let sig_header = &req.headers["X-Webhook-Signature"];
        let sig = sig_header.strip_prefix("sha256=").unwrap();
        let ts = &req.headers["X-Webhook-Timestamp"];
        assert!(signer::verify_signature(&wh.secret, &req.body, ts, sig));
    }

    #[test]
    fn delivery_tracking_creates_record() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec!["file.created"]);
        let id = mgr.register(wh).unwrap();
        mgr.dispatch("file.created", serde_json::json!({}));
        let history = mgr.get_delivery_history(&id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].webhook_id, id);
        assert_eq!(history[0].status, DeliveryStatus::Succeeded);
    }

    #[test]
    fn retry_logic_exponential_backoff() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let id = mgr.register(wh).unwrap();
        mgr.record_delivery_failure(&id, "payload-1", 0);
        let history = mgr.get_delivery_history(&id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, DeliveryStatus::Retrying);
        assert!(history[0].next_retry.is_some());
    }

    #[test]
    fn retry_exceeds_max_retries_fails() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let id = mgr.register(wh).unwrap();
        mgr.record_delivery_failure(&id, "payload-1", 3);
        let history = mgr.get_delivery_history(&id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, DeliveryStatus::Failed);
    }

    #[test]
    fn disable_on_failure_threshold() {
        let mgr = make_manager();
        let wh = make_webhook("https://example.com/hook", vec![]);
        let id = mgr.register(wh).unwrap();
        assert!(mgr.get(&id).unwrap().enabled);
        for _ in 0..9 {
            mgr.disable_on_failure(&id);
        }
        assert!(mgr.get(&id).unwrap().enabled);
        mgr.disable_on_failure(&id);
        assert!(!mgr.get(&id).unwrap().enabled);
    }

    #[test]
    fn different_webhook_secrets() {
        let mgr = make_manager();
        let wh_a = Webhook::new("https://a.com/h".to_string(), "secret-a".to_string())
            .with_events(vec!["file.created".to_string()]);
        let wh_b = Webhook::new("https://b.com/h".to_string(), "secret-b".to_string())
            .with_events(vec!["file.created".to_string()]);
        mgr.register(wh_a).unwrap();
        mgr.register(wh_b).unwrap();
        let results = mgr.dispatch("file.created", serde_json::json!({}));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn webhook_new_default_values() {
        let wh = Webhook::new("https://example.com/h".to_string(), "secret".to_string());
        assert!(wh.enabled);
        assert!(wh.events.is_empty());
        assert_eq!(wh.failure_count, 0);
        assert!(wh.last_delivery_at.is_none());
        assert!(wh.tenant_id.is_none());
    }

    #[test]
    fn webhook_matches_event_empty_events() {
        let wh = Webhook::new("https://example.com/h".to_string(), "secret".to_string());
        assert!(wh.matches_event("anything"));
    }

    #[test]
    fn webhook_matches_event_specific() {
        let wh = Webhook::new("https://example.com/h".to_string(), "secret".to_string())
            .with_events(vec!["file.created".to_string()]);
        assert!(wh.matches_event("file.created"));
        assert!(!wh.matches_event("file.deleted"));
    }

    #[test]
    fn delivery_result_default_config() {
        let cfg = WebhookConfig::default();
        assert_eq!(cfg.timeout, Duration::from_secs(10));
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.retry_backoff_base, Duration::from_secs(1));
        assert_eq!(cfg.max_concurrent_deliveries, 10);
        assert_eq!(cfg.failure_threshold, 10);
    }

    #[test]
    fn calculate_backoff_values() {
        let base = Duration::from_secs(2);
        assert_eq!(calculate_backoff(0, base), Duration::from_secs(1));
        assert_eq!(calculate_backoff(1, base), Duration::from_secs(2));
        assert_eq!(calculate_backoff(2, base), Duration::from_secs(4));
        assert_eq!(calculate_backoff(3, base), Duration::from_secs(8));
    }

    #[test]
    fn payload_new_generates_id() {
        let p = WebhookPayload::new("file.created".to_string(), serde_json::json!({"key": "val"}));
        assert!(!p.id.is_empty());
        assert_eq!(p.event_type, "file.created");
        assert_eq!(p.retry_count, 0);
    }
}
