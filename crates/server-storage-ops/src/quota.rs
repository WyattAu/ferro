//! Storage quota enforcement and usage tracking.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use common::server_context::HasQuota;

/// Storage quota information.
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub used_bytes: u64,
    pub quota_bytes: u64,
    pub used_percent: f64,
    pub file_count: u64,
    pub unlimited: bool,
}

pub async fn get_quota_impl<S: HasQuota>(state: &S) -> Response {
    let quota_bytes = state.quota_bytes().unwrap_or(0);
    let unlimited = state.quota_bytes().is_none();

    let used_bytes = state.used_bytes().load(std::sync::atomic::Ordering::Relaxed);

    let file_count = state.file_count().load(std::sync::atomic::Ordering::Relaxed);

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

/// Parse a human-readable size string (e.g., "10GB") into bytes.
#[must_use]
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

    #[test]
    fn test_parse_human_size_bytes() {
        assert_eq!(parse_human_size("100B"), Some(100));
        assert_eq!(parse_human_size("0B"), Some(0));
    }

    #[test]
    fn test_parse_human_size_plain_number() {
        // Plain number defaults to bytes multiplier
        assert_eq!(parse_human_size("500"), Some(500));
    }

    #[test]
    fn test_parse_human_size_whitespace() {
        assert_eq!(parse_human_size(" 10GB "), Some(10_737_418_240));
    }
}
