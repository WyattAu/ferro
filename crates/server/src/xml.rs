use bytes::Bytes;
use common::metadata::FileMetadata;
use common::webdav::{LockDepth, LockScope};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn build_multistatus_xml(items: &[(String, FileMetadata)]) -> Bytes {
    let mut writer = Writer::new(Vec::new());

    let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)));
    let mut multistatus = BytesStart::new("D:multistatus");
    multistatus.push_attribute(("xmlns:D", "DAV:"));
    let _ = writer.write_event(Event::Start(multistatus));

    for (path, meta) in items {
        let _ = writer.write_event(Event::Start(BytesStart::new("D:response")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:href")));
        let _ = writer.write_event(Event::Text(BytesText::new(path)));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:href")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:propstat")));
        let _ = writer.write_event(Event::Start(BytesStart::new("D:prop")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:getlastmodified")));
        let _ = writer.write_event(Event::Text(BytesText::new(
            &meta
                .modified_at
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string(),
        )));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:getlastmodified")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:getcontentlength")));
        let _ = writer.write_event(Event::Text(BytesText::new(&meta.size.to_string())));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:getcontentlength")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:getetag")));
        let _ = writer.write_event(Event::Text(BytesText::new(&meta.etag)));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:getetag")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:getcontenttype")));
        let _ = writer.write_event(Event::Text(BytesText::new(&meta.mime_type)));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:getcontenttype")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:resourcetype")));
        if meta.is_collection {
            let _ = writer.write_event(Event::Empty(BytesStart::new("D:collection")));
        }
        let _ = writer.write_event(Event::End(BytesEnd::new("D:resourcetype")));

        let _ = writer.write_event(Event::End(BytesEnd::new("D:prop")));

        let _ = writer.write_event(Event::Start(BytesStart::new("D:status")));
        let _ = writer.write_event(Event::Text(BytesText::new("HTTP/1.1 200 OK")));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:status")));

        let _ = writer.write_event(Event::End(BytesEnd::new("D:propstat")));
        let _ = writer.write_event(Event::End(BytesEnd::new("D:response")));
    }

    let _ = writer.write_event(Event::End(BytesEnd::new("D:multistatus")));
    Bytes::from(writer.into_inner())
}

#[derive(Debug, Clone)]
pub struct LockRequest {
    pub scope: LockScope,
    pub depth: LockDepth,
    pub owner: Option<String>,
    pub timeout_hint: Option<u32>,
}

impl Default for LockRequest {
    fn default() -> Self {
        Self {
            scope: LockScope::Exclusive,
            depth: LockDepth::Infinity,
            owner: None,
            timeout_hint: None,
        }
    }
}

impl LockRequest {
    pub fn parse(body: &[u8]) -> Self {
        if body.is_empty() {
            return Self::default();
        }

        let mut request = Self::default();
        let mut reader = quick_xml::Reader::from_reader(body);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut current_element = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local = current_element
                        .strip_prefix("D:")
                        .unwrap_or(&current_element);
                    match local {
                        "exclusive" => request.scope = LockScope::Exclusive,
                        "shared" => request.scope = LockScope::Shared,
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default();
                    let local = current_element
                        .strip_prefix("D:")
                        .unwrap_or(&current_element);
                    match local {
                        "href" => {
                            request.owner = Some(text.to_string());
                        }
                        "timeout" => {
                            if let Some(secs) = text.strip_prefix("Second-")
                                && let Ok(s) = secs.parse::<u32>()
                            {
                                request.timeout_hint = Some(s);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(_)) => {
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        request
    }
}

pub fn build_lock_response_xml(
    lock_token: &str,
    depth: &str,
    principal: &str,
    timeout_secs: u32,
    path: &str,
) -> Bytes {
    let mut writer = Writer::new(Vec::new());

    let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)));
    let mut prop = BytesStart::new("D:prop");
    prop.push_attribute(("xmlns:D", "DAV:"));
    let _ = writer.write_event(Event::Start(prop));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:lockdiscovery")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:activelock")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:locktype")));
    let _ = writer.write_event(Event::Empty(BytesStart::new("D:write")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:locktype")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:lockscope")));
    let _ = writer.write_event(Event::Empty(BytesStart::new("D:exclusive")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:lockscope")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:depth")));
    let _ = writer.write_event(Event::Text(BytesText::new(depth)));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:depth")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:owner")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:href")));
    let _ = writer.write_event(Event::Text(BytesText::new(principal)));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:href")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:owner")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:timeout")));
    let _ = writer.write_event(Event::Text(BytesText::new(&format!(
        "Second-{}",
        timeout_secs
    ))));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:timeout")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:locktoken")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:href")));
    let _ = writer.write_event(Event::Text(BytesText::new(lock_token)));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:href")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:locktoken")));

    let _ = writer.write_event(Event::Start(BytesStart::new("D:lockroot")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:href")));
    let _ = writer.write_event(Event::Text(BytesText::new(path)));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:href")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:lockroot")));

    let _ = writer.write_event(Event::End(BytesEnd::new("D:activelock")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:lockdiscovery")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:prop")));

    Bytes::from(writer.into_inner())
}

