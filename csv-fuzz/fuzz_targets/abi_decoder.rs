#![allow(dead_code)]
//! Fuzz target for seal point decoding
//!
//! This fuzz target tests the robustness of seal point decoding
//! against malformed or unexpected input.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize seal point from fuzzed data
    // This tests that the deserializer handles malformed input gracefully
    let _ = csv_core::seal::SealPoint::from_bytes(data);
});
