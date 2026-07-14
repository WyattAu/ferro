mod context;
mod copy;
mod delete;
mod get;
mod lock;
mod mkcol;
mod move_cmd;
mod propfind;
mod proppatch;
mod put;

pub(crate) use context::WebdavHandlerContext;
pub(crate) use copy::handle_copy;
pub(crate) use delete::handle_delete;
pub(crate) use get::{handle_get, handle_head};
pub(crate) use lock::{handle_lock, handle_unlock};
pub(crate) use mkcol::handle_mkcol;
pub(crate) use move_cmd::handle_move;
pub(crate) use propfind::handle_propfind;
pub(crate) use proppatch::handle_proppatch;
pub(crate) use put::handle_put;

use crate::WebdavAppState;
use crate::xml_util;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;
use http_body_util::BodyExt;
use tracing::{debug, warn};

pub fn sanitize_path(path: &str) -> Result<String> {
    if path.contains('\0') {
        return Err(FerroError::InvalidArgument("Path contains null bytes".to_string()));
    }

    for component in std::path::Path::new(path).components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(FerroError::InvalidArgument(
                    "Path traversal detected: '..' not allowed".to_string(),
                ));
            }
            std::path::Component::CurDir => {
                return Err(FerroError::InvalidArgument("Path contains '.' component".to_string()));
            }
            _ => {}
        }
    }

    let normalized = normalize_path(path);
    Ok(normalized.to_string())
}

pub(crate) fn strip_uri_authority(uri: &str) -> String {
    if let Ok(parsed) = url::Url::parse(uri) {
        return parsed.path().to_string();
    }
    if uri.starts_with('/') {
        uri.to_string()
    } else if let Some(idx) = uri.find('/') {
        if idx > 0 && uri[..idx].contains("://") {
            uri[idx..].to_string()
        } else {
            uri.to_string()
        }
    } else {
        uri.to_string()
    }
}

pub(crate) fn extract_owner(headers: &HeaderMap, claims: Option<&common::auth::Claims>) -> String {
    if let Some(user) = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok()) {
        return user.to_string();
    }
    if let Some(c) = claims {
        return c.sub.clone();
    }
    "anonymous".to_string()
}

