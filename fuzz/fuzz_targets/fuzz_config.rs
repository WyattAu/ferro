#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_server::config::FileConfigValues;

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);

    // Fuzz TOML config deserialization — must not panic on any input
    let _ = toml::from_str::<FileConfigValues>(&s);
});
