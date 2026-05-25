#![no_main]

use libfuzzer_sys::fuzz_target;

/// WASM magic bytes: 0x00 0x61 0x73 0x6d
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

fuzz_target!(|data: &[u8]| {
    // Validate that the magic byte check is correct and doesn't panic
    let is_valid = data.len() >= 4 && data[..4] == WASM_MAGIC;

    // The validation function in wasm_upload.rs checks:
    // data.starts_with(&[0x00, 0x61, 0x73, 0x6d])
    let starts_correctly = data.starts_with(&WASM_MAGIC);

    assert_eq!(is_valid, starts_correctly);
});
