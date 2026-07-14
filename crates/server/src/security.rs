pub use ferro_server_security::security::*;

pub fn response_require_password_change() -> axum::response::Response {
    ferro_server_security::security::response_require_password_change()
}

pub async fn auth_guard_middleware<S: ferro_server_security::SecurityAppState>(
    axum::extract::State(state): axum::extract::State<S>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    ferro_server_security::security::auth_guard_middleware::<S>(axum::extract::State(state), req, next).await
}

const CSRF_COOKIE_NAME: &str = "csrf_token";
const CSRF_HEADER_NAME: &str = "x-csrf-token";

/// CSRF protection middleware.
///
/// For safe methods (GET/HEAD/OPTIONS): generates a CSRF token, sets it as a
/// cookie and includes it in the `X-CSRF-Token` response header so clients can
/// echo it back on state-changing requests.
///
/// For unsafe methods (POST/PUT/DELETE/PATCH): validates that the
/// `X-CSRF-Token` request header matches the `csrf_token` cookie value using
/// constant-time comparison.
///
/// Skips validation for:
/// - API key authenticated requests (API-to-API)
/// - WebDAV/CalDAV/CardDAV paths
/// - Health/readiness/metrics endpoints
/// - Public auth paths (login, callback, config, etc.)
pub async fn csrf_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // --- Skip CSRF for non-applicable paths ---
    if should_skip_csrf_path(&path) {
        return next.run(req).await;
    }

    // --- Skip CSRF for API key authenticated requests ---
    // When authenticated via API key, UserInfo.username starts with "api-key:".
    // API key auth is explicit and not vulnerable to CSRF (no browser credential
    // auto-sending for cross-origin API key requests).
    let is_api_key_auth = req
        .extensions()
        .get::<ferro_auth::users::UserInfo>()
        .is_some_and(|u| u.username.starts_with("api-key:"));

    if is_api_key_auth {
        return next.run(req).await;
    }

    // --- Skip CSRF for non-browser requests ---
    // Browser cross-origin requests always include an Origin header. Requests
    // without Origin are either same-origin (browser) or non-browser (API
    // clients, curl, tests). Same-origin browser requests don't need CSRF
    // protection because they carry the user's session/credentials legitimately.
    // Non-browser requests aren't vulnerable to CSRF at all.
    if !req.headers().contains_key("origin") {
        return next.run(req).await;
    }

    // --- Safe methods: set CSRF token ---
    if matches!(
        method,
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) {
        let token = generate_csrf_token();
        let mut response = next.run(req).await;

        // Set cookie (not HttpOnly so JS clients can read it, SameSite=Strict
        // to prevent cross-origin sending, Secure in production).
        let cookie_value = format!("{}={}; Path=/; SameSite=Strict; Max-Age=3600", CSRF_COOKIE_NAME, token,);
        if let Ok(header_val) = axum::http::HeaderValue::from_str(&cookie_value) {
            response
                .headers_mut()
                .append(axum::http::header::SET_COOKIE, header_val);
        }

        // Also return token in response header for non-cookie clients.
        if let Ok(header_val) = axum::http::HeaderValue::from_str(&token) {
            response.headers_mut().insert(CSRF_HEADER_NAME, header_val);
        }

        return response;
    }

    // --- Unsafe methods: validate CSRF token ---
    // Extract token from request header.
    let header_token = req
        .headers()
        .get(CSRF_HEADER_NAME)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Extract token from cookie.
    let cookie_token = req
        .headers()
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|pair| {
                let pair = pair.trim();
                pair.strip_prefix(&format!("{}=", CSRF_COOKIE_NAME))
                    .map(|v| v.trim().to_string())
            })
        })
        .unwrap_or_default();

    // Both must be present and match.
    if header_token.is_empty() || cookie_token.is_empty() {
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "CSRF token missing",
                "error_code": "CSRF_TOKEN_MISSING",
                "message": "State-changing requests require X-CSRF-Token header matching the csrf_token cookie"
            })),
        )
            .into_response();
    }

    if !verify_csrf_token(&cookie_token, header_token) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "CSRF token mismatch",
                "error_code": "CSRF_TOKEN_INVALID",
                "message": "X-CSRF-Token header does not match csrf_token cookie"
            })),
        )
            .into_response();
    }

    next.run(req).await
}

