use crate::handler::WebdavHandlerContext;
use crate::xml_util;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::FerroError;
use common::error::Result;
use common::path::normalize_path;
use tracing::debug;

pub(crate) async fn handle_proppatch<S: crate::WebdavAppState>(
    state: &S,
    path: &str,
    _headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), _headers);
    ctx.validate_path()?;

    if !state.storage().exists(&ctx.path).await? {
        return Err(FerroError::NotFound(ctx.path.to_string()));
    }

    let props = xml_util::parse_proppatch(body);
    let xml = xml_util::build_proppatch_response(&ctx.path, &props);

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );

    debug!("PROPPATCH {} ({} properties)", ctx.path, props.len());
    Ok((StatusCode::MULTI_STATUS, resp_headers, Body::from(xml)).into_response())
}
