use axum::extract::State;
use axum::response::Response;

use crate::AppState;
use crate::api_error::ApiError;

pub use ferro_server_storage_ops::quota::{QuotaInfo, get_quota_impl, parse_human_size};

/// GET /api/quota — return current quota usage.
pub async fn get_quota(State(state): State<AppState>) -> Response {
    get_quota_impl(&state).await
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
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_quota_unlimited() {
        let state = crate::AppState::in_memory();
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
        let quota_state = crate::AppState {
            quota_bytes: Some(10_737_418_240),
            ..crate::AppState::in_memory()
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
        let state = crate::AppState::in_memory();
        state
            .used_bytes
            .store(500, std::sync::atomic::Ordering::Relaxed);
        let check_state = crate::AppState {
            quota_bytes: Some(1000),
            ..crate::AppState::in_memory()
        };
        check_state
            .used_bytes
            .store(500, std::sync::atomic::Ordering::Relaxed);
        assert!(check_quota(&check_state, 400).is_ok());
    }

    #[test]
    fn test_check_quota_exceeded() {
        let check_state = crate::AppState {
            quota_bytes: Some(1000),
            ..crate::AppState::in_memory()
        };
        check_state
            .used_bytes
            .store(800, std::sync::atomic::Ordering::Relaxed);
        assert!(check_quota(&check_state, 300).is_err());
    }

    #[test]
    fn test_record_usage() {
        let state = crate::AppState::in_memory();
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
        let state = crate::AppState {
            quota_bytes: Some(1000),
            ..crate::AppState::in_memory()
        };
        state
            .used_bytes
            .store(500, std::sync::atomic::Ordering::Relaxed);
        assert!(enforce_quota(&state, 400).is_ok());
    }

    #[test]
    fn test_enforce_quota_exceeded() {
        let state = crate::AppState {
            quota_bytes: Some(1000),
            ..crate::AppState::in_memory()
        };
        state
            .used_bytes
            .store(800, std::sync::atomic::Ordering::Relaxed);
        let result = enforce_quota(&state, 300);
        assert!(result.is_err());
    }

    #[test]
    fn test_enforce_quota_unlimited() {
        let state = crate::AppState::in_memory();
        assert!(enforce_quota(&state, u64::MAX).is_ok());
    }

    #[tokio::test]
    async fn test_calculate_usage() {
        let state = crate::AppState::in_memory();
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
