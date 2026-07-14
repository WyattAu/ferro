use common::simd::compare::contains_simd;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::borrow::Cow;

/// Escape special XML characters in a string.
/// Returns a borrowed reference if no escaping is needed (zero-copy).
#[must_use]
pub fn escape_xml(s: &str) -> Cow<'_, str> {
    if !needs_escaping(s) {
        return Cow::Borrowed(s);
    }
    let mut result = String::with_capacity(s.len() + s.len() / 4);
    for ch in s.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(ch),
        }
    }
    Cow::Owned(result)
}

/// Check if a string contains characters that need XML escaping.
/// Uses SIMD acceleration on x86_64 for large ASCII strings.
#[must_use]
fn needs_escaping(s: &str) -> bool {
    // For ASCII-only strings, use SIMD-accelerated byte search
    if s.is_ascii() {
        #[cfg(target_arch = "x86_64")]
        {
            contains_simd(s, "&")
                || contains_simd(s, "<")
                || contains_simd(s, ">")
                || contains_simd(s, "\"")
                || contains_simd(s, "'")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            s.contains('&') || s.contains('<') || s.contains('>') || s.contains('"') || s.contains('\'')
        }
    } else {
        // For non-ASCII strings, use standard library (handles UTF-8 correctly)
        s.contains('&') || s.contains('<') || s.contains('>') || s.contains('"') || s.contains('\'')
    }
}

/// Build a `WebDAV` multistatus XML response body.
#[must_use]
pub fn build_dav_multistatus(responses: &[DavResponse]) -> Vec<u8> {
    let mut writer = Writer::new(Vec::new());

    let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)));
    let mut ms = BytesStart::new("D:multistatus");
    ms.push_attribute(("xmlns:D", "DAV:"));
    ms.push_attribute(("xmlns:C", "urn:ietf:params:xml:ns:caldav"));
    ms.push_attribute(("xmlns:A", "urn:ietf:params:xml:ns:carddav"));
    let _ = writer.write_event(Event::Start(ms));

    for resp in responses {
        let _ = writer.write_event(Event::Start(BytesStart::new("D:response")));
        write_text(&mut writer, "D:href", &resp.href);

        for propstat in &resp.propstats {
            let _ = writer.write_event(Event::Start(BytesStart::new("D:propstat")));
            let _ = writer.write_event(Event::Start(BytesStart::new("D:prop")));

            for prop in &propstat.props {
                let tag = if let Some(ref ns) = prop.namespace {
                    format!("<{} xmlns=\"{}\">", prop.name, ns)
                } else {
                    format!("<{}>", prop.name)
                };
                let _ = writer.write_event(Event::Start(BytesStart::new(&tag)));
                if let Some(ref val) = prop.value {
                    let _ = writer.write_event(Event::Text(BytesText::new(val)));
                }
                let _ = writer.write_event(Event::End(BytesEnd::new(prop.name.as_str())));
            }

            let _ = writer.write_event(Event::End(BytesEnd::new("D:prop")));
            write_text(
                &mut writer,
                "D:status",
                &format!("HTTP/1.1 {} {}", propstat.status, status_text(propstat.status)),
            );
            let _ = writer.write_event(Event::End(BytesEnd::new("D:propstat")));
        }

        let _ = writer.write_event(Event::End(BytesEnd::new("D:response")));
    }

    let _ = writer.write_event(Event::End(BytesEnd::new("D:multistatus")));
    writer.into_inner()
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        404 => "Not Found",
        _ => "Unknown",
    }
}

fn write_text(writer: &mut Writer<Vec<u8>>, tag: &str, text: &str) {
    let _ = writer.write_event(Event::Start(BytesStart::new(tag)));
    let _ = writer.write_event(Event::Text(BytesText::new(text)));
    let _ = writer.write_event(Event::End(BytesEnd::new(tag)));
}

/// A single `WebDAV` response element with href and property statuses.
#[derive(Debug, Clone)]
pub struct DavResponse {
    /// Resource href.
    pub href: String,
    /// Property status groups.
    pub propstats: Vec<PropStat>,
}

/// A `WebDAV` propstat element containing status code and properties.
#[derive(Debug, Clone)]
pub struct PropStat {
    /// HTTP status code (e.g. 200, 404).
    pub status: u16,
    /// Properties with their status.
    pub props: Vec<DavProp>,
}

/// A single `WebDAV` property element.
#[derive(Debug, Clone)]
pub struct DavProp {
    /// Property name (possibly namespace-prefixed, e.g. "D:getetag").
    pub name: String,
    /// Optional XML namespace URI.
    pub namespace: Option<String>,
    /// Property value content (for leaf properties).
    pub value: Option<String>,
}

