use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use base64::Engine;

use crate::api_error::ApiError;

pub async fn simple_auth_middleware(
    req: Request,
    admin_user: Option<String>,
    admin_password: Option<String>,
    next: Next,
) -> Response {
    if admin_user.is_none() || admin_password.is_none() {
        return next.run(req).await;
    }

    let path = req.uri().path();

    if is_public_path(path) {
        return next.run(req).await;
    }

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let encoded = match auth_header {
        Some(h) if h.starts_with("Basic ") => &h[6..],
        _ => {
            return ApiError::unauthorized_with_www_authenticate(
                ApiError::AUTH_REQUIRED,
                "authentication required",
            );
        }
    };

    let decoded = match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(d) => d,
        Err(_) => {
            return ApiError::unauthorized_with_www_authenticate(
                ApiError::INVALID_CREDENTIALS,
                "invalid credentials",
            );
        }
    };

    let credentials = String::from_utf8_lossy(&decoded);
    let (user, pass) = match credentials.split_once(':') {
        Some((u, p)) => (u, p),
        None => {
            return ApiError::unauthorized_with_www_authenticate(
                ApiError::INVALID_CREDENTIALS,
                "invalid credentials",
            );
        }
    };

    let expected_user = admin_user.as_deref().unwrap_or("");
    let expected_pass = admin_password.as_deref().unwrap_or("");

    if user == expected_user && pass == expected_pass {
        next.run(req).await
    } else {
        ApiError::unauthorized_with_www_authenticate(
            ApiError::INVALID_CREDENTIALS,
            "invalid credentials",
        )
    }
}

fn is_public_path(path: &str) -> bool {
    path == "/.well-known/ferro"
        || path == "/.well-known/openid-configuration"
        || path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/callback")
        || path.starts_with("/api/config")
        || path.starts_with("/api/auth/info")
        || path == "/metrics"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn make_auth_app(user: Option<&str>, pass: Option<&str>) -> axum::Router {
        let admin_user = user.map(|s| s.to_string());
        let admin_password = pass.map(|s| s.to_string());
        axum::Router::new()
            .route("/api/test", axum::routing::get(|| async { "ok" }))
            .route("/.well-known/ferro", axum::routing::get(|| async { "ok" }))
            .route("/api/config", axum::routing::get(|| async { "ok" }))
            .route("/api/auth/info", axum::routing::get(|| async { "ok" }))
            .route("/api/auth/login", axum::routing::get(|| async { "ok" }))
            .route("/api/auth/callback", axum::routing::get(|| async { "ok" }))
            .route("/metrics", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(move |req: axum::extract::Request, next: Next| {
                let admin_user = admin_user.clone();
                let admin_password = admin_password.clone();
                async move {
                    simple_auth_middleware(req, admin_user, admin_password, next).await
                }
            }))
    }

    #[tokio::test]
    async fn test_no_auth_required_when_not_configured() {
        let app = make_auth_app(None, None);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_public_paths_bypass_auth() {
        async fn check(path: &str) {
            let app = make_auth_app(Some("admin"), Some("secret"));
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "path {} should be public", path);
        }
        check("/.well-known/ferro").await;
        check("/api/config").await;
        check("/api/auth/info").await;
        check("/api/auth/login?redirect=/ui/").await;
        check("/api/auth/callback?code=test&state=s").await;
        check("/metrics").await;
    }

    #[tokio::test]
    async fn test_valid_credentials_accepted() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_credentials_rejected() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:wrong");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_missing_auth_header_returns_401() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers().get("WWW-Authenticate").is_some());
    }

    #[tokio::test]
    async fn test_malformed_auth_returns_401() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", "Basic not-base64!!!")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_empty_username_rejected() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let creds = base64::engine::general_purpose::STANDARD.encode(":password");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_empty_password_rejected() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_username_with_colon_splits_on_first() {
        let app = make_auth_app(Some("admin"), Some("pass:word"));
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:pass:word");
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "Password 'pass:word' should be accepted via split on first colon");
    }

    #[tokio::test]
    async fn test_bearer_token_rejected() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", "Bearer some-token-here")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_very_long_credentials_handled() {
        let app = make_auth_app(Some("admin"), Some("secret"));
        let long_user = "x".repeat(10_000);
        let creds = base64::engine::general_purpose::STANDARD.encode(format!("{}:password", long_user));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "Very long credentials should be rejected");
    }
}
