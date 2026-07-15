#![no_main]

use libfuzzer_sys::fuzz_target;
use ferro_server_fips::{fips_self_test, FipsMode, FipsValidator, KeyManager};

fuzz_target!(|data: &[u8]| {
    // Fuzz FIPS self-test — must never panic
    let results = fips_self_test();
    assert!(results.len() >= 5);

    // Fuzz FipsValidator creation in all modes
    let _v_disabled = FipsValidator::new(FipsMode::Disabled);
    let _v_enabled = FipsValidator::new(FipsMode::Enabled);
    let _v_strict = FipsValidator::new(FipsMode::Strict);

    // Fuzz key wrapping/unwrapping via public KeyManager API
    if data.len() >= 32 {
        let mut km = KeyManager::new();
        km.set_master_key(data[..32].to_vec(), "fuzz-master");

        if let Ok(kek) = km.generate_kek("fuzz-kek") {
            // Generate data key (wraps with KEK internally)
            if let Ok(encrypted_dk) = km.generate_data_key(&kek.key_id, "fuzz-data") {
                // Verify the wrapped key is non-empty
                assert!(!encrypted_dk.wrapped.is_empty());
                assert!(!encrypted_dk.mac.is_empty());
                assert_eq!(encrypted_dk.version, 2);

                // Unwrap the data key back
                let unwrap_result = km.unwrap_data_key(&kek.key_id, &encrypted_dk);
                assert!(unwrap_result.is_ok());

                // Rotate the key
                let rotate_result = km.rotate_data_key(&kek.key_id, &encrypted_dk.key_id, "rotated");
                if let Ok((old, new)) = rotate_result {
                    assert_eq!(old.key_id, encrypted_dk.key_id);
                    assert_ne!(old.key_id, new.key_id);

                    // Destroy the old key
                    let _ = km.destroy_data_key(&old.key_id);
                }
            }
        }
    }
});
