#![no_main]
use libfuzzer_sys::fuzz_target;
use ferro_dav::ical::parse_ical;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_ical(s);
    }
});
