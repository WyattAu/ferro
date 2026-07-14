#![no_main]
use libfuzzer_sys::fuzz_target;
use ferro_dav::xml_ext::{
    parse_calendar_query_time_range, parse_addressbook_query_filter,
    parse_multiget_hrefs, parse_sync_collection,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse_calendar_query_time_range(data);
    let _ = parse_addressbook_query_filter(data);
    let _ = parse_multiget_hrefs(data);
    let _ = parse_sync_collection(data);
});
