use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;

/// Middleware that adds security headers (CSP, HSTS, X-Frame-Options, etc.) to responses.
pub async fn security_headers_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let is_https = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|s| s == "https")
        .unwrap_or(false);

    let mut response = next.run(req).await;

    let headers = response.headers_mut();

    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "X-XSS-Protection",
        axum::http::HeaderValue::from_static("0"),
    );
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; \
             style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
             img-src 'self' data: blob:; font-src 'self' https://fonts.gstatic.com; \
             connect-src 'self' ws: wss: https://fonts.googleapis.com https://fonts.gstatic.com; \
             frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        ),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "Permissions-Policy",
        axum::http::HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), payment=()",
        ),
    );

    if is_https {
        headers.insert(
            axum::http::header::STRICT_TRANSPORT_SECURITY,
            axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}

/// Installs a panic hook that logs panics from request handlers with the
/// request path and method.  Axum already catches panics in handlers and
/// converts them to 500 responses, so this middleware enriches panic logs
/// with request context for diagnostics.
pub async fn panic_handler_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let path = req.uri().path().to_owned();
    let method = req.method().clone();
    let response = next.run(req).await;

    // If the handler returned 500 without a structured error body, it may have
    // been a panic. Log the request context for correlation with panic logs.
    if response.status() == StatusCode::INTERNAL_SERVER_ERROR {
        tracing::error!(
            method = %method,
            path = %path,
            status = 500,
            "Internal server error (possible panic in handler)"
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    fn make_app() -> axum::Router {
        axum::Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(security_headers_middleware))
    }

    #[tokio::test]
    async fn test_security_headers_present() {
        let app = make_app();
        let resp = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let headers = resp.headers();
        assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(headers.get("X-XSS-Protection").unwrap(), "0");
        let csp = headers
            .get("Content-Security-Policy")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert_eq!(
            headers.get("Referrer-Policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            headers.get("Permissions-Policy").unwrap(),
            "camera=(), microphone=(), geolocation=(), payment=()"
        );
    }

    #[tokio::test]
    async fn test_hsts_only_on_https() {
        let app = make_app();

        let resp = app
            .clone()
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(
            resp.headers().get("Strict-Transport-Security").is_none(),
            "HSTS should not be set on HTTP requests"
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.headers().get("Strict-Transport-Security").unwrap(),
            "max-age=31536000; includeSubDomains",
            "HSTS should be set when X-Forwarded-Proto is https"
        );
    }
}
