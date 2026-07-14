use criterion::{Criterion, criterion_group, criterion_main};
use ferro_dav::ical::{IcalComponent, IcalProperty, parse_ical, serialize_ical};
use ferro_dav::vcard::{Vcard, parse_vcard, serialize_vcard};
use hashbrown::HashMap;
use std::hint::black_box;

fn benchmark_ical_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("ical_parse");

    group.bench_function("parse_small", |b| {
        let input = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
UID:test-1
DTSTART:20240101T100000Z
DTEND:20240101T110000Z
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR"#;
        b.iter(|| {
            black_box(parse_ical(input).unwrap());
        });
    });

    group.bench_function("parse_medium", |b| {
        let mut input = String::from("BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Test//Test//EN\n");
        for i in 0..100 {
            input.push_str(&format!(
                "BEGIN:VEVENT\nUID:test-{}\nDTSTART:20240101T{:02}0000Z\nDTEND:20240101T{:02}0000Z\nSUMMARY:Event {}\nEND:VEVENT\n",
                i, 10 + (i % 8), 11 + (i % 8), i
            ));
        }
        input.push_str("END:VCALENDAR");
        b.iter(|| {
            black_box(parse_ical(&input).unwrap());
        });
    });

    group.finish();
}

fn benchmark_ical_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("ical_serialize");

    group.bench_function("serialize_small", |b| {
        let mut component = IcalComponent {
            name: "VCALENDAR".to_string(),
            properties: HashMap::new(),
            children: vec![],
        };
        component.properties.insert(
            "VERSION".to_string(),
            vec![IcalProperty {
                name: "VERSION".to_string(),
                params: HashMap::new(),
                value: "2.0".to_string(),
            }],
        );
        let mut event = IcalComponent {
            name: "VEVENT".to_string(),
            properties: HashMap::new(),
            children: vec![],
        };
        event.properties.insert(
            "UID".to_string(),
            vec![IcalProperty {
                name: "UID".to_string(),
                params: HashMap::new(),
                value: "test-1".to_string(),
            }],
        );
        event.properties.insert(
            "DTSTART".to_string(),
            vec![IcalProperty {
                name: "DTSTART".to_string(),
                params: HashMap::new(),
                value: "20240101T100000Z".to_string(),
            }],
        );
        event.properties.insert(
            "DTEND".to_string(),
            vec![IcalProperty {
                name: "DTEND".to_string(),
                params: HashMap::new(),
                value: "20240101T110000Z".to_string(),
            }],
        );
        event.properties.insert(
            "SUMMARY".to_string(),
            vec![IcalProperty {
                name: "SUMMARY".to_string(),
                params: HashMap::new(),
                value: "Test Event".to_string(),
            }],
        );
        component.children.push(event);
        b.iter(|| {
            black_box(serialize_ical(&[component.clone()]));
        });
    });

    group.finish();
}

fn benchmark_vcard_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("vcard_parse");

    group.bench_function("parse_small", |b| {
        let input = r#"BEGIN:VCARD
VERSION:3.0
FN:John Doe
EMAIL:john@example.com
END:VCARD"#;
        b.iter(|| {
            black_box(parse_vcard(input).unwrap());
        });
    });

    group.bench_function("parse_complex", |b| {
        let input = r#"BEGIN:VCARD
VERSION:3.0
FN:John Doe
N:Doe;John;;;
EMAIL;TYPE=WORK:john.doe@example.com
EMAIL;TYPE=HOME:john@home.com
TEL;TYPE=CELL:+1-555-0100
TEL;TYPE=WORK:+1-555-0200
ADR;TYPE=WORK:;;100 Main St;Springfield;IL;62701;USA
ORG:Test Company;Engineering
TITLE:Software Engineer
NOTE:This is a test contact
REV:20240101T000000Z
END:VCARD"#;
        b.iter(|| {
            black_box(parse_vcard(input).unwrap());
        });
    });

    group.finish();
}

fn benchmark_vcard_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("vcard_serialize");

    group.bench_function("serialize_small", |b| {
        let vcard = Vcard {
            uid: Some("urn:uuid:test-1".to_string()),
            fn_name: "John Doe".to_string(),
            family_name: "Doe".to_string(),
            given_name: "John".to_string(),
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
            version: Some("3.0".to_string()),
            properties: HashMap::new(),
        };
        b.iter(|| {
            black_box(serialize_vcard(&vcard));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_ical_parse,
    benchmark_ical_serialize,
    benchmark_vcard_parse,
    benchmark_vcard_serialize
);
criterion_main!(benches);
