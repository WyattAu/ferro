use proptest::prelude::*;

use crate::xml_escape::{escape_xml, unescape_xml};

proptest! {
    #[test]
    fn escaped_no_raw_xml_chars(s in ".*") {
        let escaped = escape_xml(&s);
        prop_assert!(
            !escaped.contains('<'),
            "escaped output contains '<': {:?}",
            escaped
        );
        prop_assert!(
            !escaped.contains('>'),
            "escaped output contains '>': {:?}",
            escaped
        );
        prop_assert!(
            !escaped.contains('"'),
            "escaped output contains '\"': {:?}",
            escaped
        );
        prop_assert!(
            !escaped.contains('\''),
            "escaped output contains '\\'': {:?}",
            escaped
        );
    }

    #[test]
    fn roundtrip_escape_unescape(s in ".*") {
        let escaped = escape_xml(&s);
        let unescaped = unescape_xml(&escaped);
        prop_assert_eq!(
            s.clone(), unescaped.clone(),
            "roundtrip failed: escape({:?}) = {:?}, unescape(that) = {:?}",
            s, escaped, unescaped
        );
    }
}
