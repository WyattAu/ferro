use crate::xml_ext::*;

#[test]
fn test_parse_time_range_present() {
    let xml = br#"<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop><D:getetag/><C:calendar-data/></D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:prop-filter name="VEVENT">
        <C:time-range start="20240101T000000Z" end="20240201T000000Z"/>
      </C:prop-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#;

    let (start, end) = parse_calendar_query_time_range(xml).unwrap();
    assert_eq!(start, "20240101T000000Z");
    assert_eq!(end, "20240201T000000Z");
}

#[test]
fn test_parse_time_range_missing() {
    let xml = br#"<C:calendar-query xmlns:C="urn:ietf:params:xml:ns:caldav">
  <C:filter><C:comp-filter name="VCALENDAR"/></C:filter>
</C:calendar-query>"#;
    assert!(parse_calendar_query_time_range(xml).is_none());
}

#[test]
fn test_parse_time_range_attributes_reversed() {
    let xml = br#"<C:time-range end="20240201T000000Z" start="20240101T000000Z"/>"#;
    let (start, end) = parse_calendar_query_time_range(xml).unwrap();
    assert_eq!(start, "20240101T000000Z");
    assert_eq!(end, "20240201T000000Z");
}

#[test]
fn test_parse_text_match_present() {
    let xml = br#"<C:addressbook-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:carddav">
  <D:prop><D:getetag/><C:address-data/></D:prop>
  <C:filter>
    <C:prop-filter name="FN">
      <C:text-match collation="i;unicode-casemap" match-type="contains">John</C:text-match>
    </C:prop-filter>
  </C:filter>
</C:addressbook-query>"#;

    assert_eq!(parse_addressbook_query_filter(xml), Some("John".to_string()));
}

#[test]
fn test_parse_text_match_missing() {
    let xml = br#"<C:addressbook-query xmlns:C="urn:ietf:params:xml:ns:carddav">
  <C:filter><C:prop-filter name="FN"/></C:filter>
</C:addressbook-query>"#;
    assert!(parse_addressbook_query_filter(xml).is_none());
}

#[test]
fn test_parse_text_match_case_insensitive_query() {
    let xml = br#"<A:addressbook-query xmlns:A="urn:ietf:params:xml:ns:carddav">
  <A:filter>
    <A:prop-filter name="EMAIL">
      <A:text-match match-type="contains">alice@example.com</A:text-match>
    </A:prop-filter>
  </A:filter>
</A:addressbook-query>"#;

    assert_eq!(
        parse_addressbook_query_filter(xml),
        Some("alice@example.com".to_string())
    );
}

#[test]
fn test_escape_xml() {
    assert_eq!(escape_xml("a&b").as_ref(), "a&amp;b");
    assert_eq!(escape_xml("<tag>").as_ref(), "&lt;tag&gt;");
    assert_eq!(escape_xml("a\"b").as_ref(), "a&quot;b");
    assert_eq!(escape_xml("a'b").as_ref(), "a&apos;b");
}

#[test]
fn test_build_dav_multistatus_empty() {
    let result = build_dav_multistatus(&[]);
    let s = String::from_utf8(result).unwrap();
    assert!(s.contains("D:multistatus"));
    assert!(s.contains("xmlns:D=\"DAV:\""));
}

#[test]
fn test_parse_multiget_hrefs_calendars() {
    let xml = br#"<C:calendar-multiget xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop><D:getetag/><C:calendar-data/></D:prop>
  <D:href>/dav/cal/personal/evt1.ics</D:href>
  <D:href>/dav/cal/personal/evt2.ics</D:href>
  <D:href>/dav/cal/work/meeting.ics</D:href>
</C:calendar-multiget>"#;

    let hrefs = parse_multiget_hrefs(xml);
    assert_eq!(hrefs.len(), 3);
    assert_eq!(hrefs[0], "/dav/cal/personal/evt1.ics");
    assert_eq!(hrefs[1], "/dav/cal/personal/evt2.ics");
    assert_eq!(hrefs[2], "/dav/cal/work/meeting.ics");
}

#[test]
fn test_parse_multiget_hrefs_contacts() {
    let xml = br#"<A:addressbook-multiget xmlns:D="DAV:" xmlns:A="urn:ietf:params:xml:ns:carddav">
  <D:prop><D:getetag/><A:address-data/></D:prop>
  <D:href>/dav/card/contacts/uid1.vcf</D:href>
  <D:href>/dav/card/contacts/uid2.vcf</D:href>
</A:addressbook-multiget>"#;

    let hrefs = parse_multiget_hrefs(xml);
    assert_eq!(hrefs.len(), 2);
    assert_eq!(hrefs[0], "/dav/card/contacts/uid1.vcf");
    assert_eq!(hrefs[1], "/dav/card/contacts/uid2.vcf");
}

#[test]
fn test_parse_multiget_hrefs_empty() {
    let xml = br#"<C:calendar-multiget xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop><D:getetag/><C:calendar-data/></D:prop>
</C:calendar-multiget>"#;

    let hrefs = parse_multiget_hrefs(xml);
    assert!(hrefs.is_empty());
}

#[test]
fn test_parse_multiget_hrefs_oversized() {
    let mut huge = vec![0u8; 10 * 1024 * 1024 + 1];
    huge[0] = b'<';
    assert!(parse_multiget_hrefs(&huge).is_empty());
}

