#![no_main]
use libfuzzer_sys::fuzz_target;
use ferro_dav::vcard::parse_vcard;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_vcard(s);
    }
});
