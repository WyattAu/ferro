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

    assert_eq!(
        parse_addressbook_query_filter(xml),
        Some("John".to_string())
    );
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
    assert_eq!(escape_xml("a&b"), "a&amp;b");
    assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
    assert_eq!(escape_xml("a\"b"), "a&quot;b");
    assert_eq!(escape_xml("a'b"), "a&apos;b");
}

#[test]
fn test_build_dav_multistatus_empty() {
    let result = build_dav_multistatus(&[]);
    let s = String::from_utf8(result).unwrap();
    assert!(s.contains("D:multistatus"));
    assert!(s.contains("xmlns:D=\"DAV:\""));
}
