#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_common::auth::{is_public_auth_path, Claims};

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);

    // is_public_auth_path must never panic on any input
    let _ = is_public_auth_path(&s);

    // Fuzz Claims deserialization from JSON
    let _ = serde_json::from_str::<Claims>(&s);

    // Fuzz Claims serialization roundtrip
    if let Ok(claims) = serde_json::from_str::<Claims>(&s) {
        let _ = serde_json::to_string(&claims);
    }
});
