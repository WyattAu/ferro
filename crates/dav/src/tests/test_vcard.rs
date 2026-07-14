use crate::vcard::*;

#[test]
fn test_parse_basic_vcard() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
UID:contact-1\r\n\
FN:John Doe\r\n\
N:Doe;John;;;\r\n\
EMAIL:john@example.com\r\n\
TEL:+1-555-1234\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.uid, Some("contact-1".to_string()));
    assert_eq!(vcard.fn_name, "John Doe");
    assert_eq!(vcard.family_name, "Doe");
    assert_eq!(vcard.given_name, "John");
    assert_eq!(vcard.emails.len(), 1);
    assert_eq!(vcard.emails[0].value, "john@example.com");
    assert_eq!(vcard.phones.len(), 1);
    assert_eq!(vcard.phones[0].value, "+1-555-1234");
}

#[test]
fn test_parse_vcard_with_types() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Jane Smith\r\n\
N:Smith;Jane;;;\r\n\
EMAIL;TYPE=WORK:jane@company.com\r\n\
EMAIL;TYPE=HOME;PREF=1:jane@home.com\r\n\
TEL;TYPE=CELL:+1-555-5678\r\n\
TEL;TYPE=WORK:+1-555-9999\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.emails.len(), 2);
    assert_eq!(vcard.emails[0].value, "jane@company.com");
    assert_eq!(vcard.emails[0].types, vec!["WORK"]);
    assert_eq!(vcard.emails[1].value, "jane@home.com");
    assert_eq!(vcard.emails[1].types, vec!["HOME"]);
    assert_eq!(vcard.emails[1].pref, Some(1));

    assert_eq!(vcard.phones.len(), 2);
    assert_eq!(vcard.phones[0].types, vec!["CELL"]);
    assert_eq!(vcard.phones[1].types, vec!["WORK"]);
}

#[test]
fn test_parse_vcard_with_address() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Alice Brown\r\n\
N:Brown;Alice;;;\r\n\
ADR;TYPE=WORK:;;123 Main St;Springfield;IL;62704;USA\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.addresses.len(), 1);
    let addr = &vcard.addresses[0];
    assert_eq!(addr.street, "123 Main St");
    assert_eq!(addr.city, "Springfield");
    assert_eq!(addr.region, "IL");
    assert_eq!(addr.postal_code, "62704");
    assert_eq!(addr.country, "USA");
    assert_eq!(addr.types, vec!["WORK"]);
}

#[test]
fn test_parse_vcard_with_org_title() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Bob Wilson\r\n\
N:Wilson;Bob;;;\r\n\
ORG:Acme Corp\r\n\
TITLE:Engineer\r\n\
ROLE:Developer\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.org, Some("Acme Corp".to_string()));
    assert_eq!(vcard.title, Some("Engineer".to_string()));
    assert_eq!(vcard.role, Some("Developer".to_string()));
}

#[test]
fn test_parse_vcard_with_photo() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Charlie Green\r\n\
N:Green;Charlie;;;\r\n\
PHOTO:data:image/jpeg;base64,/9j/4AAQ\r\n\
REV:20260427T000000Z\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.photo, Some("data:image/jpeg;base64,/9j/4AAQ".to_string()));
    assert_eq!(vcard.rev, Some("20260427T000000Z".to_string()));
}

#[test]
fn test_serialize_vcard_roundtrip() {
    let vcard = Vcard {
        uid: Some("rt-1".to_string()),
        fn_name: "Test User".to_string(),
        family_name: "User".to_string(),
        given_name: "Test".to_string(),
        additional_names: String::new(),
        prefix: String::new(),
        suffix: String::new(),
        emails: vec![VcardValue {
            value: "test@example.com".to_string(),
            types: vec!["WORK".to_string()],
            pref: Some(1),
        }],
        phones: vec![VcardValue {
            value: "+1-555-0000".to_string(),
            types: vec!["CELL".to_string()],
            pref: None,
        }],
        addresses: vec![],
        org: Some("Test Inc".to_string()),
        title: Some("Tester".to_string()),
        role: None,
        photo: None,
        rev: Some("20260427T000000Z".to_string()),
        version: Some("3.0".to_string()),
        properties: hashbrown::HashMap::new(),
    };

    let output = crate::vcard::serialize_vcard(&vcard);
    let parsed = crate::vcard::parse_vcard(&output).unwrap();
    assert_eq!(parsed.uid, vcard.uid);
    assert_eq!(parsed.fn_name, vcard.fn_name);
    assert_eq!(parsed.family_name, vcard.family_name);
    assert_eq!(parsed.given_name, vcard.given_name);
    assert_eq!(parsed.emails.len(), 1);
    assert_eq!(parsed.emails[0].value, "test@example.com");
    assert_eq!(parsed.org, Some("Test Inc".to_string()));
}

