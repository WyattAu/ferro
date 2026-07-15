#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_common::metadata::ContentHash;
use ferro_common::path::normalize_path;

fuzz_target!(|data: &[u8]| {
    // Fuzz ContentHash::compute with random byte inputs
    let hash = ContentHash::compute(data);
    assert_eq!(hash.as_str().len(), 64);
    assert!(hash.as_str().chars().all(|c| c.is_ascii_hexdigit()));

    // Determinism check: computing twice must produce the same hash
    let hash2 = ContentHash::compute(data);
    assert_eq!(hash, hash2);

    // Fuzz normalize_path with adversarial inputs derived from the fuzzer data
    let s = String::from_utf8_lossy(data);

    // Must never panic
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
});