/// Parse a `CalDAV` calendar-query time-range filter from an XML request body.
#[must_use]
pub fn parse_calendar_query_time_range(body: &[u8]) -> Option<(String, String)> {
    if body.len() > 10 * 1024 * 1024 {
        return None;
    }

    let mut start = None;
    let mut end = None;

    let mut reader = quick_xml::Reader::from_reader(body);
    // quick-xml 0.37 does NOT expand entities by default (safe).
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = name.strip_prefix("C:").unwrap_or(&name);
                if local == "time-range" {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if key == "start" {
                            start = Some(val);
                        } else if key == "end" {
                            end = Some(val);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    match (start, end) {
        (Some(s), Some(e)) => Some((s, e)),
        _ => None,
    }
}

/// Parse a `CardDAV` addressbook-query text-match filter from an XML request body.
/// Returns the text to match against if a `<text-match>` element is found.
#[must_use]
pub fn parse_addressbook_query_filter(body: &[u8]) -> Option<String> {
    if body.len() > 10 * 1024 * 1024 {
        return None;
    }

    let mut filter_text = None;

    let mut reader = quick_xml::Reader::from_reader(body);
    // quick-xml 0.37 does NOT expand entities by default (safe).
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_text_match = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = name.strip_prefix("A:").unwrap_or(&name);
                let local = local.strip_prefix("C:").unwrap_or(local);
                if local == "text-match" {
                    in_text_match = true;
                }
            }
            Ok(Event::Text(ref e)) if in_text_match && filter_text.is_none() => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if !text.is_empty() {
                    filter_text = Some(text);
                }
            }
            Ok(Event::End(_)) => {
                in_text_match = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    filter_text
}

/// Parsed sync-collection request containing the sync-token and requested properties.
#[derive(Debug, Clone)]
pub struct SyncCollectionRequest {
    /// The sync-token value from the client, or None for a full resync.
    pub sync_token: Option<String>,
    /// Whether the client requested `getetag`.
    pub want_getetag: bool,
    /// Whether the client requested `calendar-data` (`CalDAV`).
    pub want_calendar_data: bool,
    /// Whether the client requested `address-data` (`CardDAV`).
    pub want_address_data: bool,
}

/// Parse a sync-collection REPORT request body.
/// Extracts the `<sync-token>` value and the requested properties from `<prop>`.
pub fn parse_sync_collection(body: &[u8]) -> Result<SyncCollectionRequest, String> {
    if body.len() > 10 * 1024 * 1024 {
        return Err("Request body too large".to_string());
    }

    let mut sync_token = None;
    let mut want_getetag = false;
    let mut want_calendar_data = false;
    let mut want_address_data = false;
    let mut in_sync_token = false;
    let mut in_prop = false;

    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                // Strip any namespace prefix (D:, C:, A:, or no prefix)
                let local = name
                    .strip_prefix("D:")
                    .or_else(|| name.strip_prefix("C:"))
                    .or_else(|| name.strip_prefix("A:"))
                    .unwrap_or(&name);

                match local {
                    "sync-token" => in_sync_token = true,
                    "prop" => in_prop = true,
                    "getetag" if in_prop => want_getetag = true,
                    "calendar-data" if in_prop => want_calendar_data = true,
                    "address-data" if in_prop => want_address_data = true,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_sync_token && sync_token.is_none() => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    sync_token = Some(trimmed);
                }
            }
            Ok(Event::End(_)) => {
                in_sync_token = false;
                in_prop = false;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }

    Ok(SyncCollectionRequest {
        sync_token,
        want_getetag,
        want_calendar_data,
        want_address_data,
    })
}

/// Parse a `CalDAV` calendar-multiget or `CardDAV` addressbook-multiget request.
/// Extracts the list of hrefs from `<D:href>` elements inside the report body.
#[must_use]
pub fn parse_multiget_hrefs(body: &[u8]) -> Vec<String> {
    let mut hrefs = Vec::new();

    if body.len() > 10 * 1024 * 1024 {
        return hrefs;
    }

    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_href = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = name.strip_prefix("D:").unwrap_or(&name);
                if local == "href" {
                    in_href = true;
                }
            }
            Ok(Event::Text(ref e)) if in_href => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    hrefs.push(trimmed);
                }
                in_href = false;
            }
            Ok(Event::End(_)) => {
                in_href = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    hrefs
}
