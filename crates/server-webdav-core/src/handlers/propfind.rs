use crate::WebDavCoreState;
use crate::webdav::MAX_PROPFIND_DEPTH;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::{FerroError, Result};
use common::path::normalize_path;

pub(crate) async fn handle_propfind<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Response> {
    let path = normalize_path(path);

    // RFC 4918 §9.1.2: a malformed PROPFIND request body MUST be 400 —
    // non-well-formed XML or undeclared namespace prefixes (previously the
    // body was ignored and the request succeeded with 207).
    if !body.is_empty() {
        validate_propfind_xml(body)
            .map_err(|_| FerroError::InvalidArgument("Malformed PROPFIND request body".to_string()))?;
    }

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    let sync_token = headers
        .get("Sync-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|t| t.rsplit('/').next())
        .and_then(|n| n.parse::<u64>().ok());

    let depth = headers.get("Depth").and_then(|v| v.to_str().ok()).unwrap_or("infinity");

    // Try to head the path. If it's not found but it's a depth>0 request
    // on "/", synthesize a root collection entry (the in-memory store doesn't
    // auto-create the root collection).
    let metadata = match state.storage().head(&path).await {
        Ok(m) => m,
        Err(_) if path == "/" && depth != "0" => {
            common::metadata::FileMetadata::new_collection("/".to_string(), "anonymous".to_string())
        }
        Err(e) => return Err(e),
    };
    let mut items: Vec<(String, common::metadata::FileMetadata)> = vec![(path.to_string(), metadata)];

    if depth != "0" && items[0].1.is_collection {
        if depth == "1" {
            let children = state.storage().list(&path).await?;
            items.extend(children.into_iter().map(|m| (m.path.clone(), m)));
        } else {
            // depth:infinity — use bounded list_all
            let all_descendants = state.storage().list_all(&path, MAX_PROPFIND_DEPTH).await?;
            items.extend(all_descendants.into_iter().map(|m| (m.path.clone(), m)));
        }
    }

    if let Some(token) = sync_token {
        let current = state.sync_clock().load(std::sync::atomic::Ordering::SeqCst);
        if token >= current {
            items = items.into_iter().take(1).collect();
        }
    }

    let current_clock = state.sync_clock().load(std::sync::atomic::Ordering::SeqCst);
    let xml = ferro_webdav_handler::build_multistatus_xml(&items);
    if sync_token.is_some() {
        Ok(sync_token_multistatus_response(xml, current_clock))
    } else {
        Ok(multistatus_response(xml))
    }
}

fn multistatus_response(xml: Bytes) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response()
}

fn sync_token_multistatus_response(xml: Bytes, clock: u64) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    let token_value = format!("http://ferro.local/sync/token/{}", clock);
    if let Ok(val) = HeaderValue::from_str(&token_value) {
        headers.insert("Sync-Token", val);
    }
    (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response()
}

/// Well-formedness + namespace-prefix validation for PROPFIND bodies.
fn validate_propfind_xml(body: &[u8]) -> std::result::Result<(), String> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(body);
    let mut declared: Vec<String> = vec!["xml".to_string(), "D".to_string(), "d".to_string()];
    let mut stack: Vec<Vec<String>> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(|e| e.to_string())? {
            Event::Start(e) => {
                let mut local = declared.clone();
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if let Some((prefix, _)) = name.split_once(':')
                    && !local.iter().any(|d| d == prefix)
                {
                    return Err("undeclared namespace prefix".to_string());
                }
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                    if let Some(pfx) = key.strip_prefix("xmlns:") {
                        local.push(pfx.to_string());
                    } else if key != "xmlns"
                        && let Some((prefix, _)) = key.split_once(':')
                        && !local.iter().any(|d| d == prefix)
                    {
                        return Err("undeclared namespace prefix".to_string());
                    }
                }
                declared = local.clone();
                stack.push(local);
            }
            Event::Empty(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if let Some((prefix, _)) = name.split_once(':')
                    && !declared.iter().any(|d| d == prefix)
                {
                    return Err("undeclared namespace prefix".to_string());
                }
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                    if let Some(pfx) = key.strip_prefix("xmlns:") {
                        declared.push(pfx.to_string());
                    } else if key != "xmlns"
                        && let Some((prefix, _)) = key.split_once(':')
                        && !declared.iter().any(|d| d == prefix)
                    {
                        return Err("undeclared namespace prefix".to_string());
                    }
                }
            }
            Event::End(_) => {
                if let Some(popped) = stack.pop() {
                    declared = popped;
                }
            }
            Event::Eof => {
                // Well-formedness: every Start must have been closed.
                if !stack.is_empty() {
                    return Err("unclosed element at end of body".to_string());
                }
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}
