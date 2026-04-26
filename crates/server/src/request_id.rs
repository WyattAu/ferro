use axum::body::Body;
use axum::http::{Request, Response};
use axum::middleware::Next;
use uuid::Uuid;

pub async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response<Body> {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(RequestId(request_id.clone()));

    let mut response = next.run(req).await;

    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );

    response
}

#[derive(Debug, Clone)]
pub struct RequestId(pub String);
