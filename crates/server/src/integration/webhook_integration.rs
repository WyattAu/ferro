//! Webhook integration.
//!
//! Provides helpers for managing outgoing webhooks.

use ferro_webhook::{WebhookManager, Webhook, WebhookConfig};

pub fn create_webhook_manager() -> WebhookManager {
    WebhookManager::new(WebhookConfig::default())
}

pub fn register_webhook(
    manager: &WebhookManager,
    url: &str,
    secret: &str,
    events: Vec<&str>,
) -> Result<String, ferro_webhook::WebhookError> {
    let webhook = Webhook::new(url.to_string(), secret.to_string())
        .with_events(events.into_iter().map(String::from).collect());
    manager.register(webhook)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_webhook_manager() {
        let mgr = create_webhook_manager();
        assert_eq!(mgr.list(None).len(), 0);
    }

    #[test]
    fn test_register_and_list_webhook() {
        let mgr = create_webhook_manager();
        let id = register_webhook(&mgr, "https://example.com/hook", "secret", vec!["file.created"]).unwrap();
        assert!(!id.is_empty());
        assert_eq!(mgr.list(None).len(), 1);
        assert!(mgr.get(&id).is_some());
    }
}