#[test]
fn test_parse_sync_collection_basic() {
    let xml = br#"<D:sync-collection xmlns:D="DAV:">
  <D:sync-token>12345</D:sync-token>
  <D:prop>
    <D:getetag/>
    <C:calendar-data xmlns:C="urn:ietf:params:xml:ns:caldav"/>
  </D:prop>
</D:sync-collection>"#;

    let result = parse_sync_collection(xml).unwrap();
    assert_eq!(result.sync_token, Some("12345".to_string()));
    assert!(result.want_getetag);
    assert!(result.want_calendar_data);
    assert!(!result.want_address_data);
}

#[test]
fn test_parse_sync_collection_no_token() {
    let xml = br#"<D:sync-collection xmlns:D="DAV:">
  <D:prop>
    <D:getetag/>
  </D:prop>
</D:sync-collection>"#;

    let result = parse_sync_collection(xml).unwrap();
    assert!(result.sync_token.is_none());
    assert!(result.want_getetag);
}

#[test]
fn test_parse_sync_collection_address_data() {
    let xml = br#"<D:sync-collection xmlns:D="DAV:" xmlns:A="urn:ietf:params:xml:ns:carddav">
  <D:sync-token>67890</D:sync-token>
  <D:prop>
    <D:getetag/>
    <A:address-data/>
  </D:prop>
</D:sync-collection>"#;

    let result = parse_sync_collection(xml).unwrap();
    assert_eq!(result.sync_token, Some("67890".to_string()));
    assert!(result.want_address_data);
    assert!(!result.want_calendar_data);
}

#[test]
fn test_parse_sync_collection_oversized() {
    let mut huge = vec![0u8; 10 * 1024 * 1024 + 1];
    huge[0] = b'<';
    assert!(parse_sync_collection(&huge).is_err());
}

#[test]
fn test_build_dav_multistatus_with_responses() {
    let responses = vec![
        DavResponse {
            href: "/file1.txt".to_string(),
            propstats: vec![PropStat {
                status: 200,
                props: vec![DavProp {
                    name: "D:getetag".to_string(),
                    namespace: None,
                    value: Some("\"abc123\"".to_string()),
                }],
            }],
        },
        DavResponse {
            href: "/file2.txt".to_string(),
            propstats: vec![PropStat {
                status: 404,
                props: vec![],
            }],
        },
    ];

    let xml = build_dav_multistatus(&responses);
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("D:multistatus"));
    assert!(s.contains("/file1.txt"));
    assert!(s.contains("/file2.txt"));
    assert!(s.contains("200"));
    assert!(s.contains("404"));
}

#[test]
fn test_build_dav_multistatus_with_namespace() {
    let responses = vec![DavResponse {
        href: "/calendar.ics".to_string(),
        propstats: vec![PropStat {
            status: 200,
            props: vec![DavProp {
                name: "C:calendar-data".to_string(),
                namespace: Some("urn:ietf:params:xml:ns:caldav".to_string()),
                value: Some("BEGIN:VCALENDAR...".to_string()),
            }],
        }],
    }];

    let xml = build_dav_multistatus(&responses);
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("xmlns:C=\"urn:ietf:params:xml:ns:caldav\""));
}

#[test]
fn test_parse_time_range_empty() {
    let xml = br#"<C:time-range/>"#;
    assert!(parse_calendar_query_time_range(xml).is_none());
}

#[test]
fn test_parse_text_match_empty() {
    let xml = br#"<A:text-match/>"#;
    assert!(parse_addressbook_query_filter(xml).is_none());
}

#[test]
fn test_escape_xml_special_chars() {
    assert_eq!(escape_xml("a&b").as_ref(), "a&amp;b");
    assert_eq!(escape_xml("<tag>").as_ref(), "&lt;tag&gt;");
    assert_eq!(escape_xml("a\"b").as_ref(), "a&quot;b");
    assert_eq!(escape_xml("a'b").as_ref(), "a&apos;b");
}

#[test]
fn test_escape_xml_safe_string() {
    let input = "hello world";
    let result = escape_xml(input);
    assert_eq!(result.as_ref(), input);
}

#[test]
fn test_parse_multiget_hrefs_single() {
    let xml = br#"<D:calendar-multiget xmlns:D="DAV:">
  <D:href>/dav/cal/event.ics</D:href>
</D:calendar-multiget>"#;

    let hrefs = parse_multiget_hrefs(xml);
    assert_eq!(hrefs.len(), 1);
    assert_eq!(hrefs[0], "/dav/cal/event.ics");
}

#[test]
fn test_status_text_known() {
    let xml = build_dav_multistatus(&[DavResponse {
        href: "/test".to_string(),
        propstats: vec![PropStat {
            status: 200,
            props: vec![],
        }],
    }]);
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("OK"));
}

#[test]
fn test_status_text_not_found() {
    let xml = build_dav_multistatus(&[DavResponse {
        href: "/test".to_string(),
        propstats: vec![PropStat {
            status: 404,
            props: vec![],
        }],
    }]);
    let s = String::from_utf8(xml).unwrap();
    assert!(s.contains("Not Found"));
}
