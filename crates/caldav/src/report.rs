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
    let mut name_buf = Vec::new();
    let mut attr_key_buf = Vec::new();
    let mut attr_val_buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                name_buf.clear();
                name_buf.extend_from_slice(e.name().as_ref());
                let name = String::from_utf8_lossy(&name_buf);
                let local = name.strip_prefix("C:").unwrap_or(&name);
                if local == "time-range" {
                    for attr in e.attributes().flatten() {
                        attr_key_buf.clear();
                        attr_key_buf.extend_from_slice(attr.key.as_ref());
                        let key = String::from_utf8_lossy(&attr_key_buf);

                        attr_val_buf.clear();
                        attr_val_buf.extend_from_slice(&attr.value);
                        let val = String::from_utf8_lossy(&attr_val_buf).into_owned();

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
    let mut name_buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut in_href = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                name_buf.clear();
                name_buf.extend_from_slice(e.name().as_ref());
                let name = String::from_utf8_lossy(&name_buf);
                let local = name.strip_prefix("D:").unwrap_or(&name);
                if local == "href" {
                    in_href = true;
                }
            }
            Ok(Event::Text(ref e)) if in_href => {
                text_buf.clear();
                text_buf.extend_from_slice(e.as_ref());
                let text = String::from_utf8_lossy(&text_buf);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    hrefs.push(trimmed.to_owned());
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

    let mut data_buf = Vec::new();
    for item in items {
        data_buf.clear();
        data_buf.extend_from_slice(&item.data);
        let data_lossy = String::from_utf8_lossy(&data_buf);
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
                item.uid, item.etag, data_lossy
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

    let mut data_buf = Vec::new();
    for (href, item) in items {
        match item {
            Some(event) => {
                data_buf.clear();
                data_buf.extend_from_slice(&event.data);
                let data_lossy = String::from_utf8_lossy(&data_buf);
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
                        href, event.etag, data_lossy
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
    use crate::calendar::CalendarItem;
    use chrono::Utc;

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

    #[test]
    fn test_report_type_debug() {
        let report = ReportType::CalendarQuery {
            time_range_start: None,
            time_range_end: None,
        };
        assert!(!format!("{:?}", report).is_empty());
    }

    #[test]
    fn test_report_type_clone() {
        let report = ReportType::CalendarMultiget {
            hrefs: vec!["/test".to_string()],
        };
        let cloned = report.clone();
        if let ReportType::CalendarMultiget { hrefs } = cloned {
            assert_eq!(hrefs.len(), 1);
        } else {
            panic!("Expected CalendarMultiget");
        }
    }

    #[test]
    fn test_parse_multiget_empty_hrefs() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
</D:calendar-multiget>"#;

        match parse_report(body).unwrap() {
            ReportType::CalendarMultiget { hrefs } => {
                assert!(hrefs.is_empty());
            }
            _ => panic!("Expected CalendarMultiget"),
        }
    }

    #[test]
    fn test_parse_multiget_oversized_body() {
        let body = vec![b'X'; MAX_XML_BODY_SIZE + 1];
        assert!(parse_report(&body).is_err());
    }

    #[test]
    fn test_parse_calendar_query_no_time_range() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <D:prop>
        <D:getetag/>
    </D:prop>
    <C:filter>
        <C:comp-filter name="VCALENDAR">
            <C:comp-filter name="VEVENT"/>
        </C:comp-filter>
    </C:filter>
</C:calendar-query>"#;

        match parse_report(body).unwrap() {
            ReportType::CalendarQuery {
                time_range_start,
                time_range_end,
            } => {
                assert!(time_range_start.is_none());
                assert!(time_range_end.is_none());
            }
            _ => panic!("Expected CalendarQuery"),
        }
    }

    #[test]
    fn test_build_report_response_empty() {
        let calendars = vec![];
        let items = vec![];
        let resp = build_report_response(&calendars, &items);
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("multistatus"));
    }

    #[test]
    fn test_build_report_response_with_items() {
        let calendars = vec![];
        let items = vec![CalendarItem {
            uid: "event-1".to_string(),
            etag: "123".to_string(),
            data: b"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:event-1\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n".to_vec(),
            last_modified: Utc::now(),
        }];
        let resp = build_report_response(&calendars, &items);
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("event-1"));
        assert!(resp_str.contains("123"));
    }

    #[test]
    fn test_build_multiget_response_found() {
        let items = vec![(
            "/cal/event1.ics".to_string(),
            Some(CalendarItem {
                uid: "event-1".to_string(),
                etag: "1".to_string(),
                data: b"test-data".to_vec(),
                last_modified: Utc::now(),
            }),
        )];
        let resp = build_multiget_response(&items);
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("/cal/event1.ics"));
        assert!(resp_str.contains("200 OK"));
    }

    #[test]
    fn test_build_multiget_response_not_found() {
        let items = vec![("/cal/missing.ics".to_string(), None)];
        let resp = build_multiget_response(&items);
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("404 Not Found"));
    }

    #[test]
    fn test_build_multiget_response_mixed() {
        let items = vec![
            (
                "/cal/found.ics".to_string(),
                Some(CalendarItem {
                    uid: "found".to_string(),
                    etag: "1".to_string(),
                    data: b"found-data".to_vec(),
                    last_modified: Utc::now(),
                }),
            ),
            ("/cal/missing.ics".to_string(), None),
        ];
        let resp = build_multiget_response(&items);
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("200 OK"));
        assert!(resp_str.contains("404 Not Found"));
    }

    #[test]
    fn test_build_multiget_response_empty() {
        let items = vec![];
        let resp = build_multiget_response(&items);
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("multistatus"));
    }

    #[test]
    fn test_parse_multiget_single_href() {
        let body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <D:href>/single/event.ics</D:href>
</D:calendar-multiget>"#;

        match parse_report(body).unwrap() {
            ReportType::CalendarMultiget { hrefs } => {
                assert_eq!(hrefs.len(), 1);
                assert_eq!(hrefs[0], "/single/event.ics");
            }
            _ => panic!("Expected CalendarMultiget"),
        }
    }
}
