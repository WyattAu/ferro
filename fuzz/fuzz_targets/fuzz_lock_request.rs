#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_webdav_handler::LockRequest;

fuzz_target!(|data: &[u8]| {
    // Must not panic on arbitrary byte input
    let _ = LockRequest::parse(data);
});