#[test]
fn test_parse_vcard_with_multiple_addresses() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Multi Address\r\n\
N:Test;Multi;;;\r\n\
ADR;TYPE=HOME:;;456 Home Ave;Homeville;CA;90210;USA\r\n\
ADR;TYPE=WORK:;;789 Office Blvd;Worktown;NY;10001;USA\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.addresses.len(), 2);
    assert_eq!(vcard.addresses[0].street, "456 Home Ave");
    assert_eq!(vcard.addresses[0].types, vec!["HOME"]);
    assert_eq!(vcard.addresses[1].street, "789 Office Blvd");
    assert_eq!(vcard.addresses[1].types, vec!["WORK"]);
}

#[test]
fn test_parse_vcard_empty() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert!(vcard.fn_name.is_empty());
    assert!(vcard.emails.is_empty());
    assert!(vcard.phones.is_empty());
}

#[test]
fn test_parse_vcard_no_begin() {
    let input = "VERSION:3.0\r\n\
FN:No Begin\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert!(vcard.fn_name.is_empty());
}

#[test]
fn test_parse_vcard_custom_properties() {
    let input = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Custom Props\r\n\
X-CUSTOM:value1\r\n\
X-OTHER:value2\r\n\
END:VCARD\r\n";

    let vcard = parse_vcard(input).unwrap();
    assert_eq!(vcard.fn_name, "Custom Props");
    assert!(vcard.properties.contains_key("X-CUSTOM"));
    assert!(vcard.properties.contains_key("X-OTHER"));
}

#[test]
fn test_serialize_vcard_default_version() {
    let vcard = Vcard {
        uid: None,
        fn_name: "No Version".to_string(),
        family_name: String::new(),
        given_name: String::new(),
        additional_names: String::new(),
        prefix: String::new(),
        suffix: String::new(),
        emails: vec![],
        phones: vec![],
        addresses: vec![],
        org: None,
        title: None,
        role: None,
        photo: None,
        rev: None,
        version: None,
        properties: hashbrown::HashMap::new(),
    };

    let output = crate::vcard::serialize_vcard(&vcard);
    assert!(output.contains("VERSION:3.0"));
}

#[test]
fn test_serialize_vcard_with_all_fields() {
    let vcard = Vcard {
        uid: Some("uid-123".to_string()),
        fn_name: "Full Name".to_string(),
        family_name: "Name".to_string(),
        given_name: "Full".to_string(),
        additional_names: "Middle".to_string(),
        prefix: "Dr.".to_string(),
        suffix: "Jr.".to_string(),
        emails: vec![VcardValue {
            value: "test@example.com".to_string(),
            types: vec!["HOME".to_string()],
            pref: Some(1),
        }],
        phones: vec![VcardValue {
            value: "+1-555-0000".to_string(),
            types: vec!["CELL".to_string()],
            pref: None,
        }],
        addresses: vec![VcardAddress {
            po_box: "12345".to_string(),
            extended: "Apt 1".to_string(),
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            region: "IL".to_string(),
            postal_code: "62704".to_string(),
            country: "USA".to_string(),
            types: vec!["HOME".to_string()],
        }],
        org: Some("Test Org".to_string()),
        title: Some("Engineer".to_string()),
        role: Some("Developer".to_string()),
        photo: Some("http://example.com/photo.jpg".to_string()),
        rev: Some("20260427T000000Z".to_string()),
        version: Some("4.0".to_string()),
        properties: hashbrown::HashMap::new(),
    };

    let output = crate::vcard::serialize_vcard(&vcard);
    assert!(output.contains("BEGIN:VCARD"));
    assert!(output.contains("END:VCARD"));
    assert!(output.contains("VERSION:4.0"));
    assert!(output.contains("UID:uid-123"));
    assert!(output.contains("FN:Full Name"));
    assert!(output.contains("ORG:Test Org"));
    assert!(output.contains("TITLE:Engineer"));
    assert!(output.contains("ROLE:Developer"));
    assert!(output.contains("PHOTO:http://example.com/photo.jpg"));
    assert!(output.contains("REV:20260427T000000Z"));
}

#[test]
fn test_vcard_escape_special_chars() {
    let vcard = Vcard {
        uid: None,
        fn_name: "Name with;semicolons".to_string(),
        family_name: String::new(),
        given_name: String::new(),
        additional_names: String::new(),
        prefix: String::new(),
        suffix: String::new(),
        emails: vec![VcardValue {
            value: "test@example.com".to_string(),
            types: vec![],
            pref: None,
        }],
        phones: vec![],
        addresses: vec![],
        org: None,
        title: None,
        role: None,
        photo: None,
        rev: None,
        version: None,
        properties: hashbrown::HashMap::new(),
    };

    let output = crate::vcard::serialize_vcard(&vcard);
    assert!(output.contains("FN:Name with\\;semicolons"));
}
