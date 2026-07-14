#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_common::path::{normalize_path, validate_path, parent_path, base_name, join_path};

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);

    // normalize_path must never panic
    let normalized = normalize_path(&s);

    // Result must start with /
    assert!(
        normalized.starts_with('/'),
        "normalize_path did not start with /: {:?}",
        normalized
    );

    // Result must not contain ".." segments
    assert!(
        !normalized.contains(".."),
        "normalize_path produced .. segment: {:?}",
        normalized
    );

    // validate_path must never panic
    let _ = validate_path(&s);

    // parent_path must never panic
    let _ = parent_path(&s);

    // base_name must never panic
    let _ = base_name(&s);

    // join_path must never panic
    let _ = join_path(&s, &s);
});
