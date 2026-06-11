pub use ferro_server_security::security::*;

pub fn response_require_password_change() -> axum::response::Response {
    ferro_server_security::security::response_require_password_change()
}

pub async fn auth_guard_middleware(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    ferro_server_security::security::auth_guard_middleware::<crate::AppState>(
        axum::extract::State(state),
        req,
        next,
    )
    .await
}
