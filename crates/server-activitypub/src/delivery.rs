use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::Value;
use sha2::Sha256;
use tracing::{info, warn};

use crate::FederationState;

type HmacSha256 = Hmac<Sha256>;

static FEDERATION_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("ferro-server/2.5.0")
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|e| {
            tracing::error!("Failed to build federation HTTP client: {e}");
            reqwest::Client::new()
        })
});

pub fn federation_client() -> &'static reqwest::Client {
    &FEDERATION_CLIENT
}

fn sign_request(method: &str, url: &str, secret: &str, key_id: &str) -> Result<String, String> {
    let path = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .skip(1)
        .collect::<Vec<&str>>()
        .join("/");
    let path = format!("/{}", path);

    let signing_string = format!("(request-target): {} {}", method.to_lowercase(), path);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| format!("HMAC error: {}", e))?;
    mac.update(signing_string.as_bytes());
    let sig_bytes = mac.finalize().into_bytes();
    let sig_b64 = STANDARD.encode(sig_bytes);

    Ok(format!(
        r#"keyId="{}",algorithm="hs2019",headers="(request-target)",signature="{}""#,
        key_id, sig_b64
    ))
}

pub async fn deliver_to_followers(state: &FederationState, activity: &Value) -> Vec<Result<(), String>> {
    let followers = state.activity_store.get_followers("admin");
    let mut results = Vec::new();

    for follower_url in &followers {
        let inbox_url = format!("{}/inbox", follower_url.trim_end_matches('/'));
        match deliver_to_inbox(&inbox_url, activity, &state.federation_secret, &state.external_url).await {
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

pub async fn deliver_to_inbox(
    inbox_url: &str,
    activity: &Value,
    secret: &str,
    external_url: &str,
) -> Result<(), String> {
    let key_id = format!("{}/fed/actor/admin#main-key", external_url);
    let signature = sign_request("POST", inbox_url, secret, &key_id)?;

    let response = FEDERATION_CLIENT
        .post(inbox_url)
        .header("Content-Type", "application/activity+json")
        .header("Signature", &signature)
        .header(
            "Date",
            chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        )
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
    fn test_sign_request() {
        let sig = sign_request(
            "POST",
            "https://remote.example.com/inbox",
            "test-secret",
            "https://local.example.com/fed/actor/admin#main-key",
        );
        assert!(sig.is_ok());
        let sig = sig.unwrap();
        assert!(sig.contains("keyId=\"https://local.example.com/fed/actor/admin#main-key\""));
        assert!(sig.contains("algorithm=\"hs2019\""));
        assert!(sig.contains("headers=\"(request-target)\""));
        assert!(sig.contains("signature=\""));
    }

    #[test]
    fn test_sign_request_verification_roundtrip() {
        let secret = "test-secret-key";
        let key_id = "https://local.example.com/fed/actor/admin#main-key";
        let url = "https://remote.example.com/inbox";

        let sig_header = sign_request("POST", url, secret, key_id).unwrap();

        let parsed = crate::http_sig::HttpSignature::parse(&sig_header).unwrap();
        let result = parsed.verify_hmac(
            &axum::http::Method::POST,
            "/inbox",
            &axum::http::HeaderMap::new(),
            secret,
        );
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_deliver_to_inbox_invalid_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            deliver_to_inbox(
                "not-a-valid-url",
                &json!({"type": "Create"}),
                "secret",
                "https://local.example.com",
            )
            .await
        });
        assert!(result.is_err());
    }
}
