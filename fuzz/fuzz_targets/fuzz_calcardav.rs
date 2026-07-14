#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_dav::xml_ext::{
    parse_calendar_query_time_range, parse_addressbook_query_filter, parse_multiget_hrefs,
};
use ferro_dav::ical::parse_ical;
use ferro_webdav_handler::escape_xml;

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);

    // Fuzz CalDAV time-range XML parsing
    let _ = parse_calendar_query_time_range(data);

    // Fuzz CardDAV addressbook-query filter parsing
    let _ = parse_addressbook_query_filter(data);

    // Fuzz multiget href extraction
    let _ = parse_multiget_hrefs(data);

    // Fuzz iCalendar parsing
    let _ = parse_ical(&s);

    // Fuzz XML escape (must not panic on any input)
    let escaped = escape_xml(&s);
    for ch in escaped.chars() {
        assert!(
            !matches!(ch, '<' | '>' | '"' | '\''),
            "unescaped XML char: {:?}",
            ch
        );
    }

    // Fuzz quick_xml Reader on raw bytes
    let mut reader = quick_xml::Reader::from_reader(data);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
});
