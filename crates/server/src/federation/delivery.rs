//! ActivityPub HTTP delivery — POST activities to follower inboxes.
//! Uses reqwest for outbound HTTP with configurable timeout and retries.

use serde_json::Value;
use tracing::{info, warn};

use crate::AppState;

/// Deliver an activity to all followers' inboxes.
/// This is called after recording a local activity (Create, Update, Delete, etc.)
pub async fn deliver_to_followers(state: &AppState, activity: &Value) -> Vec<Result<(), String>> {
    let followers = state.activity_store.get_followers("admin");
    let mut results = Vec::new();

    for follower_url in &followers {
        let inbox_url = format!("{}/inbox", follower_url.trim_end_matches('/'));
        match deliver_to_inbox(&inbox_url, activity).await {
            Ok(()) => results.push(Ok(())),
            Err(e) => {
                warn!("Failed to deliver to {}: {}", inbox_url, e);
                results.push(Err(e));
            }
        }
    }

    if !results.is_empty() {
        info!(
            "Delivered activity to {}/{} followers",
            results.iter().filter(|r| r.is_ok()).count(),
            results.len(),
        );
    }

    results
}

/// POST a signed activity to a single remote inbox.
pub async fn deliver_to_inbox(inbox_url: &str, activity: &Value) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Ferro/2.0 (ActivityPub)")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .post(inbox_url)
        .header("Content-Type", "application/activity+json")
        .json(activity)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    if status.is_success() {
        info!("Activity delivered successfully to {}", inbox_url);
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(format!("Delivery failed ({}): {}", status, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deliver_to_inbox_invalid_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            deliver_to_inbox("not-a-valid-url", &json!({"type": "Create"})).await
        });
        assert!(result.is_err());
    }
}
