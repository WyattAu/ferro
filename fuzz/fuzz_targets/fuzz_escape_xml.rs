#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_webdav_handler::escape_xml;

fuzz_target!(|data: &[u8]| {
    // Convert arbitrary bytes to string (lossy) and verify escape never panics
    let s = String::from_utf8_lossy(data);
    let escaped = escape_xml(&s);
    // Verify no raw < > " ' characters remain (these are always escaped)
    // Note: & is allowed since escape sequences like &amp; contain &
    for ch in escaped.chars() {
        assert!(
            !matches!(ch, '<' | '>' | '"' | '\''),
            "unescaped XML char: {:?}",
            ch
        );
    }
});
