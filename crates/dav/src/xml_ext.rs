use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

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

#[derive(Debug, Clone)]
pub struct DavResponse {
    pub href: String,
    pub propstats: Vec<PropStat>,
}

#[derive(Debug, Clone)]
pub struct PropStat {
    pub status: u16,
    pub props: Vec<DavProp>,
}

#[derive(Debug, Clone)]
pub struct DavProp {
    pub name: String,
    pub namespace: Option<String>,
    pub value: Option<String>,
}

pub fn parse_calendar_query_time_range(body: &[u8]) -> Option<(String, String)> {
    let mut start = None;
    let mut end = None;

    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
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

pub fn parse_addressbook_query_filter(body: &[u8]) -> Option<String> {
    let mut filter_text = None;

    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_prop_filter = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = name.strip_prefix("A:").unwrap_or(&name);
                if local == "prop-filter" {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        if key == "name" {
                            filter_text = Some(val);
                        }
                    }
                    in_prop_filter = true;
                }
                if in_prop_filter && local == "text-match" {
                    in_prop_filter = false;
                }
            }
            Ok(Event::End(_)) => {
                in_prop_filter = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    filter_text
}
