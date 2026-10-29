use crate::WebDavCoreState;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::{FerroError, Result};
use common::path::normalize_path;
use tracing::debug;

/// PROPPATCH — modify dead properties on a resource.
/// Currently supports setting `displayname` and `owner` via simple XML parsing.
pub(crate) async fn handle_proppatch<S: WebDavCoreState>(
    state: S,
    path: &str,
    _headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    if !state.storage().exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    // Token-aware lock check (RFC 4918 §7.1): PROPPATCH on a locked
    // resource requires the covering lock's token (litmus owner_modify).
    let submitted: Vec<String> = _headers
        .get("If")
        .and_then(|v| v.to_str().ok())
        .and_then(crate::ifheader::parse)
        .map(|h| h.submitted_tokens())
        .unwrap_or_default();
    state
        .lock_manager()
        .check_lock_for_write_with_token(&path, &submitted)
        .await?;

    // Parse simple PROPPATCH body to extract property operations
    let props = ferro_webdav_handler::parse_proppatch(body);

    // Dead properties (any non-DAV: namespace) are persisted; DAV:-
    // namespace properties are live/protected and rejected (403).
    let mut dead = crate::handlers::deadprops::load(&state, &path).await;
    let mut dead_changed = false;
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    // Canonical (Clark-notation) keys make stored props independent of the
    // client's prefix choice; the 207 echoes the request's raw name.
    let canonical_names = crate::handlers::deadprops::canonical_prop_names(body);
    for (idx, prop) in props.iter().enumerate() {
        if prop.name.starts_with("D:") || prop.name.starts_with("d:") {
            rejected.push(prop);
            continue;
        }
        if let Some(pairs) = &canonical_names
            && let Some((_, canonical)) = pairs.get(idx)
        {
            match prop.operation {
                ferro_webdav_handler::PropPatchOperation::Set => {
                    if let Some(ref val) = prop.value {
                        dead.insert(canonical.clone(), val.clone());
                    }
                }
                ferro_webdav_handler::PropPatchOperation::Remove => {
                    dead.remove(canonical);
                }
            }
            dead_changed = true;
            accepted.push(prop);
        }
    }
    if dead_changed {
        crate::handlers::deadprops::save(&state, &path, &dead).await?;
    }

    let mut writer = quick_xml::Writer::new(Vec::new());

    macro_rules! xml_write {
        ($event:expr) => {
            writer
                .write_event($event)
                .map_err(|e| FerroError::XmlError(e.to_string()))?
        };
    }

    xml_write!(quick_xml::events::Event::Decl(quick_xml::events::BytesDecl::new(
        "1.0",
        Some("utf-8"),
        None
    ),));
    let mut multistatus = quick_xml::events::BytesStart::new("D:multistatus");
    multistatus.push_attribute(("xmlns:D", "DAV:"));
    if let Some(pairs) = &canonical_names {
        for (attr, uri) in crate::handlers::deadprops::xmlns_attrs_for(pairs) {
            multistatus.push_attribute((attr.as_str(), uri.as_str()));
        }
    }
    xml_write!(quick_xml::events::Event::Start(multistatus));

    xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
        "D:response"
    ),));
    xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
        "D:href"
    ),));
    xml_write!(quick_xml::events::Event::Text(quick_xml::events::BytesText::new(&path),));
    xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
        "D:href"
    ),));

    // 200 OK block for accepted properties
    if !accepted.is_empty() {
        xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
            "D:propstat"
        ),));
        xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
            "D:prop"
        ),));
        for prop in &accepted {
            xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
                prop.name.as_str()
            ),));
            if let Some(ref val) = prop.value {
                xml_write!(quick_xml::events::Event::Text(quick_xml::events::BytesText::new(val),));
            }
            xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
                prop.name.as_str()
            ),));
        }
        xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
            "D:prop"
        ),));
        xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
            "D:status"
        ),));
        xml_write!(quick_xml::events::Event::Text(quick_xml::events::BytesText::new(
            "HTTP/1.1 200 OK"
        ),));
        xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
            "D:status"
        ),));
        xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
            "D:propstat"
        ),));
    }

    // 403 Forbidden block for rejected properties
    if !rejected.is_empty() {
        xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
            "D:propstat"
        ),));
        xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
            "D:prop"
        ),));
        for prop in &rejected {
            xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
                prop.name.as_str()
            ),));
            xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
                prop.name.as_str()
            ),));
        }
        xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
            "D:prop"
        ),));
        xml_write!(quick_xml::events::Event::Start(quick_xml::events::BytesStart::new(
            "D:status"
        ),));
        xml_write!(quick_xml::events::Event::Text(quick_xml::events::BytesText::new(
            "HTTP/1.1 403 Forbidden"
        ),));
        xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
            "D:status"
        ),));
        xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
            "D:propstat"
        ),));
    }

    xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
        "D:response"
    ),));
    xml_write!(quick_xml::events::Event::End(quick_xml::events::BytesEnd::new(
        "D:multistatus"
    ),));

    let xml = Bytes::from(writer.into_inner());

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );

    debug!("PROPPATCH {} ({} properties)", path, props.len());
    Ok((StatusCode::MULTI_STATUS, resp_headers, Body::from(xml)).into_response())
}
