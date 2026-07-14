use proptest::prelude::*;

use crate::xml_ext::escape_xml;

proptest! {
    #[test]
    fn escaped_xml_no_lt(s in ".*") {
        let escaped = escape_xml(&s);
        prop_assert!(!escaped.contains('<'), "escaped output contains '<': {:?}", escaped);
    }

    #[test]
    fn escaped_xml_no_gt(s in ".*") {
        let escaped = escape_xml(&s);
        prop_assert!(!escaped.contains('>'), "escaped output contains '>': {:?}", escaped);
    }

    #[test]
    fn build_dav_multistatus_produces_valid_xml(href in "[a-zA-Z0-9]{1,50}") {
        let resp = crate::xml_ext::DavResponse {
            href: href.clone(),
            propstats: vec![crate::xml_ext::PropStat {
                status: 200,
                props: vec![crate::xml_ext::DavProp {
                    name: "D:getetag".to_string(),
                    namespace: None,
                    value: Some(href),
                }],
            }],
        };
        let xml = crate::xml_ext::build_dav_multistatus(&[resp]);
        let xml_str = String::from_utf8(xml).unwrap();
        prop_assert!(xml_str.contains("<D:multistatus"));
        prop_assert!(xml_str.contains("</D:multistatus>"));
    }

    #[test]
    fn parse_multiget_hrefs_never_panics(body in ".*") {
        let _ = crate::xml_ext::parse_multiget_hrefs(body.as_bytes());
    }

    #[test]
    fn parse_calendar_query_never_panics(body in ".*") {
        let _ = crate::xml_ext::parse_calendar_query_time_range(body.as_bytes());
    }

    #[test]
    fn parse_addressbook_query_never_panics(body in ".*") {
        let _ = crate::xml_ext::parse_addressbook_query_filter(body.as_bytes());
    }
}
