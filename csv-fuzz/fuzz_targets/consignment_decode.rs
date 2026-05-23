#![allow(dead_code)]
//! Fuzz target for consignment decoding
//!
//! This fuzz target tests the robustness of consignment decoding
//! against malformed or unexpected input.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize consignment from fuzzed data
    // This tests that the deserializer handles malformed input gracefully
    let _: Result<csv_core::consignment::Consignment, _> = serde_cbor::from_slice(data);
});
