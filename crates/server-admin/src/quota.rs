use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use ferro_server::AppState;
use ferro_server::api_error::ApiError;

/// Storage quota information.
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub used_bytes: u64,
    pub quota_bytes: u64,
    pub used_percent: f64,
    pub file_count: u64,
    pub unlimited: bool,
}

/// GET /api/quota — return current quota usage.
pub async fn get_quota(State(state): State<AppState>) -> Response {
    let quota_bytes = state.quota_bytes.unwrap_or(0);
    let unlimited = state.quota_bytes.is_none();

    let used_bytes = state.used_bytes.load(std::sync::atomic::Ordering::Relaxed);

    let file_count = state.file_count.load(std::sync::atomic::Ordering::Relaxed);

    let used_percent = if unlimited || quota_bytes == 0 {
        0.0
    } else {
        (used_bytes as f64 / quota_bytes as f64) * 100.0
    };

    (
        StatusCode::OK,
        axum::Json(QuotaInfo {
            used_bytes,
            quota_bytes,
            used_percent,
            file_count,
            unlimited,
        }),
    )
        .into_response()
}

/// Check whether an upload would exceed the storage quota.
pub fn check_quota(state: &AppState, content_len: u64) -> Result<(), &'static str> {
    if let Some(quota_bytes) = state.quota_bytes {
        let used = state.used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        if used + content_len > quota_bytes {
            return Err("QUOTA_EXCEEDED");
        }
    }
    Ok(())
}

/// Record storage usage delta (positive for uploads, negative for deletes).
pub fn record_usage(state: &AppState, bytes: i64) {
    state
        .used_bytes
        .fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |current| current.checked_add_signed(bytes),
        )
        .ok();

    if bytes > 0 {
        state
            .file_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    } else if bytes < 0 {
        state
            .file_count
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |c| c.checked_sub(1),
            )
            .ok();
    }
}

/// Parse a human-readable size string (e.g., "10GB") into bytes.
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("TB") {
        (n.trim_end().to_string(), 1_099_511_627_776u64)
    } else if let Some(n) = s.strip_suffix("GB") {
        (n.trim_end().to_string(), 1_073_741_824u64)
    } else if let Some(n) = s.strip_suffix("MB") {
        (n.trim_end().to_string(), 1_048_576u64)
    } else if let Some(n) = s.strip_suffix("KB") {
        (n.trim_end().to_string(), 1024u64)
    } else if let Some(n) = s.strip_suffix('B') {
        (n.trim_end().to_string(), 1u64)
    } else {
        (s, 1u64)
    };
    num_str.parse::<u64>().ok().map(|n| n * multiplier)
}

/// Check if a PUT operation would exceed the quota (best-effort pre-check).
/// Uses the atomic counter for fast checks. Returns Ok(()) if within quota.
pub fn enforce_quota(
    state: &AppState,
    content_length: u64,
) -> Result<(), Box<axum::response::Response>> {
    if let Some(quota_bytes) = state.quota_bytes {
        if quota_bytes == 0 {
            return Ok(());
        }
        let used = state.used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        if used + content_length > quota_bytes {
            return Err(Box::new(ApiError::quota_exceeded(
                used,
                quota_bytes,
                content_length,
            )));
        }
    }
    Ok(())
}

