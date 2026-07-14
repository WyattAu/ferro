use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;

pub use ferro_server_config::*;

/// GET /api/config — return server configuration and capabilities.
pub async fn get_server_config(State(state): State<AppState>) -> Response {
    let body = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "auth_enabled": state.auth_enabled(),
        "search_enabled": state.search.is_some(),
        "wasm_enabled": state.wasm_runtime.is_some(),
        "wasm_workers_enabled": state.wasm_runtime.is_some(),
        "cedar_enabled": state.cedar.is_some(),
        "metadata_persistent": state.metadata_store.is_some(),
        "cas_enabled": state.cas_store.is_some(),
        "storage": "configured",
        "external_url": state.external_url,
        "wopi_configured": !state.wopi_office_url.is_empty(),
    });
    (StatusCode::OK, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_config_auth_disabled_without_oidc() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["auth_enabled"], false);
    }

    #[tokio::test]
    async fn test_config_has_required_fields() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        assert!(json.get("version").is_some());
        assert!(json.get("auth_enabled").is_some());
        assert!(json.get("search_enabled").is_some());
        assert!(json.get("wasm_workers_enabled").is_some());
        assert!(json.get("cedar_enabled").is_some());
        assert!(json.get("metadata_persistent").is_some());
        assert!(json.get("cas_enabled").is_some());
        assert!(json.get("storage").is_some());
    }

    #[tokio::test]
    async fn test_config_metadata_persistent_false() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        assert_eq!(json["metadata_persistent"], false);
    }

    #[tokio::test]
    async fn test_config_cas_enabled_false() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        assert_eq!(json["cas_enabled"], false);
    }
}
