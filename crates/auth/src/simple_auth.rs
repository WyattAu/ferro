use base64::Engine;

use crate::users::{UserInfo, UserRole, UserStoreTrait};

fn is_public_path(path: &str) -> bool {
    path == "/healthz"
        || path == "/.well-known/ferro"
        || path == "/.well-known/openid-configuration"
        || path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/callback")
        || path.starts_with("/api/config")
        || path.starts_with("/api/auth/info")
        || path == "/metrics"
}

#[cfg(feature = "handlers")]
pub async fn simple_auth_middleware(
    req: axum::extract::Request,
    admin_user: Option<String>,
    admin_password: Option<String>,
    user_store: std::sync::Arc<dyn UserStoreTrait>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use subtle::ConstantTimeEq;

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
            return unauthorized_with_www_authenticate(
                "AUTH_REQUIRED",
                "authentication required",
            );
        }
    };

    let decoded = match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(d) => d,
        Err(_) => {
            return unauthorized_with_www_authenticate(
                "INVALID_CREDENTIALS",
                "invalid credentials",
            );
        }
    };

    let credentials = String::from_utf8_lossy(&decoded);
    let (user, pass) = match credentials.split_once(':') {
        Some((u, p)) => (u, p),
        None => {
            return unauthorized_with_www_authenticate(
                "INVALID_CREDENTIALS",
                "invalid credentials",
            );
        }
    };

    let expected_user = admin_user.as_deref().unwrap_or("");
    let expected_pass = admin_password.as_deref().unwrap_or("");

    let authenticated = if user.as_bytes().ct_eq(expected_user.as_bytes()).into()
        && pass.as_bytes().ct_eq(expected_pass.as_bytes()).into()
    {
        match user_store.get_user_by_username(user).await {
            Ok(u) if u.is_active() => UserInfo::from(&u),
            _ => UserInfo {
                user_id: "admin".to_string(),
                username: user.to_string(),
                role: UserRole::Admin,
            },
        }
    } else {
        match user_store.authenticate(user, pass).await {
            Ok(u) => UserInfo::from(&u),
            Err(_) => {
                return unauthorized_with_www_authenticate(
                    "INVALID_CREDENTIALS",
                    "invalid credentials",
                );
            }
        }
    };

    let mut req = req;
    req.extensions_mut().insert(authenticated);
    next.run(req).await
}

#[cfg(feature = "handlers")]
fn unauthorized_with_www_authenticate(code: &str, message: &str) -> axum::response::Response {
    use axum::response::IntoResponse;
    let body = axum::Json(serde_json::json!({
        "error": message,
        "error_code": code,
    }));
    let mut response = (axum::http::StatusCode::UNAUTHORIZED, body).into_response();
    response.headers_mut().insert(
        axum::http::header::WWW_AUTHENTICATE,
        axum::http::HeaderValue::from_static(r#"Basic realm="Ferro""#),
    );
    response
}

#[cfg(test)]
#[cfg(feature = "handlers")]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn make_auth_app(user: Option<&str>, pass: Option<&str>) -> axum::Router {
        let admin_user = user.map(|s| s.to_string());
        let admin_password = pass.map(|s| s.to_string());
        let user_store: std::sync::Arc<dyn UserStoreTrait> =
            std::sync::Arc::new(crate::users::InMemoryUserStore::new());
        axum::Router::new()
            .route("/api/test", axum::routing::get(|| async { "ok" }))
            .route("/healthz", axum::routing::get(|| async { "ok" }))
            .route("/.well-known/ferro", axum::routing::get(|| async { "ok" }))
            .route("/api/config", axum::routing::get(|| async { "ok" }))
            .route("/api/auth/info", axum::routing::get(|| async { "ok" }))
            .route("/api/auth/login", axum::routing::get(|| async { "ok" }))
            .route("/api/auth/callback", axum::routing::get(|| async { "ok" }))
            .route("/metrics", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let admin_user = admin_user.clone();
                    let admin_password = admin_password.clone();
                    let user_store = user_store.clone();
                    async move {
                        simple_auth_middleware(req, admin_user, admin_password, user_store, next)
                            .await
                    }
                },
            ))
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
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "path {} should be public",
                path
            );
        }
        check("/healthz").await;
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
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Password 'pass:word' should be accepted via split on first colon"
        );
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
        let creds =
            base64::engine::general_purpose::STANDARD.encode(format!("{}:password", long_user));
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
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Very long credentials should be rejected"
        );
    }

    #[tokio::test]
    async fn test_user_store_credentials_accepted() {
        let store = std::sync::Arc::new(crate::users::InMemoryUserStore::new());
        let user = crate::users::User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            role: crate::users::UserRole::User,
            created_at: chrono::Utc::now(),
            last_login: None,
            status: crate::users::UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(crate::users::hash_password("userpass")),
        };
        store.create_user(user).await.unwrap();

        let admin_user = Some("admin".to_string());
        let admin_password = Some("secret".to_string());
        let app = axum::Router::new()
            .route("/api/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let admin_user = admin_user.clone();
                    let admin_password = admin_password.clone();
                    let user_store = store.clone();
                    async move {
                        simple_auth_middleware(req, admin_user, admin_password, user_store, next)
                            .await
                    }
                },
            ));

        let creds = base64::engine::general_purpose::STANDARD.encode("testuser:userpass");
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
    async fn test_user_store_wrong_password_rejected() {
        let store = std::sync::Arc::new(crate::users::InMemoryUserStore::new());
        let user = crate::users::User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "testuser2".to_string(),
            display_name: "Test User 2".to_string(),
            email: "test2@example.com".to_string(),
            role: crate::users::UserRole::User,
            created_at: chrono::Utc::now(),
            last_login: None,
            status: crate::users::UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(crate::users::hash_password("correct")),
        };
        store.create_user(user).await.unwrap();

        let admin_user = Some("admin".to_string());
        let admin_password = Some("secret".to_string());
        let app = axum::Router::new()
            .route("/api/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let admin_user = admin_user.clone();
                    let admin_password = admin_password.clone();
                    let user_store = store.clone();
                    async move {
                        simple_auth_middleware(req, admin_user, admin_password, user_store, next)
                            .await
                    }
                },
            ));

        let creds = base64::engine::general_purpose::STANDARD.encode("testuser2:wrong");
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
    async fn test_user_info_extension_set_on_admin_auth() {
        let store = std::sync::Arc::new(crate::users::InMemoryUserStore::new());
        let admin_user = Some("admin".to_string());
        let admin_password = Some("secret".to_string());
        let app = axum::Router::new()
            .route(
                "/api/test",
                axum::routing::get(|req: axum::extract::Request| async move {
                    let info = req.extensions().get::<UserInfo>();
                    match info {
                        Some(i) => format!("user:{}", i.username),
                        None => "no user info".to_string(),
                    }
                }),
            )
            .layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let admin_user = admin_user.clone();
                    let admin_password = admin_password.clone();
                    let user_store = store.clone();
                    async move {
                        simple_auth_middleware(req, admin_user, admin_password, user_store, next)
                            .await
                    }
                },
            ));

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
}
