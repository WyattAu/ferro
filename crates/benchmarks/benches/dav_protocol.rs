use criterion::{Criterion, criterion_group, criterion_main};

const SMALL_ICAL: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Ferro//EN\r\n\
BEGIN:VEVENT\r\n\
UID:evt-001@ferro\r\n\
DTSTART:20240115T090000Z\r\n\
DTEND:20240115T100000Z\r\n\
SUMMARY:Team Standup\r\n\
DESCRIPTION:Daily standup meeting\r\n\
LOCATION:Conference Room A\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

const MEDIUM_ICAL: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Ferro//EN\r\n\
CALSCALE:GREGORIAN\r\n\
METHOD:PUBLISH\r\n\
X-WR-CALNAME:Work Calendar\r\n\
BEGIN:VEVENT\r\n\
UID:evt-001@ferro\r\n\
DTSTART:20240115T090000Z\r\n\
DTEND:20240115T100000Z\r\n\
SUMMARY:Team Standup\r\n\
DESCRIPTION:Daily standup\\, be on time\r\n\
LOCATION:Conference Room A\r\n\
RRULE:FREQ=DAILY;COUNT=30\r\n\
CATEGORIES:MEETING,WORK\r\n\
ATTENDEE;CN=John:mailto:john@example.com\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:evt-002@ferro\r\n\
DTSTART:20240116T140000Z\r\n\
DTEND:20240116T153000Z\r\n\
SUMMARY:Sprint Planning\r\n\
DESCRIPTION:Bi-weekly sprint planning\r\n\
LOCATION:Zoom\r\n\
STATUS:CONFIRMED\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:evt-003@ferro\r\n\
DTSTART;VALUE=DATE:20240120\r\n\
DTEND;VALUE=DATE:20240121\r\n\
SUMMARY:Project Deadline\r\n\
DESCRIPTION:Final delivery\r\n\
PRIORITY:1\r\n\
END:VEVENT\r\n\
BEGIN:VTODO\r\n\
UID:todo-001@ferro\r\n\
SUMMARY:Write documentation\r\n\
STATUS:IN-PROCESS\r\n\
PERCENT-COMPLETE:50\r\n\
DUE:20240130T235959Z\r\n\
END:VTODO\r\n\
END:VCALENDAR\r\n";

const LARGE_ICAL: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Ferro//EN\r\n\
CALSCALE:GREGORIAN\r\n\
METHOD:PUBLISH\r\n\
X-WR-CALNAME:Company Events\r\n\
BEGIN:VEVENT\r\n\
UID:evt-001@ferro\r\n\
DTSTART:20240115T090000Z\r\n\
DTEND:20240115T100000Z\r\n\
SUMMARY:All-Hands Meeting\r\n\
DESCRIPTION:Monthly company all-hands with CEO presentation and Q&A session\r\n\
LOCATION:Main Auditorium\r\n\
RRULE:FREQ=MONTHLY;COUNT=12\r\n\
CATEGORIES:MEETING,COMPANY\r\n\
ATTENDEE;CN=CEO:mailto:ceo@example.com\r\n\
ATTENDEE;CN=CTO:mailto:cto@example.com\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:evt-002@ferro\r\n\
DTSTART:20240116T140000Z\r\n\
DTEND:20240116T153000Z\r\n\
SUMMARY:Engineering Sync\r\n\
DESCRIPTION:Cross-team engineering sync\r\n\
LOCATION:Room B\r\n\
STATUS:TENTATIVE\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:evt-003@ferro\r\n\
DTSTART:20240117T110000Z\r\n\
DTEND:20240117T120000Z\r\n\
SUMMARY:Lunch & Learn\r\n\
DESCRIPTION:Tech talk on Rust async patterns\r\n\
LOCATION:Cafeteria\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:evt-004@ferro\r\n\
DTSTART:20240118T160000Z\r\n\
DTEND:20240118T170000Z\r\n\
SUMMARY:Design Review\r\n\
DESCRIPTION:Review new UI mockups\r\n\
LOCATION:Design Lab\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:evt-005@ferro\r\n\
DTSTART:20240119T090000Z\r\n\
DTEND:20240119T100000Z\r\n\
SUMMARY:1:1 with Manager\r\n\
DESCRIPTION:Weekly check-in\r\n\
END:VEVENT\r\n\
BEGIN:VTODO\r\n\
UID:todo-001@ferro\r\n\
SUMMARY:Implement feature X\r\n\
STATUS:IN-PROCESS\r\n\
PERCENT-COMPLETE:75\r\n\
DUE:20240130T235959Z\r\n\
PRIORITY:1\r\n\
END:VTODO\r\n\
BEGIN:VTODO\r\n\
UID:todo-002@ferro\r\n\
SUMMARY:Write tests for feature X\r\n\
STATUS:NEEDS-ACTION\r\n\
DUE:20240205T235959Z\r\n\
END:VTODO\r\n\
END:VCALENDAR\r\n";