/// Calculate total storage usage by summing file sizes from the storage backend.
pub async fn calculate_usage(state: &AppState) -> u64 {
    match state.storage.list_all("/", 100_000).await {
        Ok(files) => files
            .iter()
            .filter(|f| !f.is_collection)
            .map(|f| f.size)
            .sum(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_human_size() {
        assert_eq!(parse_human_size("10GB"), Some(10_737_418_240));
        assert_eq!(parse_human_size("500MB"), Some(524_288_000));
        assert_eq!(parse_human_size("1TB"), Some(1_099_511_627_776));
        assert_eq!(parse_human_size("1024KB"), Some(1_048_576));
        assert_eq!(parse_human_size("100B"), Some(100));
        assert_eq!(parse_human_size("0"), Some(0));
        assert_eq!(parse_human_size("invalid"), None);
        assert_eq!(parse_human_size(""), None);
    }

    #[test]
    fn test_parse_human_size_case_insensitive() {
        assert_eq!(parse_human_size("10gb"), Some(10_737_418_240));
        assert_eq!(parse_human_size("Gb"), None);
        assert_eq!(parse_human_size("1Gb"), Some(1_073_741_824));
        assert_eq!(parse_human_size("500mb"), Some(524_288_000));
    }

    #[tokio::test]
    async fn test_quota_unlimited() {
        let state = ferro_server::AppState::in_memory();
        let response = get_quota(axum::extract::State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let info: QuotaInfo = serde_json::from_slice(&bytes).unwrap();
        assert!(info.unlimited);
        assert_eq!(info.used_bytes, 0);
    }

    #[tokio::test]
    async fn test_quota_with_limit() {
        let quota_state = ferro_server::AppState {
            quota_bytes: Some(10_737_418_240),
            ..ferro_server::AppState::in_memory()
        };
        quota_state
            .used_bytes
            .store(5_000_000_000, std::sync::atomic::Ordering::Relaxed);

        let response = get_quota(axum::extract::State(quota_state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let info: QuotaInfo = serde_json::from_slice(&bytes).unwrap();
        assert!(!info.unlimited);
        assert_eq!(info.quota_bytes, 10_737_418_240);
        assert!((info.used_percent - 46.57).abs() < 0.1);
    }

    #[test]
    fn test_check_quota_under_limit() {
        let state = ferro_server::AppState::in_memory();
        state
            .used_bytes
            .store(500, std::sync::atomic::Ordering::Relaxed);
        let check_state = ferro_server::AppState {
            quota_bytes: Some(1000),
            ..ferro_server::AppState::in_memory()
        };
        check_state
            .used_bytes
            .store(500, std::sync::atomic::Ordering::Relaxed);
        assert!(check_quota(&check_state, 400).is_ok());
    }

    #[test]
    fn test_check_quota_exceeded() {
        let check_state = ferro_server::AppState {
            quota_bytes: Some(1000),
            ..ferro_server::AppState::in_memory()
        };
        check_state
            .used_bytes
            .store(800, std::sync::atomic::Ordering::Relaxed);
        assert!(check_quota(&check_state, 300).is_err());
    }

    #[test]
    fn test_record_usage() {
        let state = ferro_server::AppState::in_memory();
        record_usage(&state, 100);
        assert_eq!(
            state.used_bytes.load(std::sync::atomic::Ordering::Relaxed),
            100
        );
        assert_eq!(
            state.file_count.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        record_usage(&state, -50);
        assert_eq!(
            state.used_bytes.load(std::sync::atomic::Ordering::Relaxed),
            50
        );
        assert_eq!(
            state.file_count.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_enforce_quota_ok() {
        let state = ferro_server::AppState {
            quota_bytes: Some(1000),
            ..ferro_server::AppState::in_memory()
        };
        state
            .used_bytes
            .store(500, std::sync::atomic::Ordering::Relaxed);
        assert!(enforce_quota(&state, 400).is_ok());
    }

    #[test]
    fn test_enforce_quota_exceeded() {
        let state = ferro_server::AppState {
            quota_bytes: Some(1000),
            ..ferro_server::AppState::in_memory()
        };
        state
            .used_bytes
            .store(800, std::sync::atomic::Ordering::Relaxed);
        let result = enforce_quota(&state, 300);
        assert!(result.is_err());
    }

    #[test]
    fn test_enforce_quota_unlimited() {
        let state = ferro_server::AppState::in_memory();
        assert!(enforce_quota(&state, u64::MAX).is_ok());
    }

    #[tokio::test]
    async fn test_calculate_usage() {
        let state = ferro_server::AppState::in_memory();
        state
            .storage
            .put("/a.txt", bytes::Bytes::from("hello"), "anon")
            .await
            .unwrap();
        state
            .storage
            .put("/b.txt", bytes::Bytes::from("world!!"), "anon")
            .await
            .unwrap();
        let usage = calculate_usage(&state).await;
        assert_eq!(usage, 12);
    }
}
