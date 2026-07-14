use proptest::prelude::*;

use crate::vcard::{Vcard, VcardValue, parse_vcard, serialize_vcard};

fn arb_vcard_value() -> impl Strategy<Value = VcardValue> {
    (
        "[a-zA-Z0-9._%+-]{1,50}@[a-zA-Z0-9.-]{1,50}",
        prop::collection::vec("[A-Z]{2,6}", 0..3),
        prop::option::of(1u32..100),
    )
        .prop_map(|(value, types, pref)| VcardValue { value, types, pref })
}

fn arb_vcard() -> impl Strategy<Value = Vcard> {
    (
        "[a-zA-Z]{1,50}",
        "[a-zA-Z]{1,50}",
        prop::collection::vec(arb_vcard_value(), 0..3),
        prop::option::of("[a-zA-Z0-9]{1,64}"),
        prop::option::of("[a-zA-Z]{1,30}"),
        prop::option::of("[a-zA-Z]{1,30}"),
    )
        .prop_map(|(given, family, emails, uid, org, title)| Vcard {
            uid,
            fn_name: format!("{given} {family}"),
            family_name: family,
            given_name: given,
            additional_names: String::new(),
            prefix: String::new(),
            suffix: String::new(),
            emails,
            phones: vec![],
            addresses: vec![],
            org,
            title,
            role: None,
            photo: None,
            rev: None,
            version: Some("4.0".to_string()),
            properties: Default::default(),
        })
}

proptest! {
    #[test]
    fn vcard_roundtrip(vcard in arb_vcard()) {
        let serialized = serialize_vcard(&vcard);
        let parsed = parse_vcard(&serialized).unwrap();

        prop_assert_eq!(parsed.fn_name, vcard.fn_name);
        prop_assert_eq!(parsed.given_name, vcard.given_name);
        prop_assert_eq!(parsed.family_name, vcard.family_name);
        prop_assert_eq!(parsed.emails.len(), vcard.emails.len());

        for (orig, deser) in vcard.emails.iter().zip(parsed.emails.iter()) {
            prop_assert_eq!(&orig.value, &deser.value);
        }
    }

    #[test]
    fn vcard_serialize_never_panics(vcard in arb_vcard()) {
        let _ = serialize_vcard(&vcard);
    }

    #[test]
    fn vcard_parse_never_panics(input in ".*") {
        let _ = parse_vcard(&input);
    }

    #[test]
    fn vcard_has_begin_end(input in "BEGIN:VCARD\r\nFN:Test\r\nEND:VCARD\r\n") {
        prop_assert!(input.contains("BEGIN:VCARD"));
        prop_assert!(input.contains("END:VCARD"));
    }

    #[test]
    fn vcard_serialize_has_version(vcard in arb_vcard()) {
        let serialized = serialize_vcard(&vcard);
        prop_assert!(serialized.contains("VERSION:"));
    }
}
