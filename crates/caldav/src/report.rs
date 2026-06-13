use quick_xml::Reader;
use quick_xml::events::Event;

use crate::calendar::{Calendar, CalendarItem};
use crate::error::{CalDavError, Result};

const MAX_XML_BODY_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
pub enum ReportType {
    CalendarQuery {
        time_range_start: Option<String>,
        time_range_end: Option<String>,
    },
    CalendarMultiget {
        hrefs: Vec<String>,
    },
}

pub fn parse_report(body: &[u8]) -> Result<ReportType> {
    if body.len() > MAX_XML_BODY_SIZE {
        return Err(CalDavError::XmlError("Request body too large".to_string()));
    }

    if is_multiget_request(body) {
        let hrefs = parse_multiget_hrefs(body);
        Ok(ReportType::CalendarMultiget { hrefs })
    } else {
        let (start, end) = parse_time_range(body);
        Ok(ReportType::CalendarQuery {
            time_range_start: start,
            time_range_end: end,
        })
    }
}

fn is_multiget_request(body: &[u8]) -> bool {
    let body_str = String::from_utf8_lossy(body);
    body_str.contains("calendar-multiget")
}

fn parse_time_range(body: &[u8]) -> (Option<String>, Option<String>) {
    let mut start = None;
    let mut end = None;

    if body.len() > MAX_XML_BODY_SIZE {
        return (start, end);
    }

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
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

    (start, end)
}

fn parse_multiget_hrefs(body: &[u8]) -> Vec<String> {
    let mut hrefs = Vec::new();

    if body.len() > MAX_XML_BODY_SIZE {
        return hrefs;
    }

    let mut reader = Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_href = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
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

pub fn build_report_response(_calendars: &[Calendar], items: &[CalendarItem]) -> Vec<u8> {
    let mut xml = Vec::new();
    xml.extend_from_slice(
        br#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">"#,
    );

    for item in items {
        xml.extend_from_slice(
            format!(
                r#"<D:response>
<D:href>{}</D:href>
<D:propstat>
<D:prop>
<D:getetag>{}</D:getetag>
<C:calendar-data><![CDATA[{}]]></C:calendar-data>
</D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>"#,
                item.uid,
                item.etag,
                String::from_utf8_lossy(&item.data)
            )
            .as_bytes(),
        );
    }

    xml.extend_from_slice(b"</D:multistatus>");
    xml
}

pub fn build_multiget_response(items: &[(String, Option<CalendarItem>)]) -> Vec<u8> {
    let mut xml = Vec::new();
    xml.extend_from_slice(
        br#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">"#,
    );

    for (href, item) in items {
        match item {
            Some(event) => {
                xml.extend_from_slice(
                    format!(
                        r#"<D:response>
<D:href>{}</D:href>
<D:propstat>
<D:prop>
<D:getetag>{}</D:getetag>
<C:calendar-data><![CDATA[{}]]></C:calendar-data>
</D:prop>
<D:status>HTTP/1.1 200 OK</D:status>
</D:propstat>
</D:response>"#,
                        href,
                        event.etag,
                        String::from_utf8_lossy(&event.data)
                    )
                    .as_bytes(),
                );
            }
            None => {
                xml.extend_from_slice(
                    format!(
                        r#"<D:response>
<D:href>{}</D:href>
<D:propstat>
<D:prop>
<D:getetag/>
<C:calendar-data/>
</D:prop>
<D:status>HTTP/1.1 404 Not Found</D:status>
</D:propstat>
</D:response>"#,
                        href
                    )
                    .as_bytes(),
                );
            }
        }
    }

    xml.extend_from_slice(b"</D:multistatus>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multiget_report() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <D:href>/dav/cal/work/event1.ics</D:href>
    <D:href>/dav/cal/work/event2.ics</D:href>
</D:calendar-multiget>"#;

        match parse_report(body).unwrap() {
            ReportType::CalendarMultiget { hrefs } => {
                assert_eq!(hrefs.len(), 2);
                assert_eq!(hrefs[0], "/dav/cal/work/event1.ics");
                assert_eq!(hrefs[1], "/dav/cal/work/event2.ics");
            }
            _ => panic!("Expected CalendarMultiget"),
        }
    }

    #[test]
    fn test_parse_calendar_query() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <D:prop>
        <D:getetag/>
        <C:calendar-data/>
    </D:prop>
    <C:filter>
        <C:comp-filter name="VCALENDAR">
            <C:comp-filter name="VEVENT">
                <C:time-range start="20240101T000000Z" end="20241231T235959Z"/>
            </C:comp-filter>
        </C:comp-filter>
    </C:filter>
</C:calendar-query>"#;

        match parse_report(body).unwrap() {
            ReportType::CalendarQuery {
                time_range_start,
                time_range_end,
            } => {
                assert_eq!(time_range_start.as_deref(), Some("20240101T000000Z"));
                assert_eq!(time_range_end.as_deref(), Some("20241231T235959Z"));
            }
            _ => panic!("Expected CalendarQuery"),
        }
    }
}
