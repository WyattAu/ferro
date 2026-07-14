use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::Result;

pub(crate) async fn handle_options(_path: &str) -> Result<Response> {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("DAV", HeaderValue::from_static("1, 2, 3"));
    headers.insert(
        "Allow",
        HeaderValue::from_static(
            "OPTIONS, GET, HEAD, PUT, DELETE, MKCOL, COPY, MOVE, PROPFIND, PROPPATCH, LOCK, UNLOCK",
        ),
    );
    headers.insert("MS-Author-Via", HeaderValue::from_static("DAV"));
    Ok((StatusCode::OK, headers, "").into_response())
}
