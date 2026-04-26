use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub used_bytes: u64,
    pub quota_bytes: u64,
    pub used_percent: f64,
    pub file_count: u64,
    pub unlimited: bool,
}

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

    (StatusCode::OK, axum::Json(QuotaInfo {
        used_bytes,
        quota_bytes,
        used_percent,
        file_count,
        unlimited,
    })).into_response()
}

pub fn check_quota(state: &AppState, content_len: u64) -> Result<(), &'static str> {
    if let Some(quota_bytes) = state.quota_bytes {
        let used = state.used_bytes.load(std::sync::atomic::Ordering::Relaxed);
        if used + content_len > quota_bytes {
            return Err("QUOTA_EXCEEDED");
        }
    }
    Ok(())
}

pub fn record_usage(state: &AppState, bytes: i64) {
    state.used_bytes.fetch_update(
        std::sync::atomic::Ordering::Relaxed,
        std::sync::atomic::Ordering::Relaxed,
        |current| current.checked_add_signed(bytes),
    ).ok();

    if bytes > 0 {
        state.file_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    } else if bytes < 0 {
        state.file_count.fetch_update(
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
            |c| c.checked_sub(1),
        ).ok();
    }
}

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
        let state = crate::AppState::in_memory();
        let response = get_quota(axum::extract::State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = http_body_util::BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let info: QuotaInfo = serde_json::from_slice(&bytes).unwrap();
        assert!(info.unlimited);
        assert_eq!(info.used_bytes, 0);
    }

    #[tokio::test]
    async fn test_quota_with_limit() {
        let quota_state = crate::AppState {
            quota_bytes: Some(10_737_418_240),
            ..crate::AppState::in_memory()
        };
        quota_state.used_bytes.store(5_000_000_000, std::sync::atomic::Ordering::Relaxed);

        let response = get_quota(axum::extract::State(quota_state)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = http_body_util::BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
        let info: QuotaInfo = serde_json::from_slice(&bytes).unwrap();
        assert!(!info.unlimited);
        assert_eq!(info.quota_bytes, 10_737_418_240);
        assert!((info.used_percent - 46.57).abs() < 0.1);
    }

    #[test]
    fn test_check_quota_under_limit() {
        let state = crate::AppState::in_memory();
        state.used_bytes.store(500, std::sync::atomic::Ordering::Relaxed);
        let check_state = crate::AppState {
            quota_bytes: Some(1000),
            ..crate::AppState::in_memory()
        };
        check_state.used_bytes.store(500, std::sync::atomic::Ordering::Relaxed);
        assert!(check_quota(&check_state, 400).is_ok());
    }

    #[test]
    fn test_check_quota_exceeded() {
        let check_state = crate::AppState {
            quota_bytes: Some(1000),
            ..crate::AppState::in_memory()
        };
        check_state.used_bytes.store(800, std::sync::atomic::Ordering::Relaxed);
        assert!(check_quota(&check_state, 300).is_err());
    }

    #[test]
    fn test_record_usage() {
        let state = crate::AppState::in_memory();
        record_usage(&state, 100);
        assert_eq!(state.used_bytes.load(std::sync::atomic::Ordering::Relaxed), 100);
        assert_eq!(state.file_count.load(std::sync::atomic::Ordering::Relaxed), 1);
        record_usage(&state, -50);
        assert_eq!(state.used_bytes.load(std::sync::atomic::Ordering::Relaxed), 50);
        assert_eq!(state.file_count.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
}
