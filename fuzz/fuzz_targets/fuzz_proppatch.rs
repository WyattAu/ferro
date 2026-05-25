#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_webdav_handler::parse_proppatch;

fuzz_target!(|data: &[u8]| {
    // Must not panic on arbitrary byte input
    let _ = parse_proppatch(data);
});
