use axum::body::Body;
use axum::http::Request;
use axum::Router;
use bytes::Bytes;
use http_body_util::BodyExt;
use tower::ServiceExt;

use ferro_server::AppState;

pub fn create_test_app_state() -> AppState {
    AppState::in_memory()
}

pub fn create_test_router(state: AppState) -> Router {
    ferro_server::build_router(state)
}

pub async fn make_request(app: &Router, method: &str, path: &str, body: Bytes) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    std::hint::black_box(status);

    let body = response.into_body().collect().await.unwrap();
    std::hint::black_box(body);
}

pub fn generate_test_body(size: usize) -> Bytes {
    let pattern: &[u8] = b"The quick brown fox jumps over the lazy dog. ";
    let mut data = Vec::with_capacity(size);
    while data.len() < size {
        let remaining = size - data.len();
        let to_copy = remaining.min(pattern.len());
        data.extend_from_slice(&pattern[..to_copy]);
    }
    Bytes::from(data)
}

pub async fn create_test_file(state: &AppState, path: &str, size: usize) {
    let body = generate_test_body(size);
    state
        .storage
        .put(path, body, "bench")
        .await
        .unwrap();
}
