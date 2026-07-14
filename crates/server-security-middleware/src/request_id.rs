use axum::body::Body;
use axum::http::{Request, Response};
use axum::middleware::Next;
use uuid::Uuid;

/// Middleware that assigns a unique request ID (from `X-Request-Id` header or generated).
pub async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response<Body> {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(RequestId(request_id.clone()));

    let mut response = next.run(req).await;

    let header_value = match request_id.parse::<axum::http::HeaderValue>() {
        Ok(v) => v,
        Err(_) => {
            let fresh = Uuid::new_v4().to_string();
            response.extensions_mut().insert(RequestId(fresh.clone()));
            axum::http::HeaderValue::from_bytes(fresh.as_bytes()).expect("UUID is always valid HeaderValue")
        }
    };

    response.headers_mut().insert("x-request-id", header_value);

    response
}

/// Extracted request ID extension.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);
