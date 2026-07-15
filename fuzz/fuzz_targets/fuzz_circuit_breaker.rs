#![no_main]

use libfuzzer_sys::fuzz_target;
use std::sync::Arc;
use std::time::Duration;
use ferro_circuit_breaker::CircuitBreaker;

fuzz_target!(|data: &[u8]| {
    let threshold = if data.is_empty() { 1 } else { (data[0] % 10) as u64 + 1 };
    let cb = Arc::new(CircuitBreaker::new(threshold, Duration::from_secs(60)));

    // Fuzz rapid state transitions via synchronous API
    for &byte in data.iter().take(100) {
        if byte % 2 == 0 {
            cb.record_success();
        } else {
            cb.record_failure();
        }
    }

    // State must always be valid
    let state = cb.state();
    assert!(
        matches!(state, ferro_circuit_breaker::CircuitState::Closed
            | ferro_circuit_breaker::CircuitState::Open
            | ferro_circuit_breaker::CircuitState::HalfOpen),
        "invalid state: {:?}",
        state
    );

    // Fuzz concurrent access via synchronous API
    let cb2 = cb.clone();
    let data_clone = data[..data.len().min(50)].to_vec();
    let handle = std::thread::spawn(move || {
        for &byte in data_clone.iter() {
            if byte % 3 == 0 {
                cb2.record_success();
            } else {
                cb2.record_failure();
            }
        }
    });

    for &byte in data.iter().take(50).rev() {
        if byte % 3 == 0 {
            cb.record_success();
        } else {
            cb.record_failure();
        }
    }

    let _ = handle.join();

    // State must still be valid after concurrent access
    let final_state = cb.state();
    assert!(
        matches!(final_state, ferro_circuit_breaker::CircuitState::Closed
            | ferro_circuit_breaker::CircuitState::Open
            | ferro_circuit_breaker::CircuitState::HalfOpen),
        "invalid final state: {:?}",
        final_state
    );
});