const SMALL_VCARD: &str = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:John Doe\r\n\
N:Doe;John;;;\r\n\
EMAIL:john@example.com\r\n\
TEL;TYPE=HOME:+1-555-0101\r\n\
END:VCARD\r\n";

const MEDIUM_VCARD: &str = "BEGIN:VCARD\r\n\
VERSION:3.0\r\n\
FN:Jane Smith\r\n\
N:Smith;Jane;;;\r\n\
ORG:Acme Corp;Engineering\r\n\
TITLE:Senior Engineer\r\n\
EMAIL;TYPE=WORK:Jane.Smith@acme.com\r\n\
EMAIL;TYPE=HOME:jane@personal.com\r\n\
TEL;TYPE=WORK:+1-555-0202\r\n\
TEL;TYPE=CELL:+1-555-0303\r\n\
ADR;TYPE=WORK:;;123 Main St;San Francisco;CA;94105;USA\r\n\
URL:https://acme.com/jsmith\r\n\
BDAY:19900515\r\n\
NOTE:Lead developer on Project Alpha\r\n\
END:VCARD\r\n";

const LARGE_VCARD: &str = "BEGIN:VCARD\r\n\
VERSION:4.0\r\n\
FN:Alice Johnson\r\n\
N:Johnson;Alice;;;M.S.\r\n\
ORG:TechStart Inc;R&D;Platform Team\r\n\
TITLE:Principal Engineer\r\n\
ROLE:Technical Lead\r\n\
EMAIL;TYPE=WORK;TYPE=PREF:alice.j@techstart.com\r\n\
EMAIL;TYPE=HOME:alice.johnson@gmail.com\r\n\
EMAIL;TYPE=OTHER:alice@sideproject.io\r\n\
TEL;TYPE=WORK;TYPE=PREF:+1-555-0404\r\n\
TEL;TYPE=CELL:+1-555-0505\r\n\
TEL;TYPE=HOME:+1-555-0606\r\n\
ADR;TYPE=WORK:;;456 Innovation Blvd;Palo Alto;CA;94301;USA\r\n\
ADR;TYPE=HOME:;;789 Oak Lane;Mountain View;CA;94043;USA\r\n\
URL:https://techstart.com/team/alice\r\n\
URL:https://github.com/alicej\r\n\
BDAY:19880320\r\n\
NOTE:Expert in distributed systems and Rust. Maintains the core platform SDK.\r\n\
CATEGORIES:ENGINEERING,LEADERSHIP,RUST\r\n\
PHOTO;TYPE=JPEG:https://techstart.com/photos/alice.jpg\r\n\
END:VCARD\r\n";

fn bench_ical_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("ical_parse");
    group.bench_function("small", |b| b.iter(|| ferro_dav::ical::parse_ical(SMALL_ICAL).unwrap()));
    group.bench_function("medium", |b| {
        b.iter(|| ferro_dav::ical::parse_ical(MEDIUM_ICAL).unwrap())
    });
    group.bench_function("large", |b| b.iter(|| ferro_dav::ical::parse_ical(LARGE_ICAL).unwrap()));
    group.finish();
}

fn bench_ical_serialize(c: &mut Criterion) {
    let small = ferro_dav::ical::parse_ical(SMALL_ICAL).unwrap();
    let medium = ferro_dav::ical::parse_ical(MEDIUM_ICAL).unwrap();
    let large = ferro_dav::ical::parse_ical(LARGE_ICAL).unwrap();

    let mut group = c.benchmark_group("ical_serialize");
    group.bench_function("small", |b| b.iter(|| ferro_dav::ical::serialize_ical(&small)));
    group.bench_function("medium", |b| b.iter(|| ferro_dav::ical::serialize_ical(&medium)));
    group.bench_function("large", |b| b.iter(|| ferro_dav::ical::serialize_ical(&large)));
    group.finish();
}