// === PROPPATCH support ===

#[derive(Debug, Clone)]
pub struct PropPatchOp {
    pub name: String,
    pub value: Option<String>,
    pub operation: PropPatchOperation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropPatchOperation {
    Set,
    Remove,
}

/// Parse a PROPPATCH request body to extract property operations.
pub fn parse_proppatch(body: &[u8]) -> Vec<PropPatchOp> {
    if body.is_empty() {
        return vec![];
    }

    let mut ops = Vec::new();
    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_element = String::new();
    let mut current_operation = PropPatchOperation::Set;
    let mut in_set = false;
    let mut in_remove = false;
    let mut in_prop = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = current_element
                    .strip_prefix("D:")
                    .unwrap_or(&current_element);
                match local {
                    "set" => {
                        in_set = true;
                        current_operation = PropPatchOperation::Set;
                    }
                    "remove" => {
                        in_remove = true;
                        current_operation = PropPatchOperation::Remove;
                    }
                    "prop" => {
                        in_prop = true;
                    }
                    _ => {
                        // For remove operations, empty elements like <D:owner/> should be captured
                        if in_prop && in_remove {
                            ops.push(PropPatchOp {
                                name: local.to_string(),
                                value: None,
                                operation: PropPatchOperation::Remove,
                            });
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_prop && in_set {
                    let text = e.unescape().unwrap_or_default();
                    let local = current_element
                        .strip_prefix("D:")
                        .unwrap_or(&current_element);
                    ops.push(PropPatchOp {
                        name: local.to_string(),
                        value: Some(text.to_string()),
                        operation: current_operation.clone(),
                    });
                } else if in_prop && in_remove {
                    let local = current_element
                        .strip_prefix("D:")
                        .unwrap_or(&current_element);
                    ops.push(PropPatchOp {
                        name: local.to_string(),
                        value: None,
                        operation: PropPatchOperation::Remove,
                    });
                }
            }
            Ok(Event::End(ref e)) => {
                let local = String::from_utf8_lossy(e.name().as_ref())
                    .strip_prefix("D:")
                    .unwrap_or("")
                    .to_string();
                match local.as_str() {
                    "set" => in_set = false,
                    "remove" => in_remove = false,
                    "prop" => in_prop = false,
                    _ => {}
                }
                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    ops
}

/// Build a PROPPATCH response XML (multistatus with propstat for each property).
pub fn build_proppatch_response(path: &str, props: &[PropPatchOp]) -> Bytes {
    let mut writer = Writer::new(Vec::new());

    let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)));
    let mut multistatus = BytesStart::new("D:multistatus");
    multistatus.push_attribute(("xmlns:D", "DAV:"));
    let _ = writer.write_event(Event::Start(multistatus));

    // Response element
    let _ = writer.write_event(Event::Start(BytesStart::new("D:response")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:href")));
    let _ = writer.write_event(Event::Text(BytesText::new(path)));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:href")));

    // Propstat: all properties succeeded (200 OK)
    let _ = writer.write_event(Event::Start(BytesStart::new("D:propstat")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:prop")));

    for prop in props {
        let local = &prop.name;
        let _ = writer.write_event(Event::Start(BytesStart::new(local.as_str())));
        if let Some(ref val) = prop.value {
            let _ = writer.write_event(Event::Text(BytesText::new(val)));
        }
        let _ = writer.write_event(Event::End(BytesEnd::new(local.as_str())));
    }

    let _ = writer.write_event(Event::End(BytesEnd::new("D:prop")));
    let _ = writer.write_event(Event::Start(BytesStart::new("D:status")));
    let _ = writer.write_event(Event::Text(BytesText::new("HTTP/1.1 200 OK")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:status")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:propstat")));

    let _ = writer.write_event(Event::End(BytesEnd::new("D:response")));
    let _ = writer.write_event(Event::End(BytesEnd::new("D:multistatus")));

    Bytes::from(writer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_request_parse_empty() {
        let req = LockRequest::parse(b"");
        assert_eq!(req.scope, LockScope::Exclusive);
        assert_eq!(req.depth, LockDepth::Infinity);
        assert!(req.owner.is_none());
    }

    #[test]
    fn test_lock_request_parse_full() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
    <D:locktype><D:write/></D:locktype>
    <D:lockscope><D:exclusive/></D:lockscope>
    <D:owner><D:href>http://example.com/~user/</D:href></D:owner>
    <D:timeout>Second-3600</D:timeout>
</D:lockinfo>
"#;
        let req = LockRequest::parse(xml);
        assert_eq!(req.scope, LockScope::Exclusive);
        assert_eq!(req.timeout_hint, Some(3600));
        assert_eq!(req.owner, Some("http://example.com/~user/".to_string()));
    }

    #[test]
    fn test_lock_request_parse_shared() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
    <D:locktype><D:write/></D:locktype>
    <D:lockscope><D:shared/></D:lockscope>
</D:lockinfo>
"#;
        let req = LockRequest::parse(xml);
        assert_eq!(req.scope, LockScope::Shared);
    }

    #[test]
    fn test_build_multistatus_xml() {
        let hash = common::metadata::ContentHash::new("a".repeat(64));
        let meta = FileMetadata::new("/test.txt".to_string(), hash, 42, "user1".to_string());
        let xml = build_multistatus_xml(&[("/test.txt".to_string(), meta)]);
        let xml_str = String::from_utf8(xml.to_vec()).unwrap();
        assert!(xml_str.contains("<D:multistatus"));
        assert!(xml_str.contains("<D:href>/test.txt</D:href>"));
        assert!(xml_str.contains("<D:getcontentlength>42</D:getcontentlength>"));
        assert!(xml_str.contains("</D:multistatus>"));
    }

    #[test]
    fn test_build_lock_response_xml() {
        let xml = build_lock_response_xml("urn:uuid:test", "infinity", "user1", 60, "/test.txt");
        let xml_str = String::from_utf8(xml.to_vec()).unwrap();
        assert!(xml_str.contains("<D:locktoken>"));
        assert!(xml_str.contains("urn:uuid:test"));
        assert!(xml_str.contains("Second-60"));
    }

    #[test]
    fn test_parse_proppatch_set() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:set>
        <D:prop>
            <D:displayname>My File</D:displayname>
        </D:prop>
    </D:set>
</D:propertyupdate>"#;

        let ops = parse_proppatch(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "displayname");
        assert_eq!(ops[0].value, Some("My File".to_string()));
        assert_eq!(ops[0].operation, PropPatchOperation::Set);
    }

    #[test]
    fn test_parse_proppatch_remove() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:remove>
        <D:prop>
            <D:owner/>
        </D:prop>
    </D:remove>
</D:propertyupdate>"#;

        let ops = parse_proppatch(body);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "owner");
        assert_eq!(ops[0].operation, PropPatchOperation::Remove);
    }

    #[test]
    fn test_parse_proppatch_empty() {
        let ops = parse_proppatch(b"");
        assert!(ops.is_empty());
    }

    #[test]
    fn test_build_proppatch_response() {
        let props = vec![PropPatchOp {
            name: "displayname".to_string(),
            value: Some("Test".to_string()),
            operation: PropPatchOperation::Set,
        }];
        let xml = build_proppatch_response("/file.txt", &props);
        let xml_str = String::from_utf8(xml.to_vec()).unwrap();
        assert!(xml_str.contains("<D:multistatus"));
        assert!(xml_str.contains("<D:href>/file.txt</D:href>"));
        assert!(xml_str.contains(">Test<"));
        assert!(xml_str.contains("HTTP/1.1 200 OK"));
    }
}
