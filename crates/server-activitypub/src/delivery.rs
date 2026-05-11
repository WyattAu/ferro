use serde_json::Value;
use tracing::{info, warn};

use crate::FederationState;

static FEDERATION_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("ferro-server/2.5.0")
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build federation HTTP client")
});

pub async fn deliver_to_followers(
    state: &FederationState,
    activity: &Value,
) -> Vec<Result<(), String>> {
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

pub async fn deliver_to_inbox(inbox_url: &str, activity: &Value) -> Result<(), String> {
    let response = FEDERATION_CLIENT
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
