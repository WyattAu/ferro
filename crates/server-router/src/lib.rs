//! Generic Axum router builder over ServerState trait.
//!
//! This crate provides a `RouterState` supertrait that combines `ServerState`
//! with the methods needed by middleware. Handlers in this crate are generic
//! over `S: RouterState`, proving that the extract-and-delegate pattern works
//! at the router level.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use ferro_server_state::ServerState;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;

/// Supertrait that combines ServerState with methods needed by middleware.
///
/// This trait extends ServerState with additional methods that the Axum
/// middleware stack needs. Note: maintenance_mode() and started_at()
/// are already on ServerState, so RouterState only adds auth_enabled().
pub trait RouterState: ServerState {
    /// Whether authentication is enabled.
    fn auth_enabled(&self) -> bool;
}

/// Build a generic Axum router over any `S: RouterState`.
///
/// This function proves that Axum's `Router<S>` can work with a generic
/// state type, enabling handler extraction to separate crates.
pub fn build_router<S: RouterState>(state: S) -> Router<S> {
    Router::new()
        .route("/healthz", get(health_handler::<S>))
        .route("/readyz", get(ready_handler::<S>))
        .route("/version", get(version_handler))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .with_state(state)
}

/// Generic health handler.
async fn health_handler<S: RouterState>(State(state): State<S>) -> Response {
    let started = state.started_at();
    let uptime = started.elapsed();
    let maintenance = state.maintenance_mode().load(std::sync::atomic::Ordering::Relaxed);

    let status = if maintenance { "maintenance" } else { "healthy" };

    let body = serde_json::json!({
        "status": status,
        "uptime_seconds": uptime.as_secs(),
        "version": env!("CARGO_PKG_VERSION"),
    });

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Generic readiness handler.
async fn ready_handler<S: RouterState>(State(state): State<S>) -> Response {
    let maintenance = state.maintenance_mode().load(std::sync::atomic::Ordering::Relaxed);

    if maintenance {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "status": "not_ready",
                "reason": "maintenance_mode",
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ready",
        })),
    )
        .into_response()
}

/// Version handler (no state needed).
async fn version_handler() -> Response {
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "edition": "2024",
        })),
    )
        .into_response()
}