fn bench_ical_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("ical_roundtrip");
    group.bench_function("small", |b| {
        b.iter(|| {
            let parsed = ferro_dav::ical::parse_ical(SMALL_ICAL).unwrap();
            ferro_dav::ical::serialize_ical(&parsed)
        })
    });
    group.bench_function("medium", |b| {
        b.iter(|| {
            let parsed = ferro_dav::ical::parse_ical(MEDIUM_ICAL).unwrap();
            ferro_dav::ical::serialize_ical(&parsed)
        })
    });
    group.finish();
}

fn bench_vcard_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("vcard_parse");
    group.bench_function("small", |b| {
        b.iter(|| ferro_dav::vcard::parse_vcard(SMALL_VCARD).unwrap())
    });
    group.bench_function("medium", |b| {
        b.iter(|| ferro_dav::vcard::parse_vcard(MEDIUM_VCARD).unwrap())
    });
    group.bench_function("large", |b| {
        b.iter(|| ferro_dav::vcard::parse_vcard(LARGE_VCARD).unwrap())
    });
    group.finish();
}

fn bench_vcard_serialize(c: &mut Criterion) {
    let small = ferro_dav::vcard::parse_vcard(SMALL_VCARD).unwrap();
    let medium = ferro_dav::vcard::parse_vcard(MEDIUM_VCARD).unwrap();
    let large = ferro_dav::vcard::parse_vcard(LARGE_VCARD).unwrap();

    let mut group = c.benchmark_group("vcard_serialize");
    group.bench_function("small", |b| b.iter(|| ferro_dav::vcard::serialize_vcard(&small)));
    group.bench_function("medium", |b| b.iter(|| ferro_dav::vcard::serialize_vcard(&medium)));
    group.bench_function("large", |b| b.iter(|| ferro_dav::vcard::serialize_vcard(&large)));
    group.finish();
}

fn bench_vcard_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("vcard_roundtrip");
    group.bench_function("small", |b| {
        b.iter(|| {
            let parsed = ferro_dav::vcard::parse_vcard(SMALL_VCARD).unwrap();
            ferro_dav::vcard::serialize_vcard(&parsed)
        })
    });
    group.bench_function("medium", |b| {
        b.iter(|| {
            let parsed = ferro_dav::vcard::parse_vcard(MEDIUM_VCARD).unwrap();
            ferro_dav::vcard::serialize_vcard(&parsed)
        })
    });
    group.finish();
}

fn bench_xml_escape(c: &mut Criterion) {
    let plain = "a".repeat(1024);
    let special = "<tag attr=\"value\">&text</tag>".repeat(100);

    let mut group = c.benchmark_group("xml_escape");
    group.bench_function("plain_1kb", |b| b.iter(|| ferro_dav::xml_ext::escape_xml(&plain)));
    group.bench_function("special_chars", |b| b.iter(|| ferro_dav::xml_ext::escape_xml(&special)));
    group.finish();
}

fn bench_xml_multistatus(c: &mut Criterion) {
    use ferro_dav::xml_ext::{DavResponse, PropStat};

    let responses = vec![
        DavResponse {
            href: "/dav/file1.txt".into(),
            propstats: vec![PropStat {
                status: 200,
                props: vec![],
            }],
        },
        DavResponse {
            href: "/dav/file2.txt".into(),
            propstats: vec![PropStat {
                status: 404,
                props: vec![],
            }],
        },
        DavResponse {
            href: "/dav/dir/".into(),
            propstats: vec![PropStat {
                status: 200,
                props: vec![],
            }],
        },
    ];

    c.bench_function("dav_multistatus_3_responses", |b| {
        b.iter(|| ferro_dav::xml_ext::build_dav_multistatus(&responses))
    });
}

criterion_group!(
    benches,
    bench_ical_parse,
    bench_ical_serialize,
    bench_ical_roundtrip,
    bench_vcard_parse,
    bench_vcard_serialize,
    bench_vcard_roundtrip,
    bench_xml_escape,
    bench_xml_multistatus,
);
criterion_main!(benches);