/// Returns `true` for paths that should bypass CSRF protection.
fn should_skip_csrf_path(path: &str) -> bool {
    // Health / readiness / startup
    path == "/healthz"
        || path == "/readyz"
        || path == "/startupz"
        || path == "/health"
        || path == "/.well-known/ferro"
        // Metrics
        || path == "/metrics"
        || path == "/metrics/prometheus"
        // Public auth paths (no credentials sent, read-only)
        || path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/callback")
        || path == "/api/auth/info"
        || path == "/api/config"
        || path == "/api/policies"
        // WebDAV / CalDAV / CardDAV (use their own auth mechanisms)
        || path.starts_with("/dav/")
        || path == "/"
        // WOPI (token-based auth)
        || path.starts_with("/wopi/")
        || path.starts_with("/hosting/")
        // Federation (actor/inbox/outbox use signature-based auth)
        || path.starts_with("/fed/")
        // UI (SPA static files)
        || path.starts_with("/ui/")
        || path == "/ui"
        // Share links (public or token-based)
        || path.starts_with("/s/")
        // WebSocket (upgraded connection)
        || path.starts_with("/ws/")
        // OpenAPI / Swagger
        || path.starts_with("/api-docs")
        || path.starts_with("/swagger")
        || path.starts_with("/api/swagger")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_csrf_health_endpoints() {
        assert!(should_skip_csrf_path("/healthz"));
        assert!(should_skip_csrf_path("/readyz"));
        assert!(should_skip_csrf_path("/startupz"));
        assert!(should_skip_csrf_path("/health"));
        assert!(should_skip_csrf_path("/.well-known/ferro"));
    }

    #[test]
    fn test_skip_csrf_metrics() {
        assert!(should_skip_csrf_path("/metrics"));
        assert!(should_skip_csrf_path("/metrics/prometheus"));
    }

    #[test]
    fn test_skip_csrf_auth_paths() {
        assert!(should_skip_csrf_path("/api/auth/login"));
        assert!(should_skip_csrf_path("/api/auth/login/oidc"));
        assert!(should_skip_csrf_path("/api/auth/callback"));
        assert!(should_skip_csrf_path("/api/auth/callback?code=abc"));
        assert!(should_skip_csrf_path("/api/auth/info"));
        assert!(should_skip_csrf_path("/api/config"));
        assert!(should_skip_csrf_path("/api/policies"));
    }

    #[test]
    fn test_skip_csrf_dav() {
        assert!(should_skip_csrf_path("/dav/"));
        assert!(should_skip_csrf_path("/dav/cal"));
        assert!(should_skip_csrf_path("/dav/card"));
    }

    #[test]
    fn test_skip_csrf_root() {
        assert!(should_skip_csrf_path("/"));
    }

    #[test]
    fn test_skip_csrf_wopi() {
        assert!(should_skip_csrf_path("/wopi/"));
        assert!(should_skip_csrf_path("/wopi/files/123"));
        assert!(should_skip_csrf_path("/hosting/discovery"));
    }

    #[test]
    fn test_skip_csrf_federation() {
        assert!(should_skip_csrf_path("/fed/"));
        assert!(should_skip_csrf_path("/fed/actor/alice"));
        assert!(should_skip_csrf_path("/fed/inbox"));
    }

    #[test]
    fn test_skip_csrf_ui() {
        assert!(should_skip_csrf_path("/ui/"));
        assert!(should_skip_csrf_path("/ui"));
        assert!(should_skip_csrf_path("/ui/index.html"));
    }

    #[test]
    fn test_skip_csrf_shares() {
        assert!(should_skip_csrf_path("/s/"));
        assert!(should_skip_csrf_path("/s/abc123"));
    }

    #[test]
    fn test_skip_csrf_websocket() {
        assert!(should_skip_csrf_path("/ws/"));
        assert!(should_skip_csrf_path("/ws/chat/room1"));
    }

    #[test]
    fn test_skip_csrf_swagger() {
        assert!(should_skip_csrf_path("/api-docs"));
        assert!(should_skip_csrf_path("/swagger-ui"));
        assert!(should_skip_csrf_path("/api/swagger"));
    }

    #[test]
    fn test_no_skip_csrf_api_endpoints() {
        assert!(!should_skip_csrf_path("/api/v1/files"));
        assert!(!should_skip_csrf_path("/api/files/upload"));
        assert!(!should_skip_csrf_path("/api/shares"));
        assert!(!should_skip_csrf_path("/api/admin/stats"));
    }

    #[test]
    fn test_no_skip_csrf_arbitrary_paths() {
        assert!(!should_skip_csrf_path("/some/random/path"));
        assert!(!should_skip_csrf_path("/api/auth/change-password"));
        assert!(!should_skip_csrf_path(""));
    }
}