pub(crate) fn check_conditional_if_match(headers: &HeaderMap, etag: &str) -> Result<()> {
    if let Some(if_match) = headers.get("If-Match").and_then(|v| v.to_str().ok()) {
        let trimmed = if_match.trim();
        if trimmed == "*" {
        } else {
            let tags: Vec<&str> = trimmed.split(',').map(|t| t.trim()).collect();
            if !tags.contains(&etag) {
                return Err(FerroError::PreconditionFailed(format!(
                    "If-Match: expected one of {}, got {}",
                    trimmed, etag
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn check_if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok()) {
        let trimmed = if_none_match.trim();
        if trimmed == "*" || trimmed == etag {
            return true;
        }
        let tags: Vec<&str> = trimmed.split(',').map(|t| t.trim()).collect();
        if tags.contains(&etag) {
            return true;
        }
    }
    false
}

pub(crate) fn sniff_content_type(data: &[u8], path: &str) -> String {
    if let Some(mime) = mime_guess::from_path(path).first() {
        let mime_str = mime.essence_str();
        if mime_str != "application/octet-stream" {
            return mime_str.to_string();
        }
    }

    if data.len() >= 4 {
        match &data[..4] {
            b"%PDF" => return "application/pdf".to_string(),
            b"\x89PNG" => return "image/png".to_string(),
            b"GIF8" => return "image/gif".to_string(),
            _ => {}
        }
    }
    if data.len() >= 3 && &data[..3] == b"\xff\xd8\xff" {
        return "image/jpeg".to_string();
    }
    if data.len() >= 5 && &data[..5] == b"<?xml" {
        return "application/xml".to_string();
    }
    if data.len() >= 2 && &data[..2] == b"PK" {
        return "application/zip".to_string();
    }
    if data.len() >= 6 && &data[..6] == b"Rar!\x1a\x07" {
        return "application/vnd.rar".to_string();
    }
    if data.len() >= 4 && &data[..4] == b"OggS" {
        return "audio/ogg".to_string();
    }
    if data.len() >= 12 && &data[8..12] == b"WEBP" {
        return "image/webp".to_string();
    }
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return "video/mp4".to_string();
    }

    "application/octet-stream".to_string()
}

async fn handle_options(_path: &str) -> Result<Response> {
    let mut headers = HeaderMap::new();
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

pub async fn handle_any<S: WebdavAppState>(
    method: Method,
    uri: axum::http::Uri,
    State(state): State<S>,
    path: Option<AxumPath<String>>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let raw_path = match path {
        Some(AxumPath(p)) => format!("/{}", p),
        None => uri.path().to_string(),
    };

    let path_str = match sanitize_path(&raw_path) {
        Ok(p) => p,
        Err(e) => {
            warn!("Path sanitization failed for '{}': {}", raw_path, e);
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            return (
                status,
                axum::Json(serde_json::json!({
                    "error": e.to_string(),
                })),
            )
                .into_response();
        }
    };
    debug!("{} {}", method, path_str);

    if let Some(content_len) = headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        && content_len > state.max_body_size()
    {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            axum::Json(serde_json::json!({
                "error": "Request body too large",
                "size": content_len,
                "max": state.max_body_size(),
            })),
        )
            .into_response();
    }

    let user_sub = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok());
    let resolved_path = match user_sub {
        Some(sub) if sub != "anonymous" => {
            let user_root = format!("/users/{}", sub);
            if path_str == "/" || path_str.is_empty() {
                user_root
            } else {
                format!("{}{}", user_root, path_str)
            }
        }
        _ => path_str.clone(),
    };

    let result: Result<Response> = async {
        if method.as_str() == "PUT"
            && let Some(content_len) = headers
                .get("Content-Length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
            && let Some(quota_resp) = state.enforce_quota(content_len)
        {
            return Ok(quota_resp);
        }

        match method.as_str() {
            "OPTIONS" => handle_options(&resolved_path).await,
            "PROPFIND" => handle_propfind(&state, &resolved_path, &headers).await,
            "GET" => handle_get(&state, &resolved_path, &headers).await,
            "HEAD" => handle_head(&state, &resolved_path, &headers).await,
            "PUT" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_put(&state, &resolved_path, &headers, bytes).await
            }
            "DELETE" => handle_delete(&state, &resolved_path, &headers).await,
            "MKCOL" => handle_mkcol(&state, &resolved_path).await,
            "COPY" => handle_copy(&state, &resolved_path, &headers).await,
            "MOVE" => handle_move(&state, &resolved_path, &headers).await,
            "LOCK" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_lock(&state, &resolved_path, &headers, &bytes).await
            }
            "UNLOCK" => handle_unlock(&state, &resolved_path, &headers).await,
            "PROPPATCH" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_proppatch(&state, &resolved_path, &headers, &bytes).await
            }
            "MKCALENDAR" | "REPORT" if resolved_path.starts_with("/dav/cal") => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                let m = method.clone();
                Ok(state.dispatch_caldav(&m, &resolved_path, bytes).await)
            }
            "REPORT" if resolved_path.starts_with("/dav/card") => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                let m = method.clone();
                Ok(state.dispatch_carddav(&m, &resolved_path, bytes).await)
            }
            _ => Err(FerroError::InvalidArgument(format!("Method {} not supported", method))),
        }
    }
    .await;

    match result {
        Ok(response) => response,
        Err(e) => {
            warn!("Error handling {} {}: {}", method, path_str, e);
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let xml = format!(
                r#"<?xml version="1.0" encoding="utf-8"?><d:error xmlns:d="DAV:"><s:message>{}</s:message></d:error>"#,
                xml_util::escape_xml(&e.to_string())
            );
            (status, xml).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_dotdot_rejected() {
        assert!(sanitize_path("/foo/../bar").is_err());
        assert!(sanitize_path("/../etc/passwd").is_err());
    }

    #[test]
    fn test_path_with_null_byte_rejected() {
        assert!(sanitize_path("/foo\0bar").is_err());
    }

    #[test]
    fn test_path_normalized_correctly() {
        assert_eq!(sanitize_path("/foo/bar").unwrap(), "/foo/bar");
        assert_eq!(sanitize_path("/foo//bar").unwrap(), "/foo/bar");
        assert_eq!(sanitize_path("/foo/bar/").unwrap(), "/foo/bar");
        assert_eq!(sanitize_path("/").unwrap(), "/");
    }

    #[test]
    fn test_sniff_content_type() {
        assert_eq!(sniff_content_type(&[], "test.txt"), "text/plain");
        assert_eq!(sniff_content_type(b"%PDF-1.4", "doc.pdf"), "application/pdf");
    }

    #[test]
    fn test_strip_uri_authority() {
        assert_eq!(
            strip_uri_authority("http://localhost:8080/path/to/resource"),
            "/path/to/resource"
        );
        assert_eq!(strip_uri_authority("/already/a/path"), "/already/a/path");
    }

    #[test]
    fn test_extract_owner_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Ferro-User", "user1".parse().unwrap());
        assert_eq!(extract_owner(&headers, None), "user1");
    }

    #[test]
    fn test_extract_owner_from_claims() {
        let headers = HeaderMap::new();
        let claims = common::auth::Claims {
            sub: "user2".to_string(),
            aud: "ferro".to_string(),
            iss: "ferro".to_string(),
            exp: 0,
            iat: 0,
            nonce: None,
            email: None,
            name: None,
            groups: None,
        };
        assert_eq!(extract_owner(&headers, Some(&claims)), "user2");
    }

    #[test]
    fn test_extract_owner_anonymous() {
        let headers = HeaderMap::new();
        assert_eq!(extract_owner(&headers, None), "anonymous");
    }

    #[test]
    fn test_check_conditional_if_match_no_header() {
        let headers = HeaderMap::new();
        assert!(check_conditional_if_match(&headers, "etag").is_ok());
    }

    #[test]
    fn test_check_conditional_if_match_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert("If-Match", "*".parse().unwrap());
        assert!(check_conditional_if_match(&headers, "etag").is_ok());
    }

    #[test]
    fn test_check_conditional_if_match_match() {
        let mut headers = HeaderMap::new();
        headers.insert("If-Match", "\"abc123\"".parse().unwrap());
        assert!(check_conditional_if_match(&headers, "\"abc123\"").is_ok());
    }

    #[test]
    fn test_check_conditional_if_match_no_match() {
        let mut headers = HeaderMap::new();
        headers.insert("If-Match", "\"abc123\"".parse().unwrap());
        assert!(check_conditional_if_match(&headers, "\"def456\"").is_err());
    }

    #[test]
    fn test_check_conditional_if_match_multiple_tags() {
        let mut headers = HeaderMap::new();
        headers.insert("If-Match", "\"abc123\", \"def456\"".parse().unwrap());
        assert!(check_conditional_if_match(&headers, "\"abc123\"").is_ok());
        assert!(check_conditional_if_match(&headers, "\"def456\"").is_ok());
        assert!(check_conditional_if_match(&headers, "\"ghi789\"").is_err());
    }

    #[test]
    fn test_check_if_none_match_no_header() {
        let headers = HeaderMap::new();
        assert!(!check_if_none_match(&headers, "etag"));
    }

    #[test]
    fn test_check_if_none_match_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "*".parse().unwrap());
        assert!(check_if_none_match(&headers, "etag"));
    }

    #[test]
    fn test_check_if_none_match_match() {
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "\"abc123\"".parse().unwrap());
        assert!(check_if_none_match(&headers, "\"abc123\""));
    }

    #[test]
    fn test_check_if_none_match_no_match() {
        let mut headers = HeaderMap::new();
        headers.insert("If-None-Match", "\"abc123\"".parse().unwrap());
        assert!(!check_if_none_match(&headers, "\"def456\""));
    }

    #[test]
    fn test_sniff_content_type_pdf() {
        assert_eq!(sniff_content_type(b"%PDF-1.4", "doc.pdf"), "application/pdf");
    }

    #[test]
    fn test_sniff_content_type_png() {
        let mut data = vec![0x89, 0x50, 0x4E, 0x47];
        data.extend(vec![0; 100]);
        assert_eq!(sniff_content_type(&data, "image.png"), "image/png");
    }

    #[test]
    fn test_sniff_content_type_gif() {
        assert_eq!(sniff_content_type(b"GIF89a", "image.gif"), "image/gif");
    }

    #[test]
    fn test_sniff_content_type_jpeg() {
        assert_eq!(sniff_content_type(b"\xff\xd8\xff", "image.jpg"), "image/jpeg");
    }

    #[test]
    fn test_sniff_content_type_xml() {
        assert_eq!(sniff_content_type(b"<?xml", "data.xml"), "text/xml");
    }

    #[test]
    fn test_sniff_content_type_zip() {
        assert_eq!(sniff_content_type(b"PK", "archive.zip"), "application/zip");
    }

    #[test]
    fn test_sniff_content_type_rar() {
        assert_eq!(
            sniff_content_type(b"Rar!\x1a\x07", "archive.rar"),
            "application/x-rar-compressed"
        );
    }

    #[test]
    fn test_sniff_content_type_ogg() {
        assert_eq!(sniff_content_type(b"OggS", "audio.ogg"), "audio/ogg");
    }

    #[test]
    fn test_sniff_content_type_webp() {
        let mut data = vec![0, 0, 0, 0, 0, 0, 0, 0, b'W', b'E', b'B', b'P'];
        data.extend(vec![0; 100]);
        assert_eq!(sniff_content_type(&data, "image.webp"), "image/webp");
    }

    #[test]
    fn test_sniff_content_type_mp4() {
        let mut data = vec![0, 0, 0, 0, b'f', b't', b'y', b'p'];
        data.extend(vec![0; 100]);
        assert_eq!(sniff_content_type(&data, "video.mp4"), "video/mp4");
    }

    #[test]
    fn test_sniff_content_type_octet() {
        assert_eq!(sniff_content_type(b"unknown", "data.bin"), "application/octet-stream");
    }

    #[test]
    fn test_strip_uri_authority_no_slash() {
        assert_eq!(strip_uri_authority("http://example.com"), "/");
    }

    #[test]
    fn test_strip_uri_authority_relative() {
        assert_eq!(strip_uri_authority("relative/path"), "relative/path");
    }

    #[tokio::test]
    async fn test_handle_options() {
        let response = handle_options("/").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
