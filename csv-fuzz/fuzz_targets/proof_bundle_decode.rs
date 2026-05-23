#![allow(dead_code)]
//! Fuzz target for proof bundle decoding
//!
//! This fuzz target tests the robustness of proof bundle decoding
//! against malformed or unexpected input.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize proof bundle from fuzzed data
    // This tests that the deserializer handles malformed input gracefully
    let _: Result<csv_core::proof::ProofBundle, _> = serde_cbor::from_slice(data);
});
