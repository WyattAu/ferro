use criterion::{Criterion, criterion_group, criterion_main};

fn bench_ical_parse(c: &mut Criterion) {
    let ical = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//Test//EN
BEGIN:VEVENT
DTSTART:20240115T100000Z
DTEND:20240120T180000Z
SUMMARY:Test Event
DESCRIPTION:This is a test event description
LOCATION:Test Location
ORGANIZER:mailto:organizer@example.com
ATTENDEE:mailto:attendee1@example.com
ATTENDEE:mailto:attendee2@example.com
UID:test-event-001@example.com
END:VEVENT
BEGIN:VEVENT
DTSTART:20240201T090000Z
DTEND:20240201T100000Z
SUMMARY:Another Event
UID:test-event-002@example.com
END:VEVENT
BEGIN:VTODO
DTSTART:20240301T000000Z
DUE:20240315T000000Z
SUMMARY:Complete task
PRIORITY:1
STATUS:NEEDS-ACTION
UID:test-todo-001@example.com
END:VTODO
END:VCALENDAR"#;

    c.bench_function("parse_icalendar_3_components", |b| {
        b.iter(|| {
            let _ = ferro_dav::ical::parse_ical(ical);
        })
    });
}

fn bench_vcard_parse(c: &mut Criterion) {
    let vcard = r#"BEGIN:VCARD
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
PHOTO;TYPE=JPEG;ENCODING=b:data:image/jpeg;base64,/9j/4AAQ
X-SOCIALPROFILE;TYPE=twitter:@johndoe
REV:20240101T000000Z
END:VCARD"#;

    c.bench_function("parse_vcard_complex", |b| {
        b.iter(|| {
            let _ = ferro_dav::vcard::parse_vcard(vcard);
        })
    });
}

fn bench_calendar_query_parse(c: &mut Criterion) {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:prop-filter name="VEVENT">
        <C:time-range start="20240101T000000Z" end="20240201T000000Z"/>
      </C:prop-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#;

    c.bench_function("parse_calendar_query", |b| {
        b.iter(|| {
            let _ = ferro_dav::xml_ext::parse_calendar_query_time_range(xml);
        })
    });
}

fn bench_addressbook_query_parse(c: &mut Criterion) {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<A:addressbook-query xmlns:D="DAV:" xmlns:A="urn:ietf:params:xml:ns:carddav">
  <D:prop>
    <D:getetag/>
    <A:address-data/>
  </D:prop>
  <A:filter>
    <A:prop-filter name="FN">
      <A:text-match collation="i;unicode-casemap" match-type="contains">Doe</A:text-match>
    </A:prop-filter>
  </A:filter>
</A:addressbook-query>"#;

    c.bench_function("parse_addressbook_query", |b| {
        b.iter(|| {
            let _ = ferro_dav::xml_ext::parse_addressbook_query_filter(xml);
        })
    });
}

fn bench_multistatus_build(c: &mut Criterion) {
    use ferro_dav::xml_ext::{DavProp, DavResponse, PropStat};

    let responses = vec![
        DavResponse {
            href: "/".to_string(),
            propstats: vec![PropStat {
                status: 200,
                props: vec![
                    DavProp {
                        name: "D:resourcetype".to_string(),
                        namespace: None,
                        value: Some("<D:collection/>".to_string()),
                    },
                    DavProp {
                        name: "D:getcontentlength".to_string(),
                        namespace: None,
                        value: Some("0".to_string()),
                    },
                    DavProp {
                        name: "D:getlastmodified".to_string(),
                        namespace: None,
                        value: Some("Wed, 01 Jan 2024 00:00:00 GMT".to_string()),
                    },
                    DavProp {
                        name: "D:getetag".to_string(),
                        namespace: None,
                        value: Some("\"root-etag\"".to_string()),
                    },
                ],
            }],
        },
        DavResponse {
            href: "/Documents/".to_string(),
            propstats: vec![PropStat {
                status: 200,
                props: vec![
                    DavProp {
                        name: "D:resourcetype".to_string(),
                        namespace: None,
                        value: Some("<D:collection/>".to_string()),
                    },
                    DavProp {
                        name: "D:getcontentlength".to_string(),
                        namespace: None,
                        value: Some("0".to_string()),
                    },
                    DavProp {
                        name: "D:getlastmodified".to_string(),
                        namespace: None,
                        value: Some("Mon, 15 Jan 2024 10:30:00 GMT".to_string()),
                    },
                ],
            }],
        },
        DavResponse {
            href: "/readme.txt".to_string(),
            propstats: vec![PropStat {
                status: 200,
                props: vec![
                    DavProp {
                        name: "D:getcontentlength".to_string(),
                        namespace: None,
                        value: Some("2048".to_string()),
                    },
                    DavProp {
                        name: "D:getcontenttype".to_string(),
                        namespace: None,
                        value: Some("text/plain".to_string()),
                    },
                ],
            }],
        },
    ];

    c.bench_function("build_multistatus_3_responses", |b| {
        b.iter(|| {
            ferro_dav::xml_ext::build_dav_multistatus(&responses);
        })
    });
}

criterion_group!(
    benches,
    bench_ical_parse,
    bench_vcard_parse,
    bench_calendar_query_parse,
    bench_addressbook_query_parse,
    bench_multistatus_build,
);
criterion_main!(benches);
