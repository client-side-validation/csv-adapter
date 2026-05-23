#![allow(dead_code)]
//! Fuzz target for finality data parsing
//!
//! This fuzz target tests the robustness of finality data parsing
//! against malformed or unexpected input.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to parse finality data from fuzzed input
    // This tests that the parser handles malformed data gracefully
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<csv_core::finality::FinalityState, _> = serde_json::from_str(s);
    }
});
